use std::sync::{
    LazyLock,
    atomic::{AtomicU32, Ordering},
};

use crossbeam::channel::Sender;

use crate::ibkr::message::{Contract, IBKRMessageID, IBMessage};

static REQ_ID: AtomicU32 = AtomicU32::new(1);

pub fn request_contract_details(tx: &Sender<IBMessage>) -> eyre::Result<()> {
    let req_id = REQ_ID.fetch_add(1, Ordering::Relaxed);

    let contract = Contract::default()
        .with_symbol("SPY")
        .with_sec_type("STK")
        .with_exchange("");
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqContractDetails)
        .with_version(8)
        .field(req_id.to_string())
        .contract(contract)
        .field("0")
        .field("")
        .field("")
        .field("");

    tx.send(msg)?;

    Ok(())
}

pub fn request_options_chain(
    symbol: impl Into<String>,
    con_id: u32,
    tx: &Sender<IBMessage>,
) -> eyre::Result<()> {
    let req_id = REQ_ID.fetch_add(1, Ordering::Relaxed);

    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqSecDefOptParams)
        .field(req_id.to_string())
        .field(symbol)
        .field("")
        .field("STK")
        .field(con_id.to_string());

    tx.send(msg)?;

    Ok(())
}
