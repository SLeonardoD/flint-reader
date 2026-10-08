//! Punto de entrada. Inicializa logging, carga la configuración persistida,
//! enlaza con PDFium y arranca la ventana principal.

mod app;
mod config;
mod input;
mod pdf;
mod ui;
mod utils;

use app::App;
use config::AppConfig;
use pdf::PdfEngine;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::load();

    let engine = match PdfEngine::new() {
        Ok(engine) => engine,
        Err(err) => {
            tracing::error!("{err}");
            eprintln!("\nError fatal: {err}\n");
            std::process::exit(1);
        }
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lector PDF")
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([480.0, 360.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "pdf-lector",
        native_options,
        Box::new(|_cc| Box::new(App::new(engine, config))),
    )
}
