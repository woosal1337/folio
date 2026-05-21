//! Attune GUI entry point.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod app;
mod components;
mod design;
mod notes;
mod playback;
mod screens;
mod state;
mod tasks;
mod transcription;

use app::App;

fn main() -> Result<()> {
    init_tracing();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Attune")
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([820.0, 600.0])
            .with_resizable(true),
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };

    eframe::run_native(
        "Attune",
        options,
        Box::new(|cc| {
            design::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(cc)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe: {e}"))?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(
            "info,cpal=warn,wgpu_core=warn,wgpu_hal=warn,eframe=warn,egui=warn,egui_wgpu=warn",
        )
    });
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}
