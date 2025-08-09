use actix_web::{web as actix_web_web, App, HttpServer};
use actix_web::web::{Data, get};
use chrono::{Duration, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use std::env;

mod indexer;
mod models;
mod web;

use indexer::index_usdc_transfers;
use web::get_transfers;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let rpc_url = env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let client = RpcClient::new(rpc_url);

    let wallet = "7cMEhpt9y3inBNVv8fNnuaEbx7hKHZnLvR1KWKKxuDDU".to_string();
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string();

    let end_time = Utc::now();
    let start_time = end_time - Duration::hours(24);

    let transfers = match index_usdc_transfers(&client, &wallet, &usdc_mint, start_time, end_time).await {
        Ok(transfers) => transfers,
        Err(err) => {
            eprintln!("Error indexing transfers: {}", err);
            vec![]
        }
    };

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(transfers.clone()))
            .route("/", get().to(get_transfers))
    })
    .bind(("0.0.0./indexer.rs`**
```rust
use chrono::{DateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use solana_client::rpc_config::GetConfirmedSignaturesForAddress2Config;
use chrono::TimeZone;
use std::str::FromStr;

use crate::models::{Transfer, TransferType};

pub async fn index_usdc_transfers(
    client: &RpcClient,
    wallet: &str,
    usdc_mint: &str,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<Vec<Transfer>, Box<dyn std::error::Error>> {
    let wallet_pubkey = Pubkey::from_str(wallet)?;
    let usdc_mint_pubkey = Pubkey::from_str(usdc_mint)?;

    let signatures = client
        .get_signatures_for_address_with_config(
            &wallet_pubkey,
            GetConfirmedSignaturesForAddress2Config {
                before: None,
                until: None,
                limit: Some(1000),
                commitment: Some(CommitmentConfig::confirmed()),
                min_context_slot: None,
            },
        )
        .await?;

    let mut transfers = Vec::new();

    for sig_info in signatures {
        let signature = Signature::from_str(&sig_info.signature)?;
        let block_time = sig_info.block_time.ok_or("Missing block time")?;

        let tx_time = Utc.timestamp_opt(block_time, 0).single().ok_or("Invalid timestamp")?;
        if tx_time < start_time || tx_time > end_time {
            continue;
        }

        let tx = client
            .get_transaction(&signature, UiTransactionEncoding::JsonParsed)
            .await?;
        transfers.extend(process_transaction(&tx, &wallet_pubkey, &usdc_mint_pubkey, tx_time, &signature));
    }

    Ok(transfers)
}

fn process_transaction(
    tx: &EncodedConfirmedTransactionWithStatusMeta,
    wallet_pubkey: &Pubkey,
    usdc_mint_pubkey: &Pubkey,
    tx_time: DateTime<Utc>,
    signature: &Signature,
) -> Vec<Transfer> {
    let mut transfers = Vec::new();

    if let Some(meta) = &tx.transaction.meta {
        for pre_balance in meta.pre_token_balances.as_ref().unwrap_or(&vec![]).iter() {
            if pre_balance.owner.as_ref().into() == Some(&wallet_pubkey.to_string())
                && pre_balance.mint.as_ref().into() == Some(&usdc_mint_pubkey.to_string())
            {
                transfers.push(Transfer {
                    date: tx_time,
                    amount: pre_balance.ui_token_amount.ui_amount.unwrap_or(0.0),
                    transfer_type: TransferType::Received,
                    signature: signature.to_string(),
                });
            }
        }
    }

    transfers
}