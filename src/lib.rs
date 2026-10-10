//! Flint Reader: lector de PDF minimalista y de bajo consumo.
//!
//! Esta biblioteca contiene toda la aplicación; `main.rs` solo llama a [`run`].
//! Tenerla como biblioteca permite usar el motor PDF desde los ejemplos y los
//! tests de integración sin duplicar código.
//!
//! Mapa de módulos:
//! - [`pdf`]: única capa que conoce `pdfium-render` (abrir, renderizar, buscar, texto, enlaces).
//! - [`config`]: ajustes y lista de recientes, guardados en `config.toml`.
//! - [`ui`]: componentes visuales (egui); devuelven acciones, no mutan el estado.
//! - [`input`]: atajos de teclado.
//! - [`app`]: estado raíz y ciclo de eventos que lo conecta todo.

pub mod app;
pub mod config;
pub mod input;
pub mod pdf;
pub mod ui;
pub mod utils;

use app::App;
use config::AppConfig;
use pdf::PdfEngine;

/// Inicializa logging, carga la configuración, enlaza con PDFium y abre la ventana.
pub fn run() -> eframe::Result<()> {
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

    // `flint-reader archivo.pdf` abre ese archivo directamente.
    let initial_file = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .filter(|p| p.is_file());

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Flint Reader")
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([340.0, 300.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "Flint Reader",
        native_options,
        Box::new(|cc| {
            // Ctrl +/- y Ctrl+rueda controlan el zoom del *documento*, no el de toda la interfaz.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            Box::new(App::new(engine, config, initial_file))
        }),
    )
}
