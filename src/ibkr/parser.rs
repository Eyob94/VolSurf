use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use crossbeam::channel::Sender;
use tracing::info;

use crate::ibkr::{
    IBMessage, cancel_market_data, message::OptionSide, parse_message, request_option_market_data,
};

#[derive(Debug, Clone, Default)]
pub struct IBData {
    pub handshake: Option<bool>,
    pub start_api: Option<bool>,
    pub tickers: HashMap<String, TickerData>,
    pub selected_ticker: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TickerData {
    pub ticker: String,
    pub con_id: u32,
    pub spot_price: u32,
    pub exchanges: Vec<String>,
    pub options_chain: Option<OptionsChain>,
    pub selected_expiry: Option<NaiveDate>,
    pub last_updated: DateTime<Utc>,
    pub subscribed_strikes: Vec<(u32, u32, bool)>, // (req_id, strike, active)
    pub iv_points: HashMap<u32, f64>,              // (strike, iv)
}

impl IBData {
    pub fn reconcile_strike_subs(&mut self, msg_tx: &Sender<IBMessage>) -> eyre::Result<()> {
        let Some(selected_ticker) = self.selected_ticker.clone() else {
            return Ok(());
        };

        if let Some(ticker_data) = self.tickers.get_mut(&selected_ticker) {
            info!(?ticker_data.subscribed_strikes, "SUBSCRIBED");
            info!(?ticker_data.selected_expiry, "Expiry");
            let Some(expiry) = ticker_data.selected_expiry else {
                return Ok(());
            };
            let spot_price = ticker_data.spot_price;
            for strike in ticker_data.subscribed_strikes.iter_mut() {
                if strike.2 && strike.0 == 0 {
                    info!("Sending IV request");
                    let req_id = request_option_market_data(
                        selected_ticker.clone(),
                        expiry.to_string(),
                        strike.1,
                        if strike.1 > spot_price {
                            OptionSide::Call
                        } else {
                            OptionSide::Put
                        },
                        msg_tx,
                    )?;
                    strike.0 = req_id;
                } else if !strike.2 {
                    cancel_market_data(msg_tx, strike.0)?;
                    strike.0 = 0;
                }
            }
            ticker_data.subscribed_strikes.retain(|s| s.2 || s.0 != 0);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct OptionsChain {
    pub trading_class: String,
    pub expirations: Vec<NaiveDate>,
    pub strikes: Vec<u32>,
}

impl TickerData {
    fn with_ticker(self, ticker: impl Into<String>) -> Self {
        Self {
            ticker: ticker.into(),
            last_updated: Utc::now(),
            ..self
        }
    }
    fn with_con_id(self, con_id: u32) -> Self {
        Self {
            con_id,
            last_updated: Utc::now(),
            ..self
        }
    }
    fn with_exchanges(self, exchanges: Vec<String>) -> Self {
        Self {
            exchanges,
            last_updated: Utc::now(),
            ..self
        }
    }

    fn with_options_chain(self, options_chain: OptionsChain) -> Self {
        Self {
            options_chain: Some(options_chain),
            last_updated: Utc::now(),
            ..self
        }
    }

    pub fn update_subscribed_strikes(&mut self, range_pct: f64, step: u32) {
        let Some(chain) = &self.options_chain else {
            return;
        };
        if self.spot_price == 0 {
            return;
        }

        let spot = self.spot_price as f64;
        let lower = spot * (1.0 - range_pct);
        let upper = spot * (1.0 + range_pct);

        let mut sorted_strikes: Vec<u32> = chain
            .strikes
            .iter()
            .filter(|&&s| ((s * 100) as f64) >= lower && ((s * 100) as f64) <= upper)
            .copied()
            .collect();
        sorted_strikes.sort();

        let wanted_strikes: Vec<u32> = sorted_strikes
            .into_iter()
            .step_by(step.max(1) as usize)
            .collect();

        for strikes in self.subscribed_strikes.iter_mut() {
            if !wanted_strikes.contains(&strikes.1) {
                strikes.2 = false
            }
        }
        for strike in wanted_strikes.iter() {
            if !self
                .subscribed_strikes
                .iter()
                .map(|(_, s, _)| s)
                .collect::<Vec<_>>()
                .contains(&strike)
            {
                self.subscribed_strikes.push((0, *strike, true));
            }
        }
    }
}

pub fn parse_ib_bytes(raw_ib_response: Vec<u8>, data: &mut IBData) -> eyre::Result<()> {
    let res = parse_message(&raw_ib_response)?;

    if res.is_empty() {
        return Ok(());
    }

    match res[0] {
        "1" => {
            let (_, info) = res.split_at(2);

            let tick_type: i32 = info[1].parse()?;
            let price: f64 = info[2].parse()?;

            let Some(ticker) = &data.selected_ticker else {
                return Ok(());
            };

            info!(price, "SPX price");
            if matches!(tick_type, 68)
                && price > 0.0
                && let Some(ticker_data) = data.tickers.get_mut(ticker)
            {
                ticker_data.spot_price = (price * 100.0) as u32;
            }
        }
        "10" => {
            let ticker = res[2];
            let con_id: u32 = res[10].parse()?;
            let valid_exchanges: Vec<String> = res[13].split(',').map(|s| s.to_string()).collect();

            data.tickers.insert(
                ticker.to_string(),
                TickerData::default()
                    .with_con_id(con_id)
                    .with_ticker(ticker)
                    .with_exchanges(valid_exchanges),
            );

            if data.selected_ticker.is_none() {
                data.selected_ticker = Some(ticker.to_string())
            }
        }

        // ["21", "22", "83", "0", "0.22921181020801557", "-1.0000000000001525", "31.608398437503524", "0.0", "-6.225082302474252E-14", "-1.4779288903810084E-12", "-3.524291969370097E-12", "767.4539794921875"]
        "21" => {
            let req_id: u32 = res[1].parse()?;
            let implied_vol: f64 = res[4].parse()?;

            if implied_vol < 0.0 {
                return Ok(());
            }

            let Some(selected_ticker) = data.selected_ticker.clone() else {
                return Ok(());
            };
            let Some(ticker_data) = data.tickers.get_mut(&selected_ticker) else {
                return Ok(());
            };

            let Some(&(_, strike, _)) = ticker_data
                .subscribed_strikes
                .iter()
                .find(|(id, _, _)| *id == req_id)
            else {
                return Ok(());
            };

            info!(strike, implied_vol, "SUBSCRIBED");

            ticker_data.iv_points.insert(strike, implied_vol);

            info!(?ticker_data.iv_points, "IV points");
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
