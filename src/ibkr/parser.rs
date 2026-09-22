use std::collections::HashMap;

use chrono::NaiveDate;
use tracing::{info, instrument};

use crate::ibkr::parse_message;

#[derive(Debug, Clone, Default)]
pub struct IBData {
    pub handshake: Option<bool>,
    pub start_api: Option<bool>,
    pub tickers: HashMap<String, TickerData>,
    pub selected_ticker: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TickerData {
    pub con_id: u32,
    pub exchanges: Vec<String>,
    pub options_chain: Option<OptionsChain>,
}

#[derive(Debug, Clone, Default)]
pub struct OptionsChain {
    pub trading_class: String,
    pub expirations: Vec<NaiveDate>,
    pub strikes: Vec<u32>,
}

impl TickerData {
    fn with_con_id(self, con_id: u32) -> Self {
        Self { con_id, ..self }
    }
    fn with_exchanges(self, exchanges: Vec<String>) -> Self {
        Self { exchanges, ..self }
    }

    fn with_options_chain(self, options_chain: OptionsChain) -> Self {
        Self {
            options_chain: Some(options_chain),
            ..self
        }
    }
}

#[instrument(skip(raw_ib_response))]
pub fn parse_ib_bytes(raw_ib_response: Vec<u8>, data: &mut IBData) -> eyre::Result<()> {
    let res = parse_message(&raw_ib_response)?;
    info!(?res, "Raw IB Response");

    if res.is_empty() {
        return Ok(());
    }

    match res[0] {
        "10" => {
            let ticker = res[2];
            let con_id: u32 = res[10].parse()?;
            let valid_exchanges: Vec<String> = res[13].split(',').map(|s| s.to_string()).collect();
            info!(con_id, ?valid_exchanges, "Parsed contract details");

            data.tickers.insert(
                ticker.to_string(),
                TickerData::default()
                    .with_con_id(con_id)
                    .with_exchanges(valid_exchanges),
            );

            if data.selected_ticker.is_none() {
                data.selected_ticker = Some(ticker.to_string())
            }
        }
        "75" => {
            let trading_class = res[4].to_string();

            let num_expirations: usize = res[6].parse()?;
            let expirations: Vec<NaiveDate> = res[7..7 + num_expirations]
                .iter()
                .map(|s| NaiveDate::parse_from_str(s, "%Y%m%d"))
                .collect::<Result<_, _>>()?;

            let strikes_count_idx = 7 + num_expirations;
            let num_strikes: usize = res[strikes_count_idx].parse()?;

            let strikes_start = strikes_count_idx + 1;
            let strikes: Vec<u32> = res[strikes_start..strikes_start + num_strikes]
                .iter()
                .map(|s| s.parse::<f64>().map(|f| f as u32))
                .collect::<Result<_, _>>()?;

            let entry = data.tickers.entry(trading_class.to_string()).or_default();
            if entry.options_chain.is_none() {
                *entry = entry.clone().with_options_chain(OptionsChain {
                    trading_class,
                    expirations,
                    strikes,
                });
            }
        }
        _ => return Ok(()),
    }

    Ok(())
}
