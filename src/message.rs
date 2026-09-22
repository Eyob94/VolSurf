use chrono::NaiveDate;

use crate::ibkr::IBMessage;

pub enum Message {
    IB(IBMessage),
    UpdateStrikes { range_pct: f64, step: u32 },
    UpdateExpiry(NaiveDate)
}
