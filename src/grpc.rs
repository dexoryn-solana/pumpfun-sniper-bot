use ahash::{HashSet, HashSetExt};
use moka::sync::Cache;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, native_token::sol_to_lamports, pubkey::Pubkey,
};
use spl_token_2022::{extension::StateWithExtensions, state::Account};
use std::{collections::HashMap, time::Instant};
use tokio::sync::mpsc::Receiver;
use yellowstone_grpc_proto::geyser::{CommitmentLevel, SubscribeRequestFilterAccounts};
use {
    anyhow::Result,
    futures::{sink::SinkExt, stream::StreamExt},
    log::{error, info},
    rustls::crypto::{CryptoProvider, ring::default_provider},
    std::{sync::Arc, time::Duration},
    tokio::sync::{Mutex, mpsc},
    tonic::{
        metadata::errors::InvalidMetadataValue,
        transport::{Endpoint, channel::ClientTlsConfig},
    },
    tonic_health::pb::health_client::HealthClient,
    yellowstone_grpc_client::{GeyserGrpcClient, InterceptorXToken},
    yellowstone_grpc_proto::{
        geyser::{
            SubscribeRequest, SubscribeUpdateAccount, geyser_client::GeyserClient,
            subscribe_update::UpdateOneof,
        },
        prelude::SubscribeRequestPing,
    },
};

#[derive(Debug, Clone)]
pub struct SubscriptionModRequest {
    pub market: Pubkey,
    pub base_pool: Pubkey,
    pub quote_pool: Pubkey,
}

#[derive(Default, Debug, Clone)]
pub struct Accounts {
    pub base: (Pubkey, u64),
    pub quote: (Pubkey, u64),
}
pub type BalanceMap = Cache<Pubkey, Accounts>;

pub struct GrpcStreamManager {
    client: GeyserGrpcClient<InterceptorXToken>,
    is_connected: bool,
    reconnect_attempts: u32,
    max_reconnect_attempts: u32,
    reconnect_interval: Duration,
    balance_map: BalanceMap,
    account_list: HashSet<String>,
    rpc_client: RpcClient,
}

pub fn lamports_to_sol_10e6(token: f64) -> u64 {
    (token * 1e6 as f64) as u64
}

impl GrpcStreamManager {
    pub async fn handle_account_update(&mut self, account_update: &SubscribeUpdateAccount) {
        let account = match &account_update.account {
            Some(account) => account,
            None => {
                error!("Could not get account data from the account update!");
                return;
            }
        };

        let ata_data = &account.data;
        let ata_state = match StateWithExtensions::<Account>::unpack(ata_data) {
            Ok(ata_state) => ata_state,
            Err(e) => {
                error!("Could not parse the ATA State: {}", e);
                return;
            }
        };
        let ata_balance = ata_state.base.amount;

        let market_pubkey = ata_state.base.owner;
        let pool_pubkey = Pubkey::new_from_array(account.clone().pubkey.try_into().unwrap());

        self.balance_map
            .entry_by_ref(&market_pubkey)
            .and_upsert_with(|map_entry| {
                if let Some(value) = map_entry {
                    let mut curr_bal = value.into_value();
                    if curr_bal.base.0 == pool_pubkey {
                        curr_bal.base.1 = ata_balance;
                    } else if curr_bal.quote.0 == pool_pubkey {
                        curr_bal.quote.1 = ata_balance;
                    }

                    curr_bal
                } else {
                    Accounts {
                        ..Default::default()
                    }
                }
            });

        for (key, value) in &self.balance_map {
            if value.base.1 != 0 && value.quote.1 != 0 {
                info!(
                    "SOL/USDC Price with {} Pool -> {}",
                    key,
                    value.base.1 as f64 / value.quote.1 as f64
                );

                info!(
                    "USDC/SOL Price with {} Pool -> {}",
                    key,
                    value.quote.1 as f64 / value.base.1 as f64
                );

                info!("");
            }
        }
    }

    pub async fn new(
        endpoint: &str,
        x_token: &str,
        cache: BalanceMap,
        rpc_endpoint: String,
    ) -> Result<Arc<Mutex<GrpcStreamManager>>> {
        let interceptor = InterceptorXToken {
            x_token: Some(
                x_token
                    .parse()
                    .map_err(|e: InvalidMetadataValue| anyhow::Error::from(e))?,
            ),
            x_request_snapshot: true,
        };

        if CryptoProvider::get_default().is_none() {
            default_provider().install_default().unwrap();
        }

        let channel = Endpoint::from_shared(endpoint.to_string())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(1000))
            .tcp_keepalive(Some(Duration::from_secs(30)))
            .tcp_nodelay(true)
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect()
            .await
            .map_err(anyhow::Error::from)?;

        let client = GeyserGrpcClient::new(
            HealthClient::with_interceptor(channel.clone(), interceptor.clone()),
            GeyserClient::with_interceptor(channel, interceptor),
        );

        Ok(Arc::new(Mutex::new(GrpcStreamManager {
            client,
            is_connected: false,
            reconnect_attempts: 0,
            max_reconnect_attempts: 5,
            reconnect_interval: Duration::from_secs(2),
            balance_map: cache,
            account_list: HashSet::new(),
            rpc_client: RpcClient::new(rpc_endpoint),
        })))
    }

    pub async fn connect(
        &mut self,
        pool_accounts: Vec<SubscriptionModRequest>,
        mut pool_recv: Receiver<SubscriptionModRequest>,
    ) -> Result<()> {
        info!("Connecting to gRPC stream...");
        info!("Subscription request: {:?}", pool_accounts);

        for pool in pool_accounts.iter().clone() {
            self.balance_map.insert(
                pool.market,
                Accounts {
                    base: (pool.base_pool, 0),
                    quote: (pool.quote_pool, 0),
                },
            );
            self.account_list.insert(pool.base_pool.to_string());
            self.account_list.insert(pool.quote_pool.to_string());
        }

        info!(
            "Account list: {:?}",
            Vec::from_iter(self.account_list.clone())
        );

        let request = SubscribeRequest {
            accounts: HashMap::from_iter(vec![(
                "accounts".to_string(),
                SubscribeRequestFilterAccounts {
                    account: Vec::from_iter(self.account_list.clone()),
                    owner: vec![],
                    filters: vec![],
                    nonempty_txn_signature: None,
                },
            )]),

            commitment: Some(CommitmentLevel::Processed as i32),
            ..Default::default()
        };

        let (mut subscribe_tx, mut stream) = self
            .client
            .subscribe_with_request(Some(request.clone()))
            .await?;

        info!("Successfully connected to gRPC stream");
        self.is_connected = true;
        self.reconnect_attempts = 0;

        let (ping_sender, mut ping_receiver) = mpsc::channel(10);

        let ping_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = ping_sender.send(()).await {
                    error!("Failed to send ping signal: {:?}", e);
                    break;
                }
            }
        });

        info!("Starting message processing loop...");
        loop {
            let recv_msg = pool_recv.try_recv();

            'a: {
                if recv_msg.is_ok() {
                    let msg_unwrapped = recv_msg.unwrap();

                    if self
                        .account_list
                        .contains(&msg_unwrapped.base_pool.to_string())
                    {
                        break 'a;
                    }

                    self.account_list
                        .insert(msg_unwrapped.base_pool.to_string());
                    self.account_list
                        .insert(msg_unwrapped.quote_pool.to_string());

                    let (sub_resp, initial_base_price, initial_quote_price) = tokio::join!(
                        subscribe_tx.send(SubscribeRequest {
                            accounts: HashMap::from_iter(vec![(
                                "accounts".to_string(),
                                SubscribeRequestFilterAccounts {
                                    account: Vec::from_iter(self.account_list.clone()),
                                    owner: vec![],
                                    filters: vec![],
                                    nonempty_txn_signature: None,
                                },
                            )]),

                            commitment: Some(CommitmentLevel::Processed as i32),
                            ..Default::default()
                        }),
                        self.rpc_client
                            .get_token_account_balance(&msg_unwrapped.base_pool),
                        self.rpc_client
                            .get_token_account_balance(&msg_unwrapped.quote_pool),
                    );

                    sub_resp?;

                    // TODO: fix the snapshot event from the gRPC server so that we don't have to make these extra requests.
                    if initial_base_price.is_ok() && initial_quote_price.is_ok() {
                        info!(
                            "Initial data for market {} requested through RPC. Pools {} and {}",
                            msg_unwrapped.market, msg_unwrapped.base_pool, msg_unwrapped.quote_pool
                        );

                        let initial_base_price = initial_base_price.unwrap();
                        let initial_quote_price = initial_quote_price.unwrap();

                        self.balance_map.insert(
                            msg_unwrapped.market,
                            Accounts {
                                base: (
                                    msg_unwrapped.base_pool,
                                    u64::from_str_radix(&initial_base_price.amount, 10)?,
                                ),
                                quote: (
                                    msg_unwrapped.quote_pool,
                                    u64::from_str_radix(&initial_quote_price.amount, 10)?,
                                ),
                            },
                        );
                    } else {
                        self.balance_map.insert(
                            msg_unwrapped.market,
                            Accounts {
                                base: (msg_unwrapped.base_pool, 0),
                                quote: (msg_unwrapped.quote_pool, 0),
                            },
                        );
                    }

                    info!(
                        "Updated pool subscription: {:?}",
                        Vec::from_iter(&self.account_list)
                    );
                }
            }

            tokio::select! {
                Some(message) = stream.next() => {
                    match message {
                        Ok(msg) => {
                            match msg.update_oneof {
                                Some(UpdateOneof::Account(account)) => {
                                    // let startup = account.is_startup;
                                    // if startup {

                                    //     info!("IS A startup update");
                                    // } else {
                                    //     info!("Not a startup update");
                                    // }

                                    self.handle_account_update(&account).await;
                                }
                                Some(UpdateOneof::Ping(_)) => {
                                    subscribe_tx
                                        .send(SubscribeRequest {
                                            ping: Some(SubscribeRequestPing { id: 1 }),
                                            ..Default::default()
                                        })
                                        .await?;
                                }
                                Some(UpdateOneof::Pong(_)) => {

                                } // Ignore pong responses
                                _ => {
                                    info!("Other update received: {:?}", msg);
                                }
                            }
                        }
                        Err(err) => {
                            error!("Error GRPC: {:?}", err);

                            self.is_connected = false;
                            ping_handle.abort(); // Cleanup ping task

                            Box::pin(self.reconnect(pool_accounts, pool_recv)).await?;

                            return Ok(());
                        }
                    }
                }

                Some(_) = ping_receiver.recv() => {
                    info!("Sending ping request");
                    if let Err(e) = subscribe_tx
                        .send(SubscribeRequest {
                            ping: Some(SubscribeRequestPing { id: 1 }),
                            ..Default::default()
                        })
                        .await
                    {
                        error!("Failed to send ping: {:?}", e);
                        break;
                    }
                }

                else => break,
            }
        }

        ping_handle.abort();

        Ok(())
    }

    async fn reconnect(
        &mut self,
        pool_accounts: Vec<SubscriptionModRequest>,
        pool_recv: Receiver<SubscriptionModRequest>,
    ) -> Result<()> {
        if self.reconnect_attempts >= self.max_reconnect_attempts {
            info!("Max reconnection attempts reached");
            return Ok(());
        }

        self.reconnect_attempts += 1;
        info!("Reconnecting... Attempt {}", self.reconnect_attempts);

        let backoff = self.reconnect_interval * std::cmp::min(self.reconnect_attempts, 5);
        tokio::time::sleep(backoff).await;

        Box::pin(self.connect(pool_accounts, pool_recv)).await
    }
}
