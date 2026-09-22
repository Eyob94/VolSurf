use std::{collections::BTreeMap, sync::Arc};

use chrono::NaiveDate;
use crossbeam::channel::Sender;
use eframe::egui;
use parking_lot::RwLock;

use crate::ibkr::{IBData, IBMessage};

#[derive(Debug, Clone, Default)]
pub struct UI {
    pub surface: BTreeMap<NaiveDate, Vec<(f64, f64)>>,
    pub selected_expiry: Option<NaiveDate>,
    pub ib_data: Arc<RwLock<IBData>>,
    pub msg_tx: Option<Sender<IBMessage>>,
}

impl eframe::App for UI {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.heading("Vol Surf");
            egui::ComboBox::from_label("Expiry")
                .selected_text(self.selected_expiry.unwrap_or_default().to_string())
                .show_ui(ui, |ui| {
                    for expiry in self.surface.keys() {
                        ui.selectable_value(
                            &mut self.selected_expiry,
                            Some(*expiry),
                            expiry.to_string(),
                        );
                    }
                })
        });

        if let Some(expiry) = &self.selected_expiry
            && let Some(curve) = self.surface.get(expiry)
        {
            let plot_points: egui_plot::PlotPoints =
                curve.iter().map(|(strike, iv)| [*strike, *iv]).collect();

            egui_plot::Plot::new("skew_plot")
                .x_axis_label("Strike")
                .y_axis_label("Implied Vol")
                .show(ui, |plot_ui| {
                    plot_ui.line(egui_plot::Line::new("skew line", plot_points));
                });
        }
    }
}
