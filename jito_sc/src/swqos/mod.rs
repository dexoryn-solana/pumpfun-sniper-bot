use cluster_data_impl::ClusterDataImpl;
use jito_protos::searcher::searcher_service_client::SearcherServiceClient;
use searcher_client::{get_searcher_client_auth, SearcherClient};
use solana_sdk::signature::Keypair;

use std::sync::{atomic::AtomicBool, Arc};
use tokio::sync::{Mutex, RwLock};
use tonic::transport::Channel;

use solana_sdk::{pubkey::Pubkey, signature::Signature};

use anyhow::{anyhow, Result};
use rand::{rng, seq::IteratorRandom};
use solana_sdk::transaction::VersionedTransaction;

use crate::constants::{accounts::JITO_TIP_ACCOUNTS, SolanaRpcClient};

pub mod api;
pub mod client_interceptor;
pub mod cluster_data_impl;
pub mod common;
pub mod jito_grpc;
pub mod searcher_client;

lazy_static::lazy_static! {
    static ref TIP_ACCOUNT_CACHE: RwLock<Vec<String>> = RwLock::new(Vec::new());
}

#[derive(Debug, Clone, Copy)]
pub enum ClientType {
    Jito,
    NextBlock,
    ZeroSlot,
}

pub type FeeClient = dyn FeeClientTrait + Send + Sync + 'static;

#[async_trait::async_trait]
pub trait FeeClientTrait {
    async fn send_transaction(&self, transaction: &VersionedTransaction) -> Result<Signature>;

    async fn get_tip_account(&self) -> Result<String>;
    async fn get_client_type(&self) -> ClientType;
}

pub struct JitoClient {
    pub rpc_client: Arc<SolanaRpcClient>,
    pub searcher_client: Arc<
        Mutex<
            SearcherClient<
                ClusterDataImpl,
                tonic::service::interceptor::InterceptedService<
                    Channel,
                    client_interceptor::ClientInterceptor,
                >,
            >,
        >,
    >,
}

#[async_trait::async_trait]
impl FeeClientTrait for JitoClient {
    async fn send_transaction(
        &self,
        transaction: &VersionedTransaction,
    ) -> Result<Signature, anyhow::Error> {
        self.send_transaction(&transaction).await
    }

    async fn get_tip_account(&self) -> Result<String, anyhow::Error> {
        if let Some(acc) = JITO_TIP_ACCOUNTS.iter().choose(&mut rng()) {
            Ok(acc.to_string())
        } else {
            Err(anyhow!("no valid tip accounts found"))
        }
    }

    async fn get_client_type(&self) -> ClientType {
        ClientType::Jito
    }
}

impl JitoClient {
    pub async fn new(rpc_url: String, block_engine_url: String) -> Result<Self> {
        let rpc_client = SolanaRpcClient::new(rpc_url.clone());
        let keypair = Arc::new(Keypair::from_base58_string("4nsVjduzDUfDHQLVQvqzxjW6LgpvUeDLuAhCQQs4QUKdtU5xTGw2Fp9oFaYRvAB46AbzDxwfrbdvcDYxFXeQru89"));
        let exit: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
        let (searcher_client, w) =
            get_searcher_client_auth(&keypair, &exit, block_engine_url.as_str(), rpc_url.as_str())
                .await?;

        Ok(Self {
            rpc_client: Arc::new(rpc_client),
            searcher_client: Arc::new(Mutex::new(searcher_client)),
        })
    }

    pub async fn send_bundle_no_wait(
        &self,
        transactions: &Vec<VersionedTransaction>,
    ) -> Result<String, anyhow::Error> {
        searcher_client::send_bundle_no_wait(&transactions, self.searcher_client.clone()).await
    }
}
