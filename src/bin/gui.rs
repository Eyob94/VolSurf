use eframe::egui;
use volsurf::ui::App;

fn main() -> eyre::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_fullscreen(true),
        ..Default::default()
    };

    eframe::run_native("VolSurf", options, Box::new(|_| Ok(Box::<App>::default())))?;

    Ok(())
}
