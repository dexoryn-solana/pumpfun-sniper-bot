use std::str::FromStr;

use anyhow::Result;
use env_logger::Env;
use jito_protos::shredstream::{
    SubscribeEntriesRequest, shredstream_proxy_client::ShredstreamProxyClient,
};
use log::{error, info};
use moka::sync::Cache;
use rustls::crypto::CryptoProvider;
use rustls::crypto::aws_lc_rs::default_provider;
use shred_pool_cache::pumpfun::PumpFunSDK;
use shred_pool_cache::pumpswap::constants::PriorityFeePump;
use shred_pool_cache::types::*;

use solana_client::rpc_client::SerializableTransaction;

use solana_hash::Hash;

use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::Duration;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if CryptoProvider::get_default().is_none() {
        default_provider().install_default().unwrap();
    }

    #[derive(Clone)]
    struct FutureSellTXPF {
        mint: Pubkey,
        token_amount: u64,
        slot_to_exec: u64,
        blockhash: Hash,
        creator: Pubkey,
    }

    let sell_tx_cache: Cache<Pubkey, FutureSellTXPF> = Cache::new(1024);

    let bribe_fee = f64::from_str(
        env::var("BRIBE_FEE")
            .unwrap_or_else(|_| "0.0001".to_string())
            .as_str(),
    )
    .unwrap();
    let buy_amount_sol = f64::from_str(
        env::var("SOL_BUY_AMOUNT")
            .unwrap_or_else(|_| "0.0001".to_string())
            .as_str(),
    )
    .unwrap();

    let payer: Keypair = Keypair::from_base58_string(
        env::var("PVT_KEY")
            .unwrap_or_else(|_| "".to_string())
            .as_str(),
    );

    tokio::spawn(async move {
        let priority_fee: PriorityFeePump = PriorityFeePump {
            unit_limit: 100_000,
            unit_price: 25_000_000,
            buy_tip_fee: 0.001,
            sell_tip_fee: 0.001,
            jito_tip_fee: bribe_fee,
        };

        let pumpfun_sdk = PumpFunSDK::new(
            "https://staked.helius-rpc.com/?api-key=3cd475cb-5f5b-4162-ab31-6a7f07072104",
            // "https://frankfurt.mainnet.block-engine.jito.wtf",
            "http://fr.gateway.astralane.io/iris?api-key=steve9zVQJpgFn6TrPWrE28d7DP1sSDh7chekrQTbSZz9WfWCPY2PYl66kzQ8GKF",
            priority_fee,
        )
        .await;

        let url = env::var("SHREDSTREAM_URL").unwrap_or_else(|_| "localhost:9999".to_string());
        let pump_fun_program_id: Pubkey = Pubkey::from_str(PUMP_FUN_PROGRAM_ID_STR).unwrap();

        println!("Connecting to ShredstreamProxy at: {}", url);

        let mut client = ShredstreamProxyClient::connect(url).await.unwrap();
        let mut stream = client
            .subscribe_entries(SubscribeEntriesRequest {})
            .await
            .unwrap()
            .into_inner();

        while let Some(slot_entry) = stream.message().await.unwrap() {
            let entries = match bincode::deserialize::<Vec<solana_entry::entry::Entry>>(
                &slot_entry.entries,
            ) {
                Ok(e) => e,
                Err(e) => {
                    println!("Deserialization failed with err: {e}");
                    continue;
                }
            };

            let curr_slot = slot_entry.slot;
            for (id, future_tx) in sell_tx_cache.iter() {
                if curr_slot == future_tx.slot_to_exec {
                    let sell_tx = pumpfun_sdk
                        .clone()
                        .sell(
                            &future_tx.mint,
                            future_tx.token_amount,
                            payer.insecure_clone(),
                            future_tx.blockhash,
                            future_tx.creator,
                        )
                        .await;

                    match sell_tx {
                        Ok(signature) => {
                            info!("Placed PumpFun TX with Signature: {}", signature)
                        }
                        Err(e) => error!("Failed to place PumpFun TX {}", e),
                    }

                    sell_tx_cache.remove(&id);
                }
            }

            for entry in &entries {
                for tx in &entry.transactions {
                    // // check if this is a create tx
                    // let mut is_create_tx = false;

                    let signature = tx.get_signature();
                    let account_keys = tx.message.static_account_keys();

                    let mut is_create_tx = false;

                    for ix in tx.message.instructions() {
                        if ix.data.len() < 8 {
                            continue;
                        }

                        let program_id = account_keys[ix.program_id_index as usize];
                        if program_id == pump_fun_program_id {
                            let data_prefix: [u8; 8] = ix.data[0..8].try_into().unwrap();
                            if data_prefix == [24, 30, 200, 40, 5, 28, 7, 119] {
                                info!("Found PumpFun Create TX: {}", signature);

                                is_create_tx = true;
                            }

                            if !is_create_tx {
                                continue;
                            }

                            let parsed_txn_wrapped = PumpFunTXInfo::from_create_tx(tx, ix);
                            let parsed_txn = parsed_txn_wrapped.unwrap();

                            info!("NEW TX User: {:?} Signature: {}", tx, signature);

                            let amt = parsed_txn.amounts;

                            let buy_amount = match amt {
                                Amounts::Unknown => continue,
                                Amounts::BuyAmounts(buy_amounts) => buy_amounts,
                                Amounts::SellAmounts(_) => {
                                    continue;
                                }
                            };

                            println!("buy_amount: {:?} tx: {:?}", buy_amount, tx);
                            let swap_res = pumpfun_sdk
                                .clone()
                                .buy(
                                    &parsed_txn.mint,
                                    buy_amount_sol,
                                    payer.insecure_clone(),
                                    parsed_txn.blockhash,
                                    buy_amount,
                                    parsed_txn.creator,
                                )
                                .await;

                            match swap_res {
                                Ok(signature) => {
                                    info!("Placed PumpFun TX with Signature: {}", signature.0);
                                    sell_tx_cache.insert(
                                        Pubkey::new_unique(),
                                        FutureSellTXPF {
                                            mint: parsed_txn.mint,
                                            token_amount: signature.1,
                                            slot_to_exec: curr_slot + 7,
                                            blockhash: parsed_txn.blockhash,
                                            creator: parsed_txn.creator,
                                        },
                                    );
                                }
                                Err(e) => error!("Failed to place PumpFun TX {}", e),
                            }

                            let curr_time = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("Time went backwards")
                                .as_millis();

                            info!("Executed TX at: {}", curr_time);

                            break;
                        }
                    }
                }
            }
        }
    });

    // gRPC thread
    // let balance_cache_clone = balance_cache.clone();
    // tokio::spawn({
    //     async move {
    //         let manager = GrpcStreamManager::new(
    //             "https://basic.grpc.solanavibestation.com/",
    //             "dd639cf91c1571d0a55e1876825f4bf7",
    //             balance_cache_clone,
    //             "https://api.mainnet-beta.solana.com".to_string(),
    //         )
    //         .await
    //         .unwrap();
    //         let mut manager_lock = manager.lock().await;

    //         info!("Starting subscription for account balance updates...");

    //         let result = manager_lock.connect(pool_accounts, receiver).await;
    //         if let Err(e) = &result {
    //             error!("Subscription error: {:?}", e);
    //         }

    //         result.unwrap();
    //     }
    // });

    // Keep the main thread running
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
