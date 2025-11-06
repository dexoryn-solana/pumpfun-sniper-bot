use crate::pumpswap::{constants::*, error::PumpSwapResult};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use spl_associated_token_account::get_associated_token_address;

pub fn create_buy_instruction(
    pool_id: &Pubkey,
    user: &Pubkey,
    mint: &Pubkey,
    base_amount_out: u64,
    max_quote_amount_in: u64,
    creator_pubkey: &Pubkey, // Token creator for revenue sharing
) -> PumpSwapResult<Instruction> {
    // Compute associated token account addresses
    let user_base_token_account = get_associated_token_address(user, mint);
    let user_quote_token_account = get_associated_token_address(user, &WSOL_TOKEN_ACCOUNT);
    let pool_base_token_account = get_associated_token_address(pool_id, mint);
    let pool_quote_token_account = get_associated_token_address(pool_id, &WSOL_TOKEN_ACCOUNT);

    // Creator vault authority PDA for revenue sharing
    let coin_creator_vault_authority = Pubkey::find_program_address(
        &[b"creator_vault", creator_pubkey.as_ref()],
        &PUMP_AMM_PROGRAM_ID,
    )
    .0;

    // Creator vault ATA for revenue sharing
    let coin_creator_vault_ata = Pubkey::find_program_address(
        &[
            coin_creator_vault_authority.as_ref(),
            TOKEN_PROGRAM_ID.as_ref(),
            WSOL_TOKEN_ACCOUNT.as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0;

    // Define the accounts for the instruction
    let accounts = vec![
        AccountMeta::new_readonly(*pool_id, false),
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(*GLOBAL, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*WSOL_TOKEN_ACCOUNT, false),
        AccountMeta::new(user_base_token_account, false),
        AccountMeta::new(user_quote_token_account, false),
        AccountMeta::new(pool_base_token_account, false),
        AccountMeta::new(pool_quote_token_account, false),
        AccountMeta::new_readonly(*FEE_RECIPIENT, false),
        AccountMeta::new(*FEE_RECIPIENT_ATA, false),
        AccountMeta::new_readonly(*TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(*TOKEN_PROGRAM_ID, false), // duplicated as in the TS code
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*ASSOCIATED_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(*EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(*PUMP_AMM_PROGRAM_ID, false),
        AccountMeta::new(coin_creator_vault_ata, false), // 18th account - creator vault ATA
        AccountMeta::new_readonly(coin_creator_vault_authority, false), // 19th account - creator vault authority
    ];

    // Pack the instruction data
    let mut data = Vec::with_capacity(24); // 8 + 8 + 8
    data.extend_from_slice(&BUY_DISCRIMINATOR);
    data.extend_from_slice(&base_amount_out.to_le_bytes());
    data.extend_from_slice(&max_quote_amount_in.to_le_bytes());

    // Create the instruction
    Ok(Instruction {
        program_id: *PUMP_AMM_PROGRAM_ID,
        accounts,
        data,
    })
}

/// Create sell instruction
pub fn create_sell_instruction(
    pool_id: &Pubkey,
    user: &Pubkey,
    mint: &Pubkey,
    base_amount_in: u64,
    min_quote_amount_out: u64,
    creator_pubkey: &Pubkey, // Token creator for revenue sharing
) -> PumpSwapResult<Instruction> {
    // Compute associated token account addresses
    let user_base_token_account = get_associated_token_address(user, mint);
    let user_quote_token_account = get_associated_token_address(user, &WSOL_TOKEN_ACCOUNT);
    let pool_base_token_account = get_associated_token_address(pool_id, mint);
    let pool_quote_token_account = get_associated_token_address(pool_id, &WSOL_TOKEN_ACCOUNT);

    // Creator vault authority PDA for revenue sharing
    let coin_creator_vault_authority = Pubkey::find_program_address(
        &[b"creator_vault", creator_pubkey.as_ref()],
        &PUMP_AMM_PROGRAM_ID,
    )
    .0;

    // Creator vault ATA for revenue sharing
    let coin_creator_vault_ata = Pubkey::find_program_address(
        &[
            coin_creator_vault_authority.as_ref(),
            TOKEN_PROGRAM_ID.as_ref(),
            WSOL_TOKEN_ACCOUNT.as_ref(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0;

    // Define the accounts for the instruction
    let accounts = vec![
        AccountMeta::new_readonly(*pool_id, false),
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(*GLOBAL, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*WSOL_TOKEN_ACCOUNT, false),
        AccountMeta::new(user_base_token_account, false),
        AccountMeta::new(user_quote_token_account, false),
        AccountMeta::new(pool_base_token_account, false),
        AccountMeta::new(pool_quote_token_account, false),
        AccountMeta::new_readonly(*FEE_RECIPIENT, false),
        AccountMeta::new(*FEE_RECIPIENT_ATA, false),
        AccountMeta::new_readonly(*TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(*TOKEN_PROGRAM_ID, false), // duplicated as in the TS code
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*ASSOCIATED_TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(*EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(*PUMP_AMM_PROGRAM_ID, false),
        AccountMeta::new(coin_creator_vault_ata, false), // 18th account - creator vault ATA
        AccountMeta::new_readonly(coin_creator_vault_authority, false), // 19th account - creator vault authority
    ];

    // Pack the instruction data
    let mut data = Vec::with_capacity(24); // 8 + 8 + 8
    data.extend_from_slice(&SELL_DISCRIMINATOR);
    data.extend_from_slice(&base_amount_in.to_le_bytes());
    data.extend_from_slice(&min_quote_amount_out.to_le_bytes());

    // Create the instruction
    Ok(Instruction {
        program_id: *PUMP_AMM_PROGRAM_ID,
        accounts,
        data,
    })
}
