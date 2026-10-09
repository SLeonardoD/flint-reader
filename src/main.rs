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

    // `pdf-lector archivo.pdf` abre ese archivo directamente.
    let initial_file = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_file());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Lector PDF")
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([340.0, 300.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "pdf-lector",
        native_options,
        Box::new(|cc| {
            // Ctrl +/- y Ctrl+rueda controlan el zoom del *documento*, no el de toda la interfaz.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            Box::new(App::new(engine, config, initial_file))
        }),
    )
}
