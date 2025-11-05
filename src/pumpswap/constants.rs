use serde::Deserialize;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Define static public keys
pub const PUMP_AMM_PROGRAM_ID_STR: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const ASSOCIATED_TOKEN_PROGRAM_ID_STR: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const TOKEN_PROGRAM_ID_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const WSOL_TOKEN_ACCOUNT_STR: &str = "So11111111111111111111111111111111111111112";
pub const GLOBAL_STR: &str = "ADyA8hdefvWN2dbGGWFotbzWxrAvLW83WG6QCVXvJKqw";
pub const EVENT_AUTHORITY_STR: &str = "GS4CU59F31iL7aR2Q8zVS8DRrcRnXX1yjQ66TqNVQnaR";
pub const FEE_RECIPIENT_STR: &str = "62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV";
pub const FEE_RECIPIENT_ATA_STR: &str = "94qWNrtmfn42h3ZjUZwWvK1MEo9uVmmrBPd2hpNjYDjb";

// Lazy loaded Pubkeys
lazy_static::lazy_static! {
    pub static ref PUMP_AMM_PROGRAM_ID: Pubkey = Pubkey::from_str(PUMP_AMM_PROGRAM_ID_STR).unwrap();
    pub static ref ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID_STR).unwrap();
    pub static ref TOKEN_PROGRAM_ID: Pubkey = Pubkey::from_str(TOKEN_PROGRAM_ID_STR).unwrap();
    pub static ref WSOL_TOKEN_ACCOUNT: Pubkey = Pubkey::from_str(WSOL_TOKEN_ACCOUNT_STR).unwrap();
    pub static ref GLOBAL: Pubkey = Pubkey::from_str(GLOBAL_STR).unwrap();
    pub static ref EVENT_AUTHORITY: Pubkey = Pubkey::from_str(EVENT_AUTHORITY_STR).unwrap();
    pub static ref FEE_RECIPIENT: Pubkey = Pubkey::from_str(FEE_RECIPIENT_STR).unwrap();
    pub static ref FEE_RECIPIENT_ATA: Pubkey = Pubkey::from_str(FEE_RECIPIENT_ATA_STR).unwrap();
}

// Default decimals
pub const DEFAULT_DECIMALS: u8 = 6;

// Default commitment level
pub const DEFAULT_COMMITMENT: CommitmentConfig = CommitmentConfig::confirmed();

pub const TRADER_TIP_AMOUNT: f64 = 0.0001;
pub const DEFAULT_SLIPPAGE: u64 = 80000; // 30%
pub const DEFAULT_COMPUTE_UNIT_LIMIT: u32 = 100000;
pub const DEFAULT_COMPUTE_UNIT_PRICE: u64 = 830000;
pub const DEFAULT_BUY_TIP_FEE: f64 = 0.0006;
pub const DEFAULT_SELL_TIP_FEE: f64 = 0.0001;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub struct PriorityFeePump {
    pub unit_limit: u32,
    pub unit_price: u64,
    pub buy_tip_fee: f64,
    pub sell_tip_fee: f64,
    pub jito_tip_fee: f64,
}

impl Default for PriorityFeePump {
    fn default() -> Self {
        Self {
            unit_limit: DEFAULT_COMPUTE_UNIT_LIMIT,
            unit_price: DEFAULT_COMPUTE_UNIT_PRICE,
            buy_tip_fee: DEFAULT_BUY_TIP_FEE,
            sell_tip_fee: DEFAULT_SELL_TIP_FEE,
            jito_tip_fee: 0.0001,
        }
    }
}

// Instruction discriminators
pub const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
