use solana_sdk::pubkey::Pubkey;

#[derive(Default, Debug, Clone)]
pub struct Accounts {
    pub base: (Pubkey, u64),
    pub quote: (Pubkey, u64),
}

pub fn calculate_with_slippage_buy(amount: u64, basis_points: u64) -> u64 {
    (amount as f64 + (amount as f64 * (basis_points as f64 / 100.0))) as u64
}

pub fn calculate_with_slippage_sell(amount: u64, basis_points: u64) -> u64 {
    (amount as f64 + (amount as f64 * (basis_points as f64 / 100.0))) as u64
}

pub fn get_buy_token_amount_with_pool_reserves(sol_amount: u64, pool_detail: &Accounts) -> u64 {
    let token_reserve = pool_detail.quote.1;
    let sol_reserve = pool_detail.base.1;

    println!(
        "Sol reserves: {} || Token reserves: {}",
        sol_reserve, token_reserve
    );

    // token_reserve = 632404304 * 1e6 // lamports = 632404304000000
    // sol_reserve = 33.82 * 1e9 // lamports = 33820000000
    let product = sol_reserve as u128 * token_reserve as u128; // 21387913561280000000000000
    let new_sol_reserve = sol_reserve as u128 + sol_amount as u128; // 100000000 + 33820000000 = 33920000000
    let new_token_reserve = product / new_sol_reserve + 1; // 21387913561280000000000000 / 33920000000 + 1 = 630539904518868
    let amount_to_be_purchased = token_reserve as u128 - new_token_reserve; // 632404304000000 - 630539904518868 = 1864399481132

    amount_to_be_purchased as u64
}
