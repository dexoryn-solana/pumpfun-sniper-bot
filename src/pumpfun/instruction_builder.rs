use crate::pumpfun::{constants::*, error::PumpFunResult};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

pub fn create_buy_instruction(
    user: &Pubkey,
    mint: &Pubkey,
    user_ata: &Pubkey,
    amount: u64,
    max_sol_cost: u64,
    creator_pubkey: &Pubkey, // Token creator for revenue sharing
) -> PumpFunResult<Instruction> {
    let (bonding_curve, _) =
        Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &PUMPFUN_PUMP_PROGRAM);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[
            bonding_curve.as_array(),
            PUMPFUN_TOKEN_PROGRAM.as_array(),
            mint.as_ref(),
        ],
        &PUMPFUN_ASSOC_TOKEN_PROGRAM,
    );

    // Define the accounts for the instruction
    let accounts = vec![
        AccountMeta::new_readonly(*PUMPFUN_GLOBAL_ACCOUNT, false),
        AccountMeta::new(*PUMPFUN_PUMP_FEE_ACC, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(*user_ata, false),
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*PUMPFUN_TOKEN_PROGRAM, false),
        AccountMeta::new(*creator_pubkey, false), // Creator account for revenue sharing
        AccountMeta::new_readonly(*PUMPFUN_PUMP_EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(*PUMPFUN_PUMP_PROGRAM, false),
    ];

    // Pack the instruction data
    let mut data = Vec::with_capacity(24); // 8 + 8 + 8
    data.extend_from_slice(&BUY_DISCRIMINATOR);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&max_sol_cost.to_le_bytes());

    // Create the instruction
    Ok(Instruction {
        program_id: *PUMPFUN_PUMP_PROGRAM,
        accounts,
        data,
    })
}

/// Create sell instruction
pub fn create_sell_instruction(
    user: &Pubkey,
    mint: &Pubkey,
    user_ata: &Pubkey,
    amount: u64,
    min_sol_output: u64,
    creator_pubkey: &Pubkey, // Token creator for revenue sharing
) -> PumpFunResult<Instruction> {
    let (bonding_curve, _) =
        Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &PUMPFUN_PUMP_PROGRAM);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[
            bonding_curve.as_array(),
            PUMPFUN_TOKEN_PROGRAM.as_array(),
            mint.as_ref(),
        ],
        &PUMPFUN_ASSOC_TOKEN_PROGRAM,
    );

    // Define the accounts for the instruction
    let accounts = vec![
        AccountMeta::new_readonly(*PUMPFUN_GLOBAL_ACCOUNT, false),
        AccountMeta::new(*PUMPFUN_PUMP_FEE_ACC, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(associated_bonding_curve, false),
        AccountMeta::new(*user_ata, false),
        AccountMeta::new(*user, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(*creator_pubkey, false), // Creator account for revenue sharing
        AccountMeta::new_readonly(*PUMPFUN_TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(*PUMPFUN_PUMP_EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(*PUMPFUN_PUMP_PROGRAM, false),
    ];

    // Pack the instruction data
    let mut data = Vec::with_capacity(24); // 8 + 8 + 8
    data.extend_from_slice(&SELL_DISCRIMINATOR);
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&min_sol_output.to_le_bytes());

    // Create the instruction
    Ok(Instruction {
        program_id: *PUMPFUN_PUMP_PROGRAM,
        accounts,
        data,
    })
}
