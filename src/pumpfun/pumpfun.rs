use crate::pumpfun::constants::*;
use crate::pumpfun::error::{PumpFunError, PumpFunResult};
use crate::pumpfun::instruction_builder;

use crate::pumpswap::constants::PriorityFeePump;
use crate::pumpswap::pool::calculate_with_slippage_buy;
use crate::types::BuyAmounts;

use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::instruction::AccountMeta;

// use jito_searcher_client::swqos::{FeeClientTrait, JitoClient};
use log::info;

use solana_sdk::pubkey;
use solana_sdk::signer::Signer;
use spl_associated_token_account::instruction::create_associated_token_account;
use std::{str::FromStr, sync::Arc};

use rustls::crypto::{CryptoProvider, ring::default_provider};

use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, message::v0::Message as MessageV0,
    native_token::sol_to_lamports, pubkey::Pubkey, signature::Keypair, system_instruction,
    transaction::VersionedTransaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};

use solana_sdk::hash::Hash;

const ASTRALANE_TIP: Pubkey = pubkey!("astra4uejePWneqNaJKuFFA8oonqCE1sqF6b45kDMZm");
const ZEROBLOCK_FEE: Pubkey = pubkey!("7mMzAuqiZemFTrQoquVvJqv53Xb1L4pN38D1A5T2iMEZ");

#[derive(Clone)]
pub struct PumpFunSDK {
    pub client: Arc<RpcClient>,
    pub astralane_client: Arc<RpcClient>,
    // pub jito_client: Arc<JitoClient>,
    pub priority_fee: PriorityFeePump,
}

pub struct PumpFunBuyType {
    pub mint: Pubkey,
    pub sol_to_buy: f64,
    pub user_keypair: Keypair,
    pub blockhash: Hash,
    pub buy_amounts: BuyAmounts,
    pub slippage: u64,
    pub cu_price: u64,
    pub creator: Pubkey, // TODO: new pumpfun update
}

pub struct PumpFunSellType {
    pub mint: Pubkey,
    pub token_amount_lamports: u64,
    pub user_keypair: Keypair,
    pub blockhash: Hash,
    pub slippage: u64,
    pub cu_price: u64,
    pub creator: Pubkey, // TODO: new pumpfun update
}

impl PumpFunSDK {
    pub async fn new(rpc_url: &str, astralane_url: &str, priority_fee: PriorityFeePump) -> Self {
        if CryptoProvider::get_default().is_none() {
            default_provider().install_default().unwrap();
        }
        let client = Arc::new(RpcClient::new(String::from(rpc_url)));
        let astralane_client = Arc::new(RpcClient::new(astralane_url.to_string()));

        PumpFunSDK {
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

    pub async fn create_buy_tx(
        &self,
        params: PumpFunBuyType,
    ) -> PumpFunResult<(VersionedTransaction, u64)> {
        let sol_to_buy_lamports = sol_to_lamports(params.sol_to_buy);
        let user_pubkey = params.user_keypair.pubkey();

        let price: f64 = (params.buy_amounts.max_quote_amount_in as f64 / 1_000_000_000.0)
            / (params.buy_amounts.base_amount_out as f64 / 1_000_000.0);

        let bought_token_amount =
            (sol_to_buy_lamports as f64 * (price * 1_000_000.0) * 1_000_000.0) as u64;
        let sol_to_buy_after_slippage =
            calculate_with_slippage_buy(sol_to_buy_lamports, params.slippage);

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

        // Create ATA creation instruction for token being bought
        let create_token_ata_ix = create_associated_token_account(
            &user_pubkey,
            &user_pubkey,
            &params.mint,
            &PUMPFUN_TOKEN_PROGRAM,
        );

        let user_ata = get_associated_token_address(&user_pubkey, &params.mint);

        // Create the PumpFun Buy TX
        let pumpswap_buy_tx = instruction_builder::create_buy_instruction(
            &user_pubkey,
            &params.mint,
            &user_ata,
            bought_token_amount,
            sol_to_buy_after_slippage,
            &params.creator,
        )?;

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

        // let tip_acc_str = self.jito_client.get_tip_account().await.unwrap();
        // let jito_fee_ix = system_instruction::transfer(
        //     &user_pubkey,
        //     &Pubkey::from_str(&tip_acc_str).unwrap(),
        //     sol_to_lamports(self.priority_fee.jito_tip_fee),
        // );

        let astralane_fee_tx = system_instruction::transfer(
            &user_pubkey,
            &ASTRALANE_TIP,
            sol_to_lamports(self.priority_fee.jito_tip_fee),
        );

        let point_five_percent = (sol_to_buy_lamports as f64 * 0.005) as u64;
        let zeroblock_fee =
            system_instruction::transfer(&user_pubkey, &ZEROBLOCK_FEE, point_five_percent);

        // Combine instructions in correct order
        let instructions = vec![
            compute_unit_limit_ix,
            compute_unit_price_ix,
            // zeroblock_fee,
            create_token_ata_ix,
            pumpswap_buy_tx,
            astralane_fee_tx,
        ];

        // Create and sign transaction
        let message = MessageV0::try_compile(&user_pubkey, &instructions, &[], params.blockhash)
            .map_err(|e| PumpFunError::TransactionError(e.to_string()))?;

        let tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[params.user_keypair],
        )
        .map_err(|e| PumpFunError::TransactionError(e.to_string()))?;

        Ok((tx, bought_token_amount))
    }

    pub async fn create_sell_exact_amount_tx(
        &self,
        params: PumpFunSellType,
    ) -> PumpFunResult<VersionedTransaction> {
        info!(
            "🚀 SELL REQUEST: Mint {} - User {} - Amount {} lamports",
            params.mint,
            params.user_keypair.pubkey(),
            params.token_amount_lamports
        );

        let user = params.user_keypair.pubkey();
        // Note: No jito for now
        // let tip_acc_str = self.jito_client.get_tip_account().await.unwrap();
        // let jito_fee_ix = system_instruction::transfer(
        //     &params.user_keypair.pubkey(),
        //     &Pubkey::from_str(&tip_acc_str).unwrap(),
        //     sol_to_lamports(self.priority_fee.jito_tip_fee),
        // );

        let astralane_fee_tx = system_instruction::transfer(
            &user,
            &ASTRALANE_TIP,
            sol_to_lamports(self.priority_fee.jito_tip_fee),
        );

        // TODO: Add sell slippage support
        // let slippage_sell_amount_tokens = calculate_with_slippage_sell(params.token_amount_lamports, params.slippage * 100);
        // let slippage_sell_amount_sol =

        let user_ata: Pubkey = get_associated_token_address(&user, &params.mint);

        // Create sell instruction
        let pumpswap_sell_tx = instruction_builder::create_sell_instruction(
            &user,
            &params.mint,
            &user_ata,
            params.token_amount_lamports,
            0,
            &params.creator,
        )
        .unwrap();

        // Create compute budget instructions
        let compute_unit_limit_ix = ComputeBudgetInstruction::set_compute_unit_limit(150_000);
        let compute_unit_price_ix =
            ComputeBudgetInstruction::set_compute_unit_price(params.cu_price * 1_000_000);

        let zeroblock_fee =
            system_instruction::transfer(&user, &ZEROBLOCK_FEE, sol_to_lamports(0.0001));

        let close_ata = spl_token_2022::instruction::close_account(
            &PUMPFUN_TOKEN_PROGRAM,
            &user_ata,
            &user,
            &user,
            &[],
        )
        .unwrap();

        // Combine instructions
        let instructions = vec![
            compute_unit_limit_ix,
            compute_unit_price_ix,
            // zeroblock_fee,
            astralane_fee_tx,
            pumpswap_sell_tx,
            close_ata,
        ];

        // Create and sign transaction
        let message = MessageV0::try_compile(&user, &instructions, &[], params.blockhash)
            .map_err(|e| PumpFunError::TransactionError(e.to_string()))?;
        let tx = VersionedTransaction::try_new(
            solana_sdk::message::VersionedMessage::V0(message),
            &[params.user_keypair],
        )
        .map_err(|e| PumpFunError::TransactionError(e.to_string()))?;

        Ok(tx)
    }

    async fn place_tx(self, tx: VersionedTransaction) -> PumpFunResult<String> {
        // Do not use Jito GRPC for now.
        // let _ = match self
        //     .jito_client
        //     .send_bundle_no_wait(&vec![tx.clone()])
        //     .await
        // {
        //     Ok(signs) => Ok(signs.to_string()),
        //     Err(e) => Err(PumpFunError::TransactionError(e.to_string())),
        // };
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
            Err(e) => Err(PumpFunError::TransactionError(e.to_string())),
        }
    }

    pub async fn sell(
        self,
        mint: &Pubkey,
        token_amount: u64,
        user_keypair: Keypair,
        blockhash: Hash,
        creator: Pubkey,
    ) -> PumpFunResult<String> {
        let pump_sell_order = PumpFunSellType {
            mint: *mint,
            token_amount_lamports: token_amount,
            user_keypair,
            blockhash,
            slippage: 0,
            cu_price: 1,
            creator,
        };

        let tx = self
            .create_sell_exact_amount_tx(pump_sell_order)
            .await
            .unwrap();

        self.place_tx(tx).await
    }

    pub async fn buy(
        self,
        mint: &Pubkey,
        sol_to_buy: f64,
        user_keypair: Keypair,
        blockhash: Hash,
        buy_amounts: BuyAmounts,
        creator: Pubkey,
    ) -> PumpFunResult<(String, u64)> {
        let pump_buy_order = PumpFunBuyType {
            mint: *mint,
            sol_to_buy,
            user_keypair,
            blockhash,
            buy_amounts,
            slippage: 0,
            cu_price: 1,
            creator,
        };

        let tx = self.create_buy_tx(pump_buy_order).await.unwrap();

        let order_placement_res = match self.place_tx(tx.0).await {
            Ok(x) => x,
            Err(e) => return Err(e),
        };

        Ok((order_placement_res, tx.1))
    }
}
