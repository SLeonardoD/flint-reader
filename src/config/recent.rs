//! Lista de documentos abiertos recientemente, con la posición de lectura
//! (página y zoom) que se les restaura automáticamente al reabrirlos.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Un documento que el usuario abrió antes, junto con su estado de lectura.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentDocument {
    pub path: PathBuf,
    pub last_page: usize,
    pub zoom: f32,
    /// Segundos desde epoch Unix, usado solo para ordenar/mostrar "hace cuánto".
    #[serde(default)]
    pub last_opened_epoch: u64,
}

impl RecentDocument {
    /// Nombre de archivo para mostrar en la lista (sin la ruta completa).
    pub fn title(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
    }

    /// Si el archivo ya no existe en disco (fue movido o borrado).
    pub fn is_missing(&self) -> bool {
        !self.path.exists()
    }
}

const MAX_RECENT: usize = 30;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentList {
    pub items: Vec<RecentDocument>,
}

impl RecentList {
    /// Inserta o actualiza la entrada de `path` y la mueve al frente de la lista.
    pub fn touch(&mut self, path: PathBuf, last_page: usize, zoom: f32) {
        self.items.retain(|r| r.path != path);
        self.items.insert(
            0,
            RecentDocument {
                path,
                last_page,
                zoom,
                last_opened_epoch: now_epoch_secs(),
            },
        );
        self.items.truncate(MAX_RECENT);
    }

    /// Actualiza solo página/zoom de una entrada ya existente, sin reordenar
    /// la lista (se usa mientras el usuario sigue leyendo el mismo documento).
    pub fn update_state(&mut self, path: &Path, last_page: usize, zoom: f32) {
        if let Some(entry) = self.items.iter_mut().find(|r| r.path == path) {
            entry.last_page = last_page;
            entry.zoom = zoom;
        }
    }

    pub fn find(&self, path: &Path) -> Option<&RecentDocument> {
        self.items.iter().find(|r| r.path == path)
    }

    pub fn remove(&mut self, path: &Path) {
        self.items.retain(|r| r.path != path);
    }
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
