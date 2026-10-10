//! Preferencias globales del lector: tema visual, modo de desplazamiento,
//! y valores por defecto para documentos sin estado guardado todavía.

use serde::{Deserialize, Serialize};

/// Preferencia de tema visual de la aplicación.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemePreference {
    Light,
    #[default]
    Dark,
}

impl ThemePreference {
    pub fn toggled(self) -> Self {
        match self {
            ThemePreference::Light => ThemePreference::Dark,
            ThemePreference::Dark => ThemePreference::Light,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemePreference::Light => "Claro",
            ThemePreference::Dark => "Oscuro",
        }
    }
}

/// Modo de desplazamiento del documento dentro del visor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScrollMode {
    /// Todas las páginas en una tira continua, como una página web larga.
    #[default]
    Continuous,
    /// Una página visible a la vez, como pasar hojas de un libro.
    Paginated,
}

impl ScrollMode {
    pub fn toggled(self) -> Self {
        match self {
            ScrollMode::Continuous => ScrollMode::Paginated,
            ScrollMode::Paginated => ScrollMode::Continuous,
        }
    }

}

fn default_zoom() -> f32 {
    1.0
}

fn default_sidebar_width() -> f32 {
    220.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub scroll_mode: ScrollMode,
    /// Zoom inicial para documentos que no tienen un zoom guardado aún.
    #[serde(default = "default_zoom")]
    pub default_zoom: f32,
    /// Ancho recordado del panel lateral de marcadores/recientes.
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default)]
    pub sidebar_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            scroll_mode: ScrollMode::default(),
            default_zoom: default_zoom(),
            sidebar_width: default_sidebar_width(),
            sidebar_open: false,
        }
    }
}
