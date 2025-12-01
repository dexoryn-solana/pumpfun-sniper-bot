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
use shred_pool_cache::grpc::{BalanceMap, SubscriptionModRequest};
use shred_pool_cache::pumpfun::PumpFunSDK;
use shred_pool_cache::pumpswap::PumpSwapSDK;
use shred_pool_cache::pumpswap::constants::PriorityFeePump;
use shred_pool_cache::types::*;

use solana_client::rpc_client::SerializableTransaction;

use solana_hash::Hash;

use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::Duration;

const MAX_CACHE_CAPACITY: u64 = 16384;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    if CryptoProvider::get_default().is_none() {
        default_provider().install_default().unwrap();
    }

    // let balance_cache: BalanceMap = Cache::new(MAX_CACHE_CAPACITY);
    // let solfi_market = pubkey!("CAPhoEse9xEH95XmdnJjYrZdNCA8xfUWdy3aWymHa1Vj");
    // let ocra_market = pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE");
    // let openbook_market = pubkey!("HPGeZAXNNPzex7QnHP2ZCgyny13BqkNkRg6Lc49PJpdm");

    // let pool_accounts: Vec<SubscriptionModRequest> = vec![
    //     SubscriptionModRequest {
    //         market: pubkey!("CAPhoEse9xEH95XmdnJjYrZdNCA8xfUWdy3aWymHa1Vj"),
    //         base_pool: pubkey!("CTaDZW2LhvHPRnA9JWcZF8R5y2mpkV2RcHAXyEoKLbzp"),
    //         quote_pool: pubkey!("JHVJLsPsbzNW8JP8cPYmrwfzD2M9aHXdFHSjeeCDERu"),
    //     },
    //     SubscriptionModRequest {
    //         market: pubkey!("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE"),
    //         base_pool: pubkey!("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9"),
    //         quote_pool: pubkey!("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP"),
    //     },
    //     SubscriptionModRequest {
    //         market: pubkey!("HPGeZAXNNPzex7QnHP2ZCgyny13BqkNkRg6Lc49PJpdm"),
    //         base_pool: pubkey!("GVYEoR1ZbtJUaPQzDEujLX1TznRxNCpiE3JoVZKqmzVE"),
    //         quote_pool: pubkey!("47Ne3U31bZUqeju37zPouFfE9yYNMfzqnhNmSfs22Yw8"),
    //     },
    // ];

    // let (sender, receiver) = mpsc::channel(100_000);
    // let balance_cache_clone = balance_cache.clone();
    // tokio::spawn(async move {
    //     for (key, value) in &balance_cache_clone {
    //         if value.base.1 != 0 && value.quote.1 != 0 {
    //             info!(
    //                 "Price with {} Pool -> {}",
    //                 key,
    //                 value.quote.1 as f64 / value.b.1 as f64
    //             );
    //         }
    //     }
    // });

    // Shredstream thread
    // let balance_cache_clone = balance_cache.clone();

    #[derive(Clone)]
    struct FutureSellTXPF {
        mint: Pubkey,
        token_amount: u64,
        slot_to_exec: u64,
        blockhash: Hash,
    }

    #[derive(Clone)]
    struct FutureSellTXPS {
        mint: Pubkey,
        token_amount: u64,
        pool: Pubkey,
        slot_to_exec: u64,
        blockhash: Hash,
    }

    let sell_tx_cache: Cache<Pubkey, FutureSellTXPF> = Cache::new(1024);
    let sell_tx_cache_pumpswap: Cache<Pubkey, FutureSellTXPS> = Cache::new(1024);

    tokio::spawn(async move {
        let payer: Keypair = Keypair::from_base58_string(
            "2fkoh2J33Xi1dd5ua2d9sMnBA5LnpWCZSJntmnKHogP6nxNb8XT5QJkHk2ipn4VmJWbcajdjPFWWMxqyLCMNpdDV",
        );

        let priority_fee: PriorityFeePump = PriorityFeePump {
            unit_limit: 100_000,
            unit_price: 25_000_000,
            buy_tip_fee: 0.001,
            sell_tip_fee: 0.001,
            jito_tip_fee: 0.001,
        };

        let pumpswap_sdk = PumpSwapSDK::new(
            "https://staked.helius-rpc.com/?api-key=3cd475cb-5f5b-4162-ab31-6a7f07072104",
            //"https://frankfurt.mainnet.block-engine.jito.wtf",
            "http://fr.gateway.astralane.io/iris?api-key=steve9zVQJpgFn6TrPWrE28d7DP1sSDh7chekrQTbSZz9WfWCPY2PYl66kzQ8GKF",
            priority_fee,
        )
        .await;

        let pumpfun_sdk = PumpFunSDK::new(
            "https://staked.helius-rpc.com/?api-key=3cd475cb-5f5b-4162-ab31-6a7f07072104",
            //"https://frankfurt.mainnet.block-engine.jito.wtf",
            "http://fr.gateway.astralane.io/iris?api-key=steve9zVQJpgFn6TrPWrE28d7DP1sSDh7chekrQTbSZz9WfWCPY2PYl66kzQ8GKF",
            priority_fee,
        )
        .await;

        let url = env::var("SHREDSTREAM_URL").unwrap_or_else(|_| "localhost:9999".to_string());
        let pump_amm_program_id: Pubkey = Pubkey::from_str(PUMP_AMM_PROGRAM_ID_STR).unwrap();
        let pump_fun_program_id: Pubkey = Pubkey::from_str(PUMP_FUN_PROGRAM_ID_STR).unwrap();

        let wsol_address: Pubkey =
            Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();

        println!("Connecting to ShredstreamProxy at: {}", url);

        let mut client = ShredstreamProxyClient::connect(url).await.unwrap();
        let mut stream = client
            .subscribe_entries(SubscribeEntriesRequest {})
            .await
            .unwrap()
            .into_inner();

        let mut has_bought_before = false;
        let mut orders_placed = 0;

        while let Some(slot_entry) = stream.message().await.unwrap() {
            if orders_placed == 7 {
                has_bought_before = true;
            }

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

            for (id, future_tx) in sell_tx_cache_pumpswap.iter() {
                if curr_slot == future_tx.slot_to_exec {
                    let sell_tx = pumpswap_sdk
                        .clone()
                        .sell(
                            future_tx.mint,
                            future_tx.token_amount,
                            payer.insecure_clone(),
                            future_tx.blockhash,
                            future_tx.pool,
                        )
                        .await;

                    match sell_tx {
                        Ok(signature) => {
                            info!("Placed PumpSwap TX with Signature: {}", signature)
                        }
                        Err(e) => error!("Failed to place PumpFun TX {}", e),
                    }

                    sell_tx_cache_pumpswap.remove(&id);
                }
            }

            for entry in &entries {
                for tx in &entry.transactions {
                    // // check if this is a create tx
                    // let mut is_create_tx = false;

                    let signature = tx.get_signature();
                    let account_keys = tx.message.static_account_keys();

                    if account_keys
                        .contains(&pubkey!("J8zNuVj75srtpPCu1bfDAxoyXV1XAoWRc1aD2Y6cAgs4"))
                    {
                        info!("GAKE found!");

                        for ix in tx.message.instructions() {
                            if ix.data.len() < 8 {
                                continue;
                            }

                            let program_id = account_keys[ix.program_id_index as usize];

                            info!("I caught a program!!!: {}", program_id);

                            if program_id == pump_fun_program_id {
                                let data_prefix: [u8; 8] = ix.data[0..8].try_into().unwrap();

                                let (order_side, parsed_txn_wrapped) =
                                    if data_prefix == PUMP_ORDER_BUY {
                                        info!("FUCK NUBS");
                                        ("Buy", PumpFunTXInfo::from_buy_tx(tx, ix))
                                    } else if data_prefix == PUMP_ORDER_SELL {
                                        ("Sell", PumpFunTXInfo::from_sell_tx(tx, ix))
                                    } else {
                                        ("Unknown", Err(TXParserError::NotEnoughWallets))
                                    };

                                if order_side != "Buy" || parsed_txn_wrapped.is_err() {
                                    continue;
                                }

                                let parsed_txn = parsed_txn_wrapped.unwrap();

                                info!("NEW TX User: {:?} Signature: {}", tx, signature);

                                let amt = parsed_txn.amounts;

                                let buy_amount = match amt {
                                    Amounts::Unknown => {
                                        Err(AmountParsingError::UnknownAmountPassed)
                                    }
                                    Amounts::BuyAmounts(buy_amounts) => Ok(buy_amounts),
                                    Amounts::SellAmounts(_) => {
                                        Err(AmountParsingError::PassedSellInBuyFunction)
                                    }
                                }
                                .unwrap();

                                if !has_bought_before {
                                    let swap_res = pumpfun_sdk
                                        .clone()
                                        .buy(
                                            &parsed_txn.mint,
                                            0.0001,
                                            payer.insecure_clone(),
                                            parsed_txn.blockhash,
                                            buy_amount,
                                        )
                                        .await;

                                    match swap_res {
                                        Ok(signature) => {
                                            info!(
                                                "Placed PumpFun TX with Signature: {}",
                                                signature.0
                                            );
                                            sell_tx_cache.insert(
                                                Pubkey::new_unique(),
                                                FutureSellTXPF {
                                                    mint: parsed_txn.mint,
                                                    token_amount: signature.1,
                                                    slot_to_exec: curr_slot + 15,
                                                    blockhash: parsed_txn.blockhash,
                                                },
                                            );

                                            orders_placed += 1;
                                        }
                                        Err(e) => error!("Failed to place PumpFun TX {}", e),
                                    }

                                    let curr_time = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .expect("Time went backwards")
                                        .as_millis();

                                    info!("Executed TX at: {}", curr_time);
                                }
                                break;
                            } else if program_id == pump_amm_program_id {
                                let data_prefix: [u8; 8] = ix.data[0..8].try_into().unwrap();

                                let (order_side, parsed_txn_wrapped) =
                                    if data_prefix == PUMP_ORDER_BUY {
                                        let order_side;
                                        let txn;
                                        let test_tx =
                                            PumpSwapTXInfo::from_buy_tx(tx, ix).unwrap_or_default();

                                        if test_tx.base_mint == wsol_address {
                                            order_side = "Sell";
                                            txn = PumpSwapTXInfo::from_sell_tx(tx, ix);
                                        } else {
                                            order_side = "Buy";
                                            txn = PumpSwapTXInfo::from_buy_tx(tx, ix);
                                        }

                                        (order_side, txn)
                                    } else if data_prefix == PUMP_ORDER_SELL {
                                        ("Sell", PumpSwapTXInfo::from_sell_tx(tx, ix))
                                    } else {
                                        ("Unknown", Err(TXParserError::NotEnoughWallets))
                                    };

                                if order_side != "Buy" || parsed_txn_wrapped.is_err() {
                                    continue;
                                }

                                let parsed_txn = parsed_txn_wrapped.unwrap();
                                // Change gRPC sub to subscribe to these and the old ones
                                // let mod_req = SubscriptionModRequest {
                                //     market: parsed_txn.pool,
                                //     base_pool: parsed_txn.pool_base_token_account,
                                //     quote_pool: parsed_txn.pool_quote_token_account,
                                // };

                                // assert!(sender.send(mod_req.clone()).await.is_ok());

                                info!("NEW TX User: {:?} Signature: {}", tx, signature);
                                // while pool_bals.base.1 != 0 || pool_bals.quote.1 != 0 {
                                //     pool_bals = balance_cache_clone.get(&parsed_txn.pool).unwrap();
                                // }

                                let buy_amount = match get_buy_amounts(parsed_txn.amounts) {
                                    Ok(x) => x,
                                    Err(err) => {
                                        error!("Could not parse the buy amount: {:?}", err);
                                        continue;
                                    }
                                };

                                let swap_res = pumpswap_sdk
                                    .clone()
                                    .buy(
                                        parsed_txn.base_mint,
                                        0.0001,
                                        payer.insecure_clone(),
                                        parsed_txn.blockhash,
                                        parsed_txn.pool,
                                        buy_amount,
                                    )
                                    .await;

                                // let sandwich_res = pumpswap_sdk
                                //     .sandwich_attack(0.001, &payer, &parsed_txn, tx)
                                //     .await;

                                // match sandwich_res {
                                //     Ok(signature) => {
                                //         info!("Placed PumpSwap TX with Signature: {}", signature);
                                //     }
                                //     Err(e) => error!("Failed to place PumpSwap TX {}", e),
                                // }

                                match swap_res {
                                    Ok(signature) => {
                                        info!("Placed PumpSwap TX with Signature: {}", signature.0);
                                        sell_tx_cache_pumpswap.insert(
                                            Pubkey::new_unique(),
                                            FutureSellTXPS {
                                                mint: parsed_txn.base_mint,
                                                token_amount: signature.1,
                                                pool: parsed_txn.pool,
                                                slot_to_exec: curr_slot + 10,
                                                blockhash: parsed_txn.blockhash,
                                            },
                                        );

                                        orders_placed += 1;
                                    }
                                    Err(e) => error!("Failed to place PumpSwap TX {}", e),
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
