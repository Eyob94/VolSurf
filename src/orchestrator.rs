use std::{sync::Arc, thread, time::Duration};

use crossbeam::channel::unbounded;
use parking_lot::RwLock;
use tokio::runtime::Runtime;
use tracing::{error, info};

use crate::{
    Config, IBConnector,
    ibkr::{IBData,  request_contract_details, request_options_chain},
    ui::UI,
};

pub struct Orchestrator {
    pub ib_connector: IBConnector,
    pub ui: UI,
    pub rt: Runtime,
}

impl Orchestrator {
    pub fn new(config: Config) -> eyre::Result<Self> {
        let (data_tx, data_rx) = unbounded();
        let (msg_tx, msg_rx) = unbounded();

        let rt = Runtime::new()?;
        let ib_connector = IBConnector::new(&config, data_tx, msg_rx, &rt)?;

        let data = Arc::new(RwLock::new(IBData::default()));

        let ui = UI {
            msg_tx: Some(msg_tx.clone()),
            ib_data: data.clone(),
            ..Default::default()
        };

        let dx = data.clone();
        thread::spawn(move || {
            loop {
                match data_rx.recv() {
                    Ok(d) => *dx.write() = d,
                    Err(e) => {
                        error!(?e, "Error receiving data object");
                        continue;
                    }
                };
            }
        });

        thread::spawn(move || -> eyre::Result<()> {
            loop {
                let data = data.read();
                if data.handshake.is_some_and(|h| h) && data.start_api.is_some_and(|s| s) {
                    info!("Sending contract detail request");
                    request_contract_details(&msg_tx)?;
                    break;
                }
                thread::sleep(Duration::from_secs(5));
            }

            loop {
                let data = data.read();
                let Some(selected_ticker) = &data.selected_ticker else {
                    continue;
                };
                let Some(ticker) = data.tickers.get(selected_ticker) else {
                    continue;
                };

                request_options_chain(selected_ticker, ticker.con_id, &msg_tx);
                break;
            }

            Ok(())
        });

        Ok(Self {
            ib_connector,
            rt,
            ui,
        })
    }

    pub fn join(self) {
        self.rt.spawn(async move {
            for handle in self.ib_connector.handles {
                if let Err(e) = handle.await {
                    error!(?e, "IB connector handle failed")
                }
            }
        });
    }
}
