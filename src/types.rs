use std::str::FromStr;

use jito_protos::block;
use jito_searcher_client::constants::seeds::MINT_AUTHORITY_SEED;
use log::info;
use solana_client::rpc_client::RpcClient;
use solana_sdk::address_lookup_table::state::AddressLookupTable;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::CompiledInstruction;
use solana_sdk::message::AddressLookupTableAccount;
use solana_sdk::pubkey;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::VersionedTransaction;

pub const PUMP_AMM_PROGRAM_ID_STR: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_FUN_PROGRAM_ID_STR: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]

pub struct BuyAmounts {
    pub base_amount_out: u64,
    pub max_quote_amount_in: u64,
}

impl BuyAmounts {
    pub fn new() -> Self {
        Self {
            base_amount_out: 0,
            max_quote_amount_in: 0,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]

pub struct SellAmounts {
    base_amount_in: u64,
    max_quote_amount_out: u64,
}

impl SellAmounts {
    pub fn new() -> Self {
        Self {
            base_amount_in: 0,
            max_quote_amount_out: 0,
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
pub enum Amounts {
    #[default]
    Unknown,

    BuyAmounts(BuyAmounts),
    SellAmounts(SellAmounts),
}

#[derive(Debug)]
pub enum AmountParsingError {
    PassedBuyInSellFunction,
    PassedSellInBuyFunction,
    UnknownAmountPassed,
}

pub fn get_buy_amounts(amounts: Amounts) -> Result<BuyAmounts, AmountParsingError> {
    match amounts {
        Amounts::Unknown => Err(AmountParsingError::UnknownAmountPassed),
        Amounts::BuyAmounts(buy_amounts) => Ok(buy_amounts),
        Amounts::SellAmounts(_) => Err(AmountParsingError::PassedSellInBuyFunction),
    }
}

pub fn get_sell_amounts(amounts: Amounts) -> Result<SellAmounts, AmountParsingError> {
    match amounts {
        Amounts::Unknown => Err(AmountParsingError::UnknownAmountPassed),
        Amounts::BuyAmounts(_) => Err(AmountParsingError::PassedBuyInSellFunction),
        Amounts::SellAmounts(sell_amounts) => Ok(sell_amounts),
    }
}

#[derive(Debug)]
pub enum TXParserError {
    NotEnoughWallets,
    MissingAccountKeys,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransactionType {
    Buy,
    Sell,
    Unknown,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct PumpSwapTXInfo {
    pub blockhash: Hash,
    pub pool: Pubkey,
    pub user: Pubkey,
    pub global_config: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub user_base_token_account: Pubkey,
    pub user_quote_token_account: Pubkey,
    pub pool_base_token_account: Pubkey,
    pub pool_quote_token_account: Pubkey,
    pub protocol_fee_recipient: Pubkey,
    pub protocol_fee_recipient_token_account: Pubkey,
    pub base_token_program: Pubkey,
    pub quote_token_program: Pubkey,
    pub system_program: Pubkey,
    pub associated_token_program: Pubkey,
    pub event_authority: Pubkey,
    pub program: Pubkey,
    pub amounts: Amounts,
}

fn create_base_tx_pumpswap(
    tx: &VersionedTransaction,
    ix: &CompiledInstruction,
) -> Result<PumpSwapTXInfo, TXParserError> {
    let blockhash = tx.message.recent_blockhash().to_owned();
    let account_keys = tx.message.static_account_keys();
    // let client = RpcClient::new(
    //     "https://mainnet.helius-rpc.com/?api-key=f44bbff8-2b1c-450d-bd79-e9ab5ec60ba9".to_string(),
    // );

    // let mut keys = Vec::new();
    // let lookup_table = tx.message.address_table_lookups();
    // if lookup_table.is_some() {
    //     info!("The lookup table has something! {:?}", lookup_table);

    //     for table in lookup_table.unwrap() {
    //         let raw_account = client.get_account(&table.account_key).unwrap();
    //         let address_lookup_table = AddressLookupTable::deserialize(&raw_account.data).unwrap();
    //         let address_lookup_table_account = AddressLookupTableAccount {
    //             key: table.account_key,
    //             addresses: address_lookup_table.addresses.to_vec(),
    //         };

    //         println!(
    //             "THE FUCKING LOOKUP TABLE: {:?}",
    //             address_lookup_table_account
    //         );
    //     }
    // } else {
    //     info!("The lookup table is gay")
    // }

    if account_keys.len() <= ix.accounts.iter().max().unwrap().to_owned() as usize {
        println!("Missing account keys!");
        println!(
            "Account Keys: {:?}\nIX Account keys: {:?}",
            account_keys, ix.accounts
        );
        return Err(TXParserError::NotEnoughWallets);
    }

    // 17 acccounts in there
    let pool = account_keys[ix.accounts[0] as usize];
    let user = account_keys[ix.accounts[1] as usize];
    let global_config = account_keys[ix.accounts[2] as usize];
    let base_mint = account_keys[ix.accounts[3] as usize];
    let quote_mint = account_keys[ix.accounts[4] as usize];
    let user_base_token_account = account_keys[ix.accounts[5] as usize];
    let user_quote_token_account = account_keys[ix.accounts[6] as usize];
    let pool_base_token_account = account_keys[ix.accounts[7] as usize];
    let pool_quote_token_account = account_keys[ix.accounts[8] as usize];
    let protocol_fee_recipient: Pubkey = account_keys[ix.accounts[9] as usize];
    let protocol_fee_recipient_token_account = account_keys[ix.accounts[10] as usize];
    let base_token_program = account_keys[ix.accounts[11] as usize];
    let quote_token_program = account_keys[ix.accounts[12] as usize];
    let system_program = account_keys[ix.accounts[13] as usize];
    let associated_token_program = account_keys[ix.accounts[14] as usize];
    let event_authority = account_keys[ix.accounts[15] as usize];
    let program = account_keys[ix.accounts[16] as usize];

    Ok(PumpSwapTXInfo {
        blockhash,
        pool,
        user,
        global_config,
        base_mint,
        quote_mint,
        user_base_token_account,
        user_quote_token_account,
        pool_base_token_account,
        pool_quote_token_account,
        protocol_fee_recipient,
        protocol_fee_recipient_token_account,
        base_token_program,
        quote_token_program,
        system_program,
        associated_token_program,
        event_authority,
        program,
        amounts: Amounts::default(),
    })
}

impl Default for PumpSwapTXInfo {
    fn default() -> Self {
        PumpSwapTXInfo::new()
    }
}

impl PumpSwapTXInfo {
    fn new() -> Self {
        Self {
            blockhash: Hash::new_unique(),
            pool: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            global_config: Pubkey::new_unique(),
            base_mint: Pubkey::new_unique(),
            quote_mint: Pubkey::new_unique(),
            user_base_token_account: Pubkey::new_unique(),
            user_quote_token_account: Pubkey::new_unique(),
            pool_base_token_account: Pubkey::new_unique(),
            pool_quote_token_account: Pubkey::new_unique(),
            protocol_fee_recipient: Pubkey::new_unique(),
            protocol_fee_recipient_token_account: Pubkey::new_unique(),
            base_token_program: Pubkey::new_unique(),
            quote_token_program: Pubkey::new_unique(),
            system_program: Pubkey::new_unique(),
            associated_token_program: Pubkey::new_unique(),
            event_authority: Pubkey::new_unique(),
            program: Pubkey::new_unique(),
            amounts: Amounts::default(),
        }
    }

    pub fn from_buy_tx(
        tx: &VersionedTransaction,
        ix: &CompiledInstruction,
    ) -> Result<Self, TXParserError> {
        let mut base_tx = create_base_tx_pumpswap(tx, ix)?;

        let base_amount_out = u64::from_le_bytes(
            ix.data[ix.data.len() - 16..ix.data.len() - 8]
                .try_into()
                .unwrap(),
        );
        let max_quote_amount_in = u64::from_le_bytes(
            ix.data[ix.data.len() - 8..ix.data.len()]
                .try_into()
                .unwrap(),
        );

        base_tx.amounts = Amounts::BuyAmounts(BuyAmounts {
            base_amount_out,
            max_quote_amount_in,
        });

        Ok(base_tx)
    }

    pub fn from_sell_tx(
        tx: &VersionedTransaction,
        ix: &CompiledInstruction,
    ) -> Result<Self, TXParserError> {
        let mut base_tx = create_base_tx_pumpswap(tx, ix)?;

        let base_amount_in = u64::from_le_bytes(
            ix.data[ix.data.len() - 16..ix.data.len() - 8]
                .try_into()
                .unwrap(),
        );
        let max_quote_amount_out = u64::from_le_bytes(
            ix.data[ix.data.len() - 8..ix.data.len()]
                .try_into()
                .unwrap(),
        );

        base_tx.amounts = Amounts::SellAmounts(SellAmounts {
            base_amount_in,
            max_quote_amount_out,
        });

        Ok(base_tx)
    }
}

#[derive(Debug)]
pub struct PumpFunTXInfo {
    pub blockhash: Hash,
    pub fee_recipient: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub associated_bonding_curve: Pubkey,
    pub associated_user: Pubkey,
    pub user: Pubkey,
    pub amounts: Amounts,
    pub creator: Pubkey,
}

fn create_base_tx_pumpfun(
    tx: &VersionedTransaction,
    ix: &CompiledInstruction,
) -> Result<PumpFunTXInfo, TXParserError> {
    let blockhash = tx.message.recent_blockhash().to_owned();
    let account_keys = tx.message.static_account_keys();

    if account_keys.len() < 12 {
        println!("This transaction doesn't even have enough wallets.");
        return Err(TXParserError::MissingAccountKeys);
    }

    if account_keys.len() < ix.accounts.iter().max().unwrap().to_owned() as usize {
        println!("Missing account keys!");
        println!(
            "Account Keys: {:?}\nIX Account keys: {:?}",
            account_keys, ix.accounts
        );
        return Err(TXParserError::NotEnoughWallets);
    }

    // 17 acccounts in there
    let fee_recipient = account_keys[ix.accounts[1] as usize];
    let mint = account_keys[ix.accounts[2] as usize];
    let bonding_curve = account_keys[ix.accounts[3] as usize];
    let associated_bonding_curve = account_keys[ix.accounts[4] as usize];
    let associated_user = account_keys[ix.accounts[5] as usize];
    let user = account_keys[ix.accounts[6] as usize];
    let creator = account_keys[ix.accounts[10] as usize];

    Ok(PumpFunTXInfo {
        blockhash,
        fee_recipient,
        mint,
        bonding_curve,
        associated_bonding_curve,
        associated_user,
        user,
        amounts: Amounts::default(),
        creator,
    })
}

fn create_base_create_tx_pumpfun(
    tx: &VersionedTransaction,
    ix: &CompiledInstruction,
) -> Result<PumpFunTXInfo, TXParserError> {
    let blockhash = tx.message.recent_blockhash().to_owned();
    let account_keys = tx.message.static_account_keys();

    if account_keys.len() < 12 {
        println!("This transaction doesn't even have enough wallets.");
        return Err(TXParserError::MissingAccountKeys);
    }

    if account_keys.len() < ix.accounts.iter().max().unwrap().to_owned() as usize {
        println!("Missing account keys!");
        println!(
            "Account Keys: {:?}\nIX Account keys: {:?}",
            account_keys, ix.accounts
        );
        return Err(TXParserError::NotEnoughWallets);
    }

    // 17 acccounts in there
    let mint = account_keys[ix.accounts[0] as usize];
    // let mint_authority = account_keys[ix.accounts[1] as usize];
    let bonding_curve = account_keys[ix.accounts[2] as usize];
    let associated_bonding_curve = account_keys[ix.accounts[4] as usize];
    let default_key = Pubkey::default();

    let mut creator: Pubkey = Pubkey::default();
    let mut amounts = Amounts::default();

    for instructions in tx.message.instructions() {
        let program_id = account_keys[instructions.program_id_index as usize];

        if program_id == Pubkey::from_str(PUMP_FUN_PROGRAM_ID_STR).unwrap() {
            let data_prefix: [u8; 8] = instructions.data[0..8].try_into().unwrap();
            if data_prefix == PUMP_ORDER_BUY {
                creator = account_keys[instructions.accounts[9] as usize];
                let base_amount_out = u64::from_le_bytes(
                    instructions.data[instructions.data.len() - 16..instructions.data.len() - 8]
                        .try_into()
                        .unwrap_or_default(),
                );
                let max_quote_amount_in = u64::from_le_bytes(
                    instructions.data[instructions.data.len() - 8..instructions.data.len()]
                        .try_into()
                        .unwrap_or_default(),
                );

                amounts = Amounts::BuyAmounts(BuyAmounts {
                    base_amount_out,
                    max_quote_amount_in,
                });
                break;
            }
        }
    }

    Ok(PumpFunTXInfo {
        blockhash,
        fee_recipient: default_key,
        mint,
        bonding_curve,
        associated_bonding_curve,
        associated_user: default_key,
        user: default_key,
        amounts,
        creator,
    })
}

impl Default for PumpFunTXInfo {
    fn default() -> Self {
        PumpFunTXInfo::new()
    }
}

impl PumpFunTXInfo {
    fn new() -> Self {
        Self {
            blockhash: Hash::new_unique(),
            fee_recipient: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            bonding_curve: Pubkey::new_unique(),
            associated_bonding_curve: Pubkey::new_unique(),
            associated_user: Pubkey::new_unique(),
            user: Pubkey::new_unique(),
            amounts: Amounts::default(),
            creator: Pubkey::new_unique(),
        }
    }

    pub fn from_create_tx(
        tx: &VersionedTransaction,
        ix: &CompiledInstruction,
    ) -> Result<Self, TXParserError> {
        create_base_create_tx_pumpfun(tx, ix)
    }

    // pub fn from_axiom_tx(
    //     tx: &VersionedTransaction,
    //     ix: &CompiledInstruction,
    // ) -> Result<Self, TXParserError> {
    // }

    pub fn from_buy_tx(
        tx: &VersionedTransaction,
        ix: &CompiledInstruction,
    ) -> Result<Self, TXParserError> {
        let mut base_tx = create_base_tx_pumpfun(tx, ix)?;

        let base_amount_out = u64::from_le_bytes(
            ix.data[ix.data.len() - 16..ix.data.len() - 8]
                .try_into()
                .unwrap_or_default(),
        );
        let max_quote_amount_in = u64::from_le_bytes(
            ix.data[ix.data.len() - 8..ix.data.len()]
                .try_into()
                .unwrap_or_default(),
        );

        base_tx.amounts = Amounts::BuyAmounts(BuyAmounts {
            base_amount_out,
            max_quote_amount_in,
        });

        Ok(base_tx)
    }

    pub fn from_sell_tx(
        tx: &VersionedTransaction,
        ix: &CompiledInstruction,
    ) -> Result<Self, TXParserError> {
        let mut base_tx = create_base_tx_pumpfun(tx, ix)?;

        let base_amount_in = u64::from_le_bytes(
            ix.data[ix.data.len() - 16..ix.data.len() - 8]
                .try_into()
                .unwrap(),
        );
        let max_quote_amount_out = u64::from_le_bytes(
            ix.data[ix.data.len() - 8..ix.data.len()]
                .try_into()
                .unwrap(),
        );

        base_tx.amounts = Amounts::SellAmounts(SellAmounts {
            base_amount_in,
            max_quote_amount_out,
        });

        Ok(base_tx)
    }
}

pub const PUMP_ORDER_BUY: [u8; 8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMP_ORDER_SELL: [u8; 8] = [51, 230, 133, 164, 1, 127, 131, 173];
