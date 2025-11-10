use solana_program::pubkey::Pubkey;
use std::str::FromStr;

pub const WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const MAX_TICK_INDEX: i32 = 443636;
pub const MIN_TICK_INDEX: i32 = -443636;
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const PUMP_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_FEE_WALLET: &str = "JCRGumoE9Qi5BBgULTgdgTLjSgkCMSbF62ZZfGs84JeU";

pub fn whirlpool_program_id() -> Pubkey {
    Pubkey::from_str(WHIRLPOOL_PROGRAM_ID).unwrap()
}

pub fn sol_mint() -> Pubkey {
    Pubkey::from_str(SOL_MINT).unwrap()
}

pub fn dlmm_program_id() -> Pubkey {
    Pubkey::from_str("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo").unwrap()
}

pub fn dlmm_event_authority() -> Pubkey {
    Pubkey::from_str("D1ZN9Wj1fRSUQfCjhvnu1hqDMT7hzjzBBpi12nVniYD6").unwrap()
}

pub fn pump_program_id() -> Pubkey {
    Pubkey::from_str(PUMP_PROGRAM_ID).unwrap()
}

pub fn pump_fee_wallet() -> Pubkey {
    Pubkey::from_str(PUMP_FEE_WALLET).unwrap()
}

pub fn raydium_program_id() -> Pubkey {
    Pubkey::from_str("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8").unwrap()
}

pub fn raydium_cp_program_id() -> Pubkey {
    Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").unwrap()
}

pub fn raydium_clmm_program_id() -> Pubkey {
    Pubkey::from_str("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK").unwrap()
}
