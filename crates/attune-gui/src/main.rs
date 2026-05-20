//! Attune GUI entry point.

use anyhow::Result;
use tracing_subscriber::EnvFilter;

mod app;
mod theme;

use app::AttuneApp;

fn main() -> Result<()> {
    init_tracing();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Attune")
            .with_inner_size([500.0, 720.0])
            .with_min_inner_size([460.0, 600.0])
            .with_resizable(true),
        wgpu_options: eframe::egui_wgpu::WgpuConfiguration::default(),
        ..Default::default()
    };

    eframe::run_native(
        "Attune",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(AttuneApp::new(cc)))
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
