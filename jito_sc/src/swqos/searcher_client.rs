use jito_protos::{
    auth::auth_service_client::AuthServiceClient,
    bundle::Bundle,
    searcher::{
        searcher_service_client::SearcherServiceClient, SendBundleRequest,
        SubscribeBundleResultsRequest,
    },
};
use solana_quic_client::{QuicConfig, QuicConnectionManager, QuicPool};
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use bincode::serialize;

use jito_protos::bundle::BundleResult;

use solana_client::nonblocking::{rpc_client::RpcClient, tpu_client::TpuClient};
use solana_sdk::{
    signature::{Keypair, Signature},
    transaction::VersionedTransaction,
};
use thiserror::Error;
use tokio::sync::Mutex;
use tonic::{
    service::interceptor::InterceptedService,
    transport::{self, Channel, ClientTlsConfig, Endpoint},
    Status,
};
// use crate::common::SolanaRpcClient;
use super::{
    client_interceptor,
    cluster_data_impl::{ClusterData, ClusterDataImpl},
};
use crate::swqos::client_interceptor::*;
use crate::swqos::common::poll_transaction_confirmation;
use bytes::Bytes;
use tonic::codegen::{Body, StdError};

pub type SearcherClientResult<T> = Result<T, SearcherClientError>;

#[derive(Debug, Error)]
pub enum BlockEngineConnectionError {
    #[error("transport error {0}")]
    TransportError(#[from] transport::Error),
    #[error("client error {0}")]
    ClientError(#[from] Status),
}

#[derive(Debug, Error)]
pub enum BundleRejectionError {
    #[error("bundle lost state auction, auction: {0}, tip {1} lamports")]
    StateAuctionBidRejected(String, u64),
    #[error("bundle won state auction but failed global auction, auction {0}, tip {1} lamports")]
    WinningBatchBidRejected(String, u64),
    #[error("bundle simulation failure on tx {0}, message: {1:?}")]
    SimulationFailure(String, Option<String>),
    #[error("internal error {0}")]
    InternalError(String),
}

pub type BlockEngineConnectionResult<T> = Result<T, BlockEngineConnectionError>;

pub async fn get_searcher_client_no_auth(
    block_engine_url: &str,
) -> BlockEngineConnectionResult<SearcherServiceClient<Channel>> {
    let searcher_channel = create_grpc_channel(block_engine_url).await?;
    let searcher_client = SearcherServiceClient::new(searcher_channel);
    Ok(searcher_client)
}

#[derive(Clone)]
pub struct SearcherClient<C: ClusterData, T> {
    cluster_data: Arc<C>,
    searcher_service_client: Arc<Mutex<SearcherServiceClient<T>>>,
    exit: Arc<AtomicBool>,
}

use jito_protos::packet::{
    Meta as ProtoMeta, Packet as ProtoPacket, PacketBatch as ProtoPacketBatch,
    PacketFlags as ProtoPacketFlags,
};

use std::{
    cmp::min,
    net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use solana_perf::packet::{Packet, PacketBatch, PACKET_DATA_SIZE};
use solana_sdk::packet::{Meta, PacketFlags};

pub fn packet_from_versioned_tx(tx: VersionedTransaction) -> Packet {
    let tx_data = serialize(&tx).expect("serializes");
    let mut data = [0; PACKET_DATA_SIZE];
    let copy_len = min(tx_data.len(), data.len());
    data[..copy_len].copy_from_slice(&tx_data[..copy_len]);
    let mut packet = Packet::new(data, Default::default());
    packet.meta_mut().size = copy_len;
    packet
}

/// Converts a VersionedTransaction to a protobuf packet
pub fn proto_packet_from_versioned_tx(tx: &VersionedTransaction) -> ProtoPacket {
    let data = serialize(tx).expect("serializes");
    let size = data.len() as u64;
    ProtoPacket {
        data,
        meta: Some(ProtoMeta {
            size,
            addr: "".to_string(),
            port: 0,
            flags: None,
            sender_stake: 0,
        }),
    }
}

impl<C: ClusterData + Clone, T> SearcherClient<C, T>
where
    T: tonic::client::GrpcService<tonic::body::BoxBody>,
    T::Error: Into<StdError>,
    T::ResponseBody: Body<Data = Bytes> + Send + 'static,
    <T::ResponseBody as Body>::Error: Into<StdError> + Send,
{
    pub fn new(
        cluster_data: C,
        searcher_service_client: SearcherServiceClient<T>,
        exit: Arc<AtomicBool>,
    ) -> Self {
        Self {
            searcher_service_client: Arc::new(Mutex::new(searcher_service_client)),
            cluster_data: Arc::new(cluster_data),
            exit,
        }
    }

    /// Sends the list of transactions as a bundle iff the leader is a jito-solana.
    /// Returns the bundle's id.
    pub async fn send_bundle(
        &self,
        transactions: Vec<VersionedTransaction>,
        // Defines how many slots to lookahead for a jito-solana validator in order to
        // determine whether or not the bundle can be sent.
        slot_lookahead: u64,
    ) -> SearcherClientResult<String> {
        // let next_leader_slot = self
        //     .cluster_data
        //     .next_jito_validator()
        //     .await
        //     .ok_or(SearcherClientError::NoUpcomingJitoValidator)?
        //     .1;

        // println!("NEXT LEADER SLOT: {}", next_leader_slot);

        // if next_leader_slot > slot_lookahead + self.cluster_data.current_slot().await {
        //     return Err(SearcherClientError::NoUpcomingJitoValidator);
        // }

        let resp = self
            .searcher_service_client
            .lock()
            .await
            .send_bundle(SendBundleRequest {
                bundle: Some(Bundle {
                    header: None,
                    packets: transactions
                        .iter()
                        .map(proto_packet_from_versioned_tx)
                        .collect(),
                }),
            })
            .await?;

        Ok(resp.into_inner().uuid)
    }
}

pub async fn get_searcher_client_auth(
    auth_keypair: &Arc<Keypair>,
    exit: &Arc<AtomicBool>,
    block_engine_url: &str,
    rpc_pubsub_addr: &str,
) -> SearcherClientResult<(
    SearcherClient<ClusterDataImpl, InterceptedService<Channel, ClientInterceptor>>,
    ClusterDataImpl,
)> {
    let auth_channel = create_grpc_channel(block_engine_url).await.unwrap();
    let client_interceptor =
        ClientInterceptor::new(AuthServiceClient::new(auth_channel), auth_keypair).await?;

    let searcher_channel = create_grpc_channel(block_engine_url).await.unwrap();
    let searcher_service_client =
        SearcherServiceClient::with_interceptor(searcher_channel, client_interceptor);

    let cluster_data_impl = ClusterDataImpl::new(
        rpc_pubsub_addr.to_string(),
        searcher_service_client.clone(),
        exit.clone(),
    )
    .await;

    Ok((
        SearcherClient::new(
            cluster_data_impl.clone(),
            searcher_service_client,
            exit.clone(),
        ),
        cluster_data_impl,
    ))
}

pub async fn create_grpc_channel(url: &str) -> BlockEngineConnectionResult<Channel> {
    let mut endpoint = Endpoint::from_shared(url.to_string()).expect("invalid url");
    if url.starts_with("https") {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_native_roots())?;
    }

    endpoint = endpoint.tcp_nodelay(true);
    endpoint = endpoint.tcp_keepalive(Some(Duration::from_secs(100)));
    endpoint = endpoint.connect_timeout(Duration::from_secs(200));
    endpoint = endpoint.http2_keep_alive_interval(Duration::from_secs(100));

    Ok(endpoint.connect().await?)
}

pub async fn subscribe_bundle_results(
    searcher_client: Arc<Mutex<SearcherServiceClient<Channel>>>,
    request: impl tonic::IntoRequest<SubscribeBundleResultsRequest>,
) -> std::result::Result<tonic::Response<tonic::codec::Streaming<BundleResult>>, tonic::Status> {
    let mut searcher = searcher_client.lock().await;
    searcher.subscribe_bundle_results(request).await
}

pub async fn send_transaction(
    tpu_client: &TpuClient<QuicPool, QuicConnectionManager, QuicConfig>,
    transactions: VersionedTransaction,
) -> Result<(), SearcherClientError> {
    let serialized_tx = serialize(&transactions)
        .map_err(|_e| SearcherClientError::TransactionSerializationError)?;

    if !tpu_client.send_wire_transaction(serialized_tx).await {
        Err(SearcherClientError::TpuClientError)
    } else {
        Ok(())
    }
}

pub async fn send_bundle_no_wait(
    transactions: &Vec<VersionedTransaction>,
    searcher_client: Arc<
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
) -> Result<String, anyhow::Error> {
    let searcher = searcher_client.lock().await;
    let sig = searcher.send_bundle(transactions.to_owned(), 1).await?;

    Ok(sig)
}
