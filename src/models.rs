use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub enum TransferType {
    Sent,
    Received,
}

#[derive(Debug, Serialize, Clone)]
pub struct Transfer {
    pub date: DateTime<Utc>,
    pub amount: f64,
    pub transfer_type: TransferType,
    pub signature: String,
}