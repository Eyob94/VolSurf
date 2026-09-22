use std::sync::Arc;

use chrono::NaiveDate;
use crossbeam::channel::Sender;
use eframe::egui;
use parking_lot::RwLock;
use tracing::instrument;

use crate::{ibkr::IBData, message::Message};

#[derive(Debug, Clone)]
pub struct UI {
    pub selected_expiry: Option<NaiveDate>,
    pub ib_data: Arc<RwLock<IBData>>,
    pub ui_tx: Option<Sender<Message>>,

    pub strike_range_pct: f64,
    pub strike_step: u32,
}

impl Default for UI {
    fn default() -> Self {
        Self {
            selected_expiry: None,
            strike_range_pct: 0.1,
            strike_step: 5,
            ib_data: Arc::default(),
            ui_tx: None,
        }
    }
}

impl eframe::App for UI {
    #[instrument(skip_all)]
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let data = self.ib_data.read();

        let ticker_data = data
            .selected_ticker
            .as_ref()
            .and_then(|ticker| data.tickers.get(ticker));

        let options_chain = ticker_data.and_then(|t| t.options_chain.as_ref());
        let spot_price = ticker_data.map(|t| t.spot_price);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Vol Surf");

            let range_resp =
                ui.add(egui::Slider::new(&mut self.strike_range_pct, 0.02..=0.3).text("Range %"));
            let step_resp =
                ui.add(egui::Slider::new(&mut self.strike_step, 1..=20).text("Strike step"));

            if (range_resp.changed() || step_resp.changed())
                && let Some(tx) = &self.ui_tx
            {
                let _ = tx.send(Message::UpdateStrikes {
                    range_pct: self.strike_range_pct,
                    step: self.strike_step,
                });
            }

            match spot_price {
                Some(0) => {
                    ui.label("Spot: waiting...");
                }
                Some(price) => {
                    ui.label(format!("Spot: {:.2}", (price as f64) / 100.0));
                }
                None => {
                    ui.label("Spot: waiting...");
                }
            }

            let Some(chain) = options_chain else {
                ui.label("Waiting for option chain data...");
                return;
            };

            egui::ComboBox::from_label("Expiry")
                .selected_text(
                    self.selected_expiry
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                )
                .show_ui(ui, |ui| {
                    for expiry in &chain.expirations {
                        ui.selectable_value(
                            &mut self.selected_expiry,
                            Some(*expiry),
                            expiry.to_string(),
                        );
                    }
                });

            if self.selected_expiry.is_none() {
                let expiration = chain.expirations.first().copied();
                if let Some(exp) = expiration {
                    self.selected_expiry = expiration;
                    let ui_tx = self.ui_tx.as_ref().unwrap();
                    let _ = ui_tx.send(Message::UpdateExpiry(exp));
                }
            }

            let plot_points: egui_plot::PlotPoints = chain
                .strikes
                .iter()
                .map(|strike| [*strike as f64, 0.0])
                .collect();

            egui_plot::Plot::new("skew_plot")
                .x_axis_label("Strike")
                .y_axis_label("Implied Vol")
                .show(ui, |plot_ui| {
                    plot_ui.line(egui_plot::Line::new("skew line", plot_points));
                });
        });
    }
}
