use std::sync::atomic::{AtomicU32, Ordering};

use crossbeam::channel::Sender;

use crate::ibkr::message::{Contract, IBKRMessageID, IBMessage, OptionSide};

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

pub fn request_option_market_data(
    symbol: impl Into<String>,
    expiry: impl Into<String>,
    strike: u32,
    right: OptionSide,
    tx: &Sender<IBMessage>,
) -> eyre::Result<u32> {
    let req_id = REQ_ID.fetch_add(1, Ordering::Relaxed);

    let contract = Contract::default()
        .with_symbol(symbol)
        .with_sec_type("OPT")
        .with_last_trade_date_or_contract_month(expiry)
        .with_strike(strike)
        .with_right(right)
        .with_exchange("SMART");

    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMktData)
        .with_version(11)
        .field(req_id.to_string())
        .contract(contract)
        .field("")
        .field("")
        .field("106")
        .field("0")
        .field("0")
        .field("");

    tx.send(msg)?;

    Ok(req_id)
}

pub fn request_spot_price(
    symbol: impl Into<String>,
    sec_type: impl Into<String>,
    exchange: impl Into<String>,
    tx: &Sender<IBMessage>,
) -> eyre::Result<()> {
    let req_id = REQ_ID.fetch_add(1, Ordering::Relaxed);

    let contract = Contract {
        symbol: symbol.into(),
        sec_type: sec_type.into(),
        exchange: exchange.into(),
        ..Default::default()
    };

    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMktData)
        .with_version(11)
        .field(req_id.to_string())
        .contract(contract)
        .field("0")
        .field("")
        .field("0")
        .field("0")
        .field("");

    tx.send(msg)?;

    Ok(())
}

pub fn request_delayed_market_data_type(tx: &Sender<IBMessage>) -> eyre::Result<()> {
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::ReqMarketDataType)
        .with_version(1)
        .field("3");

    tx.send(msg)?;

    Ok(())
}

pub fn cancel_market_data(tx: &Sender<IBMessage>, req_id: u32) -> eyre::Result<()> {
    let msg = IBMessage::default()
        .with_id(IBKRMessageID::CancelMktData)
        .with_version(2)
        .field(req_id.to_string());

    tx.send(msg)?;

    Ok(())
}
