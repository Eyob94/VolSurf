use eframe::egui;

#[derive(Debug, Default)]
pub struct App {}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {});
    }
}
