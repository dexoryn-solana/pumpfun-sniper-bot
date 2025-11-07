use crate::pumpswap::constants::*;
use crate::pumpswap::error::{PumpSwapError, PumpSwapResult};
use crate::pumpswap::instruction_builder;
use crate::pumpswap::pool::calculate_with_slippage_buy;
use crate::types::BuyAmounts;
use solana_client::rpc_config::RpcSendTransactionConfig;

use solana_sdk::instruction::AccountMeta;
use solana_sdk::signer::Signer;

use log::info;

use solana_sdk::message::Message;
use spl_associated_token_account::get_associated_token_address;
use std::{str::FromStr, sync::Arc};

use rustls::crypto::{CryptoProvider, ring::default_provider};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::hash::Hash;
use solana_sdk::pubkey;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, message::v0::Message as MessageV0,
    native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair, system_instruction,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;

const ASTRALANE_TIP: Pubkey = pubkey!("astra4uejePWneqNaJKuFFA8oonqCE1sqF6b45kDMZm");

#[derive(Clone)]
pub struct PumpSwapSDK {
    pub client: Arc<RpcClient>,
    pub astralane_client: Arc<RpcClient>,
    // pub jito_client: Arc<JitoClient>,
    pub priority_fee: PriorityFeePump,
}

pub struct PumpSwapBuyType {
    pub mint: Pubkey,
    pub sol_to_buy: f64,
    pub user_keypair: Keypair,
    pub blockhash: Hash,
    pub pool: Pubkey,
    pub buy_amounts: BuyAmounts,
    pub slippage: u64,
    pub cu_price: u64,
    pub creator_pubkey: Pubkey, // TODO: new pumpswap update
}

pub struct PumpSwapSellType {
    pub mint: Pubkey,
    pub token_amount_lamports: u64,
    pub user_keypair: Keypair,
    pub blockhash: Hash,
    pub pool: Pubkey,
    pub slippage: u64,
    pub cu_price: u64,
    pub creator_pubkey: Pubkey, // TODO: new pumpswap update
}

impl PumpSwapSDK {
    /// Create a new PumpSwapSDK instance
    pub async fn new(rpc_url: &str, astralane_url: &str, priority_fee: PriorityFeePump) -> Self {
        if CryptoProvider::get_default().is_none() {
            default_provider().install_default().unwrap();
        }
        let client = Arc::new(RpcClient::new(String::from(rpc_url)));
        let astralane_client = Arc::new(RpcClient::new(astralane_url.to_string()));

        PumpSwapSDK {
            client,
            astralane_client,
            // jito_client: Arc::new(
            //     JitoClient::new(rpc_url.to_string(), jito_block_engine_url.to_string())
            //         .await
            //         .unwrap(),
            // ),
            priority_fee,
        }
    }

    /// Buy tokens with SOL
    pub async fn create_buy_tx(
        &self,
        params: PumpSwapBuyType,
    ) -> PumpSwapResult<(VersionedTransaction, u64)> {
        let sol_to_buy_lamports = sol_to_lamports(params.sol_to_buy);
        let user_pubkey = params.user_keypair.pubkey();

        let price: f64 = (params.buy_amounts.max_quote_amount_in as f64 / 1_000_000_000.0)
            / (params.buy_amounts.base_amount_out as f64 / 1_000_000.0);

        let bought_token_amount =
            (sol_to_buy_lamports as f64 * (price * 1_000_000.0) * 1_000_000.0) as u64;
        let sol_to_buy_after_slippage =
            calculate_with_slippage_buy(sol_to_buy_lamports, params.slippage);
        let user = params.user_keypair.pubkey();

        info!(
            "🚀 BUY REQUEST: Mint {} - User {} - Amount {} SOL ({} lamports)",
            params.mint, &user_pubkey, params.sol_to_buy, sol_to_buy_lamports
        );

        info!("Token price: {}", price);
        info!(
            "Sol to lamports after slippage: {}",
            sol_to_buy_after_slippage
        );
        info!("Bought token amount: {}", bought_token_amount);

        // Create buy instruction
        let pumpswap_buy_tx = instruction_builder::create_buy_instruction(
            &params.pool,
            &user,
            &params.mint,
            bought_token_amount,
            sol_to_buy_after_slippage,
            &params.creator_pubkey,
        )?;

        // Get WSOL token account
        let user_wsol_token_account = get_associated_token_address(&user, &WSOL_TOKEN_ACCOUNT);

        // Create instructions for WSOL handling
        let create_wsol_account_ix = create_associated_token_account_idempotent(
            &user,
            &user,
            &WSOL_TOKEN_ACCOUNT,
            &TOKEN_PROGRAM_ID,
        );

        // Create instruction to transfer SOL to WSOL account
        let transfer_sol_ix =
            system_instruction::transfer(&user, &user_wsol_token_account, sol_to_buy_lamports);

        // Create instruction to sync WSOL account after transfer
        let sync_native_ix =
            spl_token_2022::instruction::sync_native(&TOKEN_PROGRAM_ID, &user_wsol_token_account)
                .unwrap();

        // Create instruction to close WSOL account after swap
        let close_wsol_account_ix = spl_token_2022::instruction::close_account(
            &TOKEN_PROGRAM_ID,
            &user_wsol_token_account,
            &user,
            &user,
            &[],
        )
        .unwrap();

        // Create ATA creation instruction for token being bought
        let create_token_ata_ix = create_associated_token_account_idempotent(
            &user,
            &user,
            &params.mint,
            &TOKEN_PROGRAM_ID,
        );

        // Create compute budget instructions
        let mut compute_unit_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(150_000);
        let jitodontfront = AccountMeta {
            pubkey: Pubkey::from_str("jitodontfront11111111111111111ButMyNiggasDo").unwrap(),
            is_signer: false,
            is_writable: false,
        };
        compute_unit_limit_ix.accounts.push(jitodontfront);
        let compute_unit_price_ix =
            ComputeBudgetInstruction::set_compute_unit_price(params.cu_price * 1_000_000); // 25 lamports

        let astralane_fee_tx = system_instruction::transfer(
            &user,
            &ASTRALANE_TIP,
            sol_to_lamports(self.priority_fee.jito_tip_fee),
        );

        let instructions = vec![
            compute_unit_limit_ix,
            compute_unit_price_ix,
            astralane_fee_tx,
            create_wsol_account_ix,
            transfer_sol_ix,
            sync_native_ix,
            create_token_ata_ix,
            pumpswap_buy_tx,
            close_wsol_account_ix,
        ];

        // Create and sign transaction
        let message = Message::new_with_blockhash(&instructions, Some(&user), &params.blockhash);

        let tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::Legacy(message),
            &[params.user_keypair],
        )
        .map_err(|e| PumpSwapError::TransactionError(e.to_string()))?;

        Ok((tx, bought_token_amount))
    }

    // Sell exact amount of tokens
    pub async fn create_sell_exact_amount_tx(
        &self,
        params: PumpSwapSellType,
    ) -> PumpSwapResult<VersionedTransaction> {
        let user = params.user_keypair.pubkey();

        info!(
            "🚀 SELL REQUEST: Mint {} - User {} - Amount {} lamports",
            params.mint, user, params.token_amount_lamports
        );

        // let token_amt_lamports = token_amount * 10;
        // let slippage_sell_amount_tokens = calculate_with_slippage_sell(, 30_000);
        // let slippage_sell_amount_sol =

        let user_wsol_token_account = get_associated_token_address(&user, &WSOL_TOKEN_ACCOUNT);

        // Create instructions for WSOL handling
        let create_wsol_account_ix = create_associated_token_account_idempotent(
            &user,
            &user,
            &WSOL_TOKEN_ACCOUNT,
            &TOKEN_PROGRAM_ID,
        );

        // Create instruction to sync WSOL account
        let sync_native_ix =
            spl_token_2022::instruction::sync_native(&TOKEN_PROGRAM_ID, &user_wsol_token_account)
                .unwrap();

        // Create instruction to close WSOL account after swap
        let close_wsol_account_ix = spl_token_2022::instruction::close_account(
            &TOKEN_PROGRAM_ID,
            &user_wsol_token_account,
            &user,
            &user,
            &[],
        )
        .unwrap();

        let create_ata_ix = create_associated_token_account_idempotent(
            &user,
            &user,
            &params.mint,
            &TOKEN_PROGRAM_ID,
        );

        // Create sell instruction
        let pumpswap_sell_tx = instruction_builder::create_sell_instruction(
            &params.pool,
            &user,
            &params.mint,
            params.token_amount_lamports,
            0,
            &params.creator_pubkey,
        )
        .unwrap();

        // Create compute budget instructions
        let mut compute_unit_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(150_000);
        let jitodontfront = AccountMeta {
            pubkey: Pubkey::from_str("jitodontfront11111111111111111ButMyNiggasDo").unwrap(),
            is_signer: false,
            is_writable: false,
        };
        compute_unit_limit_ix.accounts.push(jitodontfront);
        let compute_unit_price_ix =
            ComputeBudgetInstruction::set_compute_unit_price(params.cu_price * 1_000_000);

        // Combine instructions
        let astralane_fee_tx = system_instruction::transfer(
            &user,
            &ASTRALANE_TIP,
            sol_to_lamports(self.priority_fee.jito_tip_fee),
        );

        let instructions = vec![
            compute_unit_limit_ix,
            compute_unit_price_ix,
            astralane_fee_tx,
            create_wsol_account_ix,
            sync_native_ix,
            create_ata_ix,
            pumpswap_sell_tx,
            close_wsol_account_ix,
        ];

        // Create and sign transaction
        let message = MessageV0::try_compile(&user, &instructions, &[], params.blockhash)
            .map_err(|e| PumpSwapError::TransactionError(e.to_string()))?;
        let tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[params.user_keypair],
        )
        .map_err(|e| PumpSwapError::TransactionError(e.to_string()))?;

        Ok(tx)
    }

    pub async fn sell(
        self,
        mint: Pubkey,
        token_amount_lamports: u64,
        user_keypair: Keypair,
        blockhash: Hash,
        pool: Pubkey,
        creator_pubkey: Pubkey,
    ) -> PumpSwapResult<String> {
        let pumpswap_params = PumpSwapSellType {
            mint,
            token_amount_lamports,
            user_keypair,
            blockhash,
            pool,
            slippage: 99,
            cu_price: 5,
            creator_pubkey,
        };

        let tx_astra = self
            .create_sell_exact_amount_tx(pumpswap_params)
            .await
            .unwrap();

        self.place_tx(tx_astra).await
    }

    pub async fn buy(
        self,
        mint: Pubkey,
        sol_to_buy: f64,
        user_keypair: Keypair,
        blockhash: Hash,
        pool: Pubkey,
        buy_amounts: BuyAmounts,
        creator_pubkey: Pubkey,
    ) -> PumpSwapResult<(String, u64)> {
        let pumpswap_params = PumpSwapBuyType {
            mint,
            sol_to_buy,
            user_keypair,
            blockhash,
            pool,
            buy_amounts,
            slippage: 99,
            cu_price: 5,
            creator_pubkey,
        };

        let tx_astra = self.create_buy_tx(pumpswap_params).await.unwrap();

        Ok((self.place_tx(tx_astra.0).await?, tx_astra.1))
    }

    async fn place_tx(self, tx: VersionedTransaction) -> PumpSwapResult<String> {
        let txconfig = RpcSendTransactionConfig {
            skip_preflight: true,
            ..Default::default()
        };

        match self
            .astralane_client
            .send_transaction_with_config(&tx, txconfig)
            .await
        {
            Ok(signs) => Ok(signs.to_string()),
            Err(e) => Err(PumpSwapError::TransactionError(e.to_string())),
        }
    }

    // pub async fn sandwich_attack(
    //     &self,
    //     sol_to_mev: f64,
    //     user_keypair: &Keypair,
    //     tx_info: &PumpSwapTXInfo,
    //     tx_to_sandwich: &VersionedTransaction,
    // ) -> PumpSwapResult<String> {
    //     let buy_amounts = get_buy_amounts(tx_info.amounts).unwrap();

    //     // make your own buy tx
    //     let buy_tx = self
    //         .create_buy_tx(
    //             &tx_info.base_mint,
    //             &user_keypair.pubkey(),
    //             sol_to_mev,
    //             user_keypair,
    //             Some(tx_to_sandwich.message.recentblockhash().to_owned()),
    //             &tx_info.pool,
    //             buy_amounts,
    //             false,
    //         )
    //         .await
    //         .unwrap();

    //     let sell_tx = self
    //         .create_sell_exact_amount_tx(
    //             &tx_info.base_mint,
    //             &user_keypair.pubkey(),
    //             buy_tx.1,
    //             user_keypair,
    //             Some(tx_to_sandwich.message.recentblockhash().to_owned()),
    //             &tx_info.pool,
    //             false,
    //         )
    //         .await
    //         .unwrap();

    //     // put user's TX in the middle of the bundle
    //     match self
    //         .jito_client
    //         .send_bundle_no_wait(&vec![buy_tx.0, tx_to_sandwich.clone(), sell_tx])
    //         .await
    //     {
    //         Ok(signs) => Ok(signs.to_string()),
    //         Err(e) => Err(PumpSwapError::TransactionError(e.to_string())),
    //     }
    // }
}
