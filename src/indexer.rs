use chrono::{DateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::{UiTransactionEncoding, EncodedConfirmedTransactionWithStatusMeta};
use solana_client::rpc_config::RpcSignaturesForAddressConfig;
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
            RpcSignaturesForAddressConfig {
                before: None,
                until: None,
                limit: Some(1000),
                commitment: Some(CommitmentConfig::confirmed()),
            },
        )
        .await?;

    let mut transfers = Vec::new();

    for sig_info in signatures {
        let signature = Signature::from_str(&sig_info.signature)?;
        let block_time = sig_info.block_time.ok_or("Missing block time ".to_string())?;

        let tx_time = Utc.timestamp_opt(block_time, 0).single().ok_or("Invalid timestamp ".to_string())?;
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
            if pre_balance.owner.as_ref() == Some(&wallet_pubkey.to_string())
                && pre_balance.mint.as_ref() == Some(&usdc_mint_pubkey.to_string())
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