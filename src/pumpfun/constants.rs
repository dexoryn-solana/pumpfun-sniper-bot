use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

pub const GLOBAL_ACCOUNT_STR: &str = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf";
pub const PUMP_FEE_ACC_STR: &str = "CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM";
pub const SYSTEM_PROGRAM_STR: &str = "11111111111111111111111111111111";
pub const TOKEN_PROGRAM_STR: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
pub const ASSOC_TOKEN_PROGRAM_STR: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
pub const RENT_PROGRAM_STR: &str = "SysvarRent111111111111111111111111111111111";
pub const PUMP_EVENT_AUTHORITY_STR: &str = "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1";
pub const PUMP_PROGRAM_STR: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

lazy_static::lazy_static! {
    pub static ref PUMPFUN_GLOBAL_ACCOUNT: Pubkey = Pubkey::from_str(GLOBAL_ACCOUNT_STR).unwrap();
    pub static ref PUMPFUN_PUMP_FEE_ACC: Pubkey = Pubkey::from_str(PUMP_FEE_ACC_STR).unwrap();
    pub static ref PUMPFUN_SYSTEM_PROGRAM: Pubkey = Pubkey::from_str(SYSTEM_PROGRAM_STR).unwrap();
    pub static ref PUMPFUN_TOKEN_PROGRAM: Pubkey = Pubkey::from_str(TOKEN_PROGRAM_STR).unwrap();
    pub static ref PUMPFUN_ASSOC_TOKEN_PROGRAM: Pubkey = Pubkey::from_str(ASSOC_TOKEN_PROGRAM_STR).unwrap();
    pub static ref PUMPFUN_RENT_PROGRAM: Pubkey = Pubkey::from_str(RENT_PROGRAM_STR).unwrap();
    pub static ref PUMPFUN_PUMP_EVENT_AUTHORITY: Pubkey = Pubkey::from_str(PUMP_EVENT_AUTHORITY_STR).unwrap();
    pub static ref PUMPFUN_PUMP_PROGRAM: Pubkey = Pubkey::from_str(PUMP_PROGRAM_STR).unwrap();
}

pub const BUY_DISCRIMINATOR: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const SELL_DISCRIMINATOR: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
