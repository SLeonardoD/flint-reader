//! Persistencia de configuración y de la lista de recientes en un archivo
//! `config.toml`, ubicado en la carpeta estándar de configuración del
//! sistema operativo (vía el crate `directories`).

mod recent;
mod settings;

pub use recent::RecentList;
pub use settings::{ScrollMode, Settings, ThemePreference};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub recent: RecentList,
}

impl AppConfig {
    fn config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "flint-reader", "flint-reader")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Ruta de la configuración con el nombre anterior del proyecto (pdf-lector).
    fn legacy_config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "pdf-lector", "pdf-lector")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    /// Carga la configuración desde disco. Si no existe o está corrupta,
    /// devuelve valores por defecto en vez de bloquear el arranque de la app.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            tracing::warn!("No se pudo determinar la carpeta de configuración del sistema");
            return Self::default();
        };

        // Migración: si todavía no hay configuración de Flint Reader pero sí la del
        // nombre anterior, se lee esa (recientes y ajustes); al guardar pasa a la nueva.
        let source = if path.exists() {
            path
        } else {
            Self::legacy_config_path().filter(|p| p.exists()).unwrap_or(path)
        };

        match std::fs::read_to_string(&source) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_else(|err| {
                tracing::warn!("config.toml inválido ({err}); usando valores por defecto");
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Guarda la configuración actual a disco, creando la carpeta si hace falta.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()
            .ok_or_else(|| anyhow::anyhow!("no se pudo determinar la carpeta de configuración"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }
}
