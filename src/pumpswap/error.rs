use thiserror::Error;

/// Custom error types for the PumpSwap SDK
#[derive(Error, Debug)]
pub enum PumpSwapError {
    #[error("Solana client error: {0}")]
    SolanaClientError(#[from] solana_client::client_error::ClientError),

    #[error("SPL token error: {0}")]
    SplTokenError(#[from] spl_token_2022::error::TokenError),

    #[error("Pubkey parsing error: {0}")]
    PubkeyParseError(String),

    #[error("Pool not found for mint {0}")]
    PoolNotFound(String),

    #[error("Insufficient token balance")]
    InsufficientBalance,

    #[error("Token account not found")]
    TokenAccountNotFound,

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Bundle submission error: {0}")]
    BundleSubmissionError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("RPC connection error: {0}")]
    RpcConnectionError(String),

    #[error("Miscellaneous error: {0}")]
    MiscError(String),

    #[error("Jito client error: {0}")]
    JitoClientError(String),
}

/// Result type for PumpSwap operations
pub type PumpSwapResult<T> = Result<T, PumpSwapError>;
