use clap::Parser;
use eframe::egui;
use volsurf::{Config, orchestrator::Orchestrator, telemetry::init_tracing, ui::UI};

fn main() -> eyre::Result<()> {
    init_tracing();
    let config = Config::parse();

    let orchestrator = Orchestrator::new(config)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_fullscreen(false),
        ..Default::default()
    };
    let ui = orchestrator.ui.clone();

    eframe::run_native("VolSurf", options, Box::new(|_| Ok(Box::new(ui))))?;
    orchestrator.join();

    Ok(())
}
