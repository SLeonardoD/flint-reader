//! Envoltorio sobre `pdfium-render` que expone solo lo que la app necesita:
//! abrir un documento, conocer el tamaño de sus páginas (en caché),
//! renderizar una página a una imagen que egui pueda mostrar, y consultar
//! enlaces, marcadores y coincidencias de búsqueda.

use anyhow::{anyhow, Context, Result};
use pdfium_render::prelude::*;
use crate::pdf::text::PageChars;
use std::path::{Path, PathBuf};

/// Motor PDF compartido por toda la aplicación. Se crea una sola vez; la
/// librería nativa de PDFium queda enlazada (dinámicamente) durante toda la
/// vida del proceso.
///
/// `Pdfium` se filtra deliberadamente a una referencia `'static` para que
/// cada `PdfDocument` pueda tener lifetime `'static` y guardarse en el
/// estado de la app sin referencias cruzadas.
pub struct PdfEngine {
    pdfium: &'static Pdfium,
}

impl PdfEngine {
    /// Busca la librería nativa de PDFium (`pdfium.dll` / `libpdfium.so`).
    ///
    /// Orden: variable `FLINT_PDFIUM_DIR`; junto al ejecutable; la carpeta
    /// `pdfium/<plataforma>/` del proyecto (se prueba subiendo desde el
    /// ejecutable, así funciona con `cargo run` y con una carpeta portable);
    /// el directorio actual; la carpeta del código fuente (solo desarrollo);
    /// y por último una instalación del sistema.
    pub fn new() -> Result<Self> {
        let dirs = candidate_dirs();

        for dir in &dirs {
            let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
            if let Ok(bindings) = Pdfium::bind_to_library(&lib) {
                tracing::info!("PDFium cargado desde {}", lib.display());
                return Ok(Self::from_bindings(bindings));
            }
        }

        match Pdfium::bind_to_system_library() {
            Ok(bindings) => {
                tracing::info!("PDFium cargado desde el sistema");
                Ok(Self::from_bindings(bindings))
            }
            Err(err) => {
                let searched: Vec<String> = dirs.iter().map(|d| format!("  - {}", d.display())).collect();
                Err(anyhow!(
                    "No se encontró la librería nativa de PDFium ({}).\n\
                     Se buscó en:\n{}\n\
                     Colócala en pdfium/{}/ dentro del proyecto, junto al ejecutable, \
                     o indica su carpeta con la variable FLINT_PDFIUM_DIR.\n\
                     Detalle: {err}",
                    platform_library_name(),
                    searched.join("\n"),
                    platform_dir(),
                ))
            }
        }
    }

    fn from_bindings(bindings: Box<dyn PdfiumLibraryBindings>) -> Self {
        let pdfium: &'static Pdfium = Box::leak(Box::new(Pdfium::new(bindings)));
        Self { pdfium }
    }

    pub fn open(&self, path: &Path) -> Result<OpenDocument> {
        let doc = self
            .pdfium
            .load_pdf_from_file(path, None)
            .with_context(|| format!("No se pudo abrir el PDF: {}", path.display()))?;

        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());

        let page_sizes = read_page_sizes(&doc);

        Ok(OpenDocument {
            path: path.to_path_buf(),
            title,
            doc,
            page_sizes,
        })
    }
}

/// Subcarpeta de `pdfium/` que corresponde a esta plataforma.
fn platform_dir() -> &'static str {
    if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") { "windows-arm64" } else { "windows-x64" }
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") { "macos-arm64" } else { "macos-x64" }
    } else if cfg!(target_arch = "aarch64") {
        "linux-arm64"
    } else {
        "linux-x64"
    }
}

fn platform_library_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    }
}

/// Carpetas donde buscar PDFium, en orden de prioridad.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut add = |dir: PathBuf| {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    };

    if let Some(custom) = std::env::var_os("FLINT_PDFIUM_DIR") {
        add(PathBuf::from(custom));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            add(exe_dir.to_path_buf());
            // Desde el ejecutable hacia arriba: target/debug/ -> target/ -> proyecto/.
            for ancestor in exe_dir.ancestors().take(5) {
                add(ancestor.join("pdfium").join(platform_dir()));
                add(ancestor.join("pdfium"));
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        add(cwd.join("pdfium").join(platform_dir()));
        add(cwd);
    }

    // Ruta del código fuente en el momento de compilar: cubre `cargo run` y
    // `cargo test` aunque el ejecutable se haya movido.
    add(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pdfium").join(platform_dir()));

    dirs
}

/// Lee el tamaño (en puntos) de todas las páginas una sola vez. La UI lo
/// consulta en cada frame para maquetar, así que no puede costar una
/// llamada a pdfium por página y por frame.
fn read_page_sizes(doc: &PdfDocument<'static>) -> Vec<(f32, f32)> {
    if let Ok(rects) = doc.pages().page_sizes() {
        return rects
            .iter()
            .map(|r| (r.width().value, r.height().value))
            .collect();
    }
    doc.pages()
        .iter()
        .map(|p| (p.width().value, p.height().value))
        .collect()
}

/// Una imagen de página ya renderizada, lista para subir como textura de egui.
pub struct RenderedPage {
    pub image: egui::ColorImage,
}

/// Un nodo del índice/marcadores del documento, en forma de árbol.
pub struct OutlineNode {
    pub title: String,
    pub page_index: Option<usize>,
    pub children: Vec<OutlineNode>,
}

/// A dónde lleva un enlace dentro de una página.
#[derive(Debug, Clone)]
pub enum LinkTarget {
    Page(usize),
    Uri(String),
}

/// Un enlace clicable de una página. `rect` está en puntos PDF con el
/// origen arriba a la izquierda: `[x0, y0, x1, y1]`.
#[derive(Debug, Clone)]
pub struct PageLink {
    pub rect: [f32; 4],
    pub target: LinkTarget,
}

/// Documento abierto en memoria, con acceso a sus páginas.
pub struct OpenDocument {
    path: PathBuf,
    title: String,
    doc: PdfDocument<'static>,
    page_sizes: Vec<(f32, f32)>,
}

/// Píxeles de pantalla (puntos de egui) por punto PDF a zoom 1.0. 1.4 son
/// unos 100 DPI lógicos: tamaño cómodo de lectura a zoom 100 %.
/// Pública porque la UI la necesita para maquetar *antes* de renderizar.
pub const PIXELS_PER_POINT: f32 = 1.4;

/// Límites de la textura de una página, para no agotar memoria de vídeo
/// con zooms altos en pantallas HiDPI.
const MAX_TEXTURE_SIDE: f32 = 8192.0;
const MAX_TEXTURE_PIXELS: f32 = 25_000_000.0;

impl OpenDocument {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn page_count(&self) -> usize {
        self.page_sizes.len()
    }

    /// Tamaño de la página en puntos PDF (sin zoom). Viene de la caché.
    pub fn page_size_points(&self, index: usize) -> (f32, f32) {
        self.page_sizes.get(index).copied().unwrap_or((612.0, 792.0))
    }

    /// Renderiza una página. `scale` es el zoom multiplicado por los
    /// píxeles físicos por punto de egui, para que el resultado sea nítido
    /// también en pantallas con escalado (125 %, 150 %...).
    pub fn render_page(&self, index: usize, scale: f32) -> Result<RenderedPage> {
        let page = self
            .doc
            .pages()
            .get(index as u16)
            .map_err(|e| anyhow!("Página {index} fuera de rango: {e}"))?;

        let (w_pts, h_pts) = self.page_size_points(index);
        let mut width_px = w_pts * PIXELS_PER_POINT * scale.max(0.05);
        let mut height_px = h_pts * PIXELS_PER_POINT * scale.max(0.05);

        let side_factor = MAX_TEXTURE_SIDE / width_px.max(height_px);
        let pixel_factor = (MAX_TEXTURE_PIXELS / (width_px * height_px)).sqrt();
        let shrink = side_factor.min(pixel_factor).min(1.0);
        width_px *= shrink;
        height_px *= shrink;

        let config = PdfRenderConfig::new()
            .set_target_width(width_px.round().max(1.0) as i32)
            .set_maximum_height(height_px.round().max(1.0) as i32);

        let bitmap = page
            .render_with_config(&config)
            .map_err(|e| anyhow!("Fallo al renderizar la página {index}: {e}"))?;

        let width = bitmap.width() as usize;
        let height = bitmap.height() as usize;
        let rgba = bitmap.as_rgba_bytes();

        let image = egui::ColorImage::from_rgba_unmultiplied([width, height], &rgba);
        Ok(RenderedPage { image })
    }

    /// Construye el árbol de marcadores/índice del documento, si tiene.
    pub fn outline(&self) -> Vec<OutlineNode> {
        crate::pdf::outline::build_outline(&self.doc)
    }

    /// Busca `query` en todo el documento, página por página.
    pub fn search(&self, query: &str) -> Vec<crate::pdf::search::SearchMatch> {
        crate::pdf::search::search_document(&self.doc, query)
    }

    /// Rectángulos (puntos PDF, origen arriba-izquierda) de cada
    /// coincidencia de `query` en una página, para resaltarlos.
    pub fn search_rects(&self, index: usize, query: &str) -> Vec<[f32; 4]> {
        let mut out = Vec::new();
        let Ok(page) = self.doc.pages().get(index as u16) else {
            return out;
        };
        let page_h = page.height().value;
        let Ok(text) = page.text() else {
            return out;
        };
        let Ok(search) = text.search(query, &PdfSearchOptions::new()) else {
            return out;
        };

        // Tope defensivo por si una búsqueda degenerada produce miles de aciertos.
        let mut guard = 0;
        while let Some(segments) = search.find_next() {
            for segment in segments.iter() {
                let b = segment.bounds();
                out.push([
                    b.left().value,
                    page_h - b.top().value,
                    b.right().value,
                    page_h - b.bottom().value,
                ]);
            }
            guard += 1;
            if guard > 2000 {
                break;
            }
        }
        out
    }

    /// Caracteres de una página con su caja, para seleccionar y copiar texto.
    /// Devuelve un resultado vacío si la página no tiene texto seleccionable.
    pub fn page_chars(&self, index: usize) -> PageChars {
        let Ok(page) = self.doc.pages().get(index as u16) else {
            return PageChars::default();
        };
        let page_h = page.height().value;
        let Ok(text) = page.text() else {
            return PageChars::default();
        };

        let mut chars = Vec::new();
        let mut rects = Vec::new();
        let mut solid = Vec::new();

        for ch in text.chars().iter() {
            let c = ch.unicode_char().unwrap_or(' ');
            let generated = ch.is_generated().unwrap_or(false);
            let rect = ch.loose_bounds().ok().map(|b| {
                [
                    b.left().value,
                    page_h - b.top().value,
                    b.right().value,
                    page_h - b.bottom().value,
                ]
            });
            let usable = rect.is_some_and(|r| r[2] - r[0] > 0.01 && r[3] - r[1] > 0.01) && !generated;
            chars.push(c);
            rects.push(rect.unwrap_or([0.0; 4]));
            solid.push(usable);
        }

        PageChars::new(chars, rects, solid)
    }

    /// Enlaces clicables de una página (saltos internos y URLs).
    pub fn page_links(&self, index: usize) -> Vec<PageLink> {
        let mut out = Vec::new();
        let Ok(page) = self.doc.pages().get(index as u16) else {
            return out;
        };
        let page_h = page.height().value;

        for link in page.links().iter() {
            let Ok(rect) = link.rect() else { continue };
            let Some(target) = link_target(&link) else {
                continue;
            };
            out.push(PageLink {
                rect: [
                    rect.left().value,
                    page_h - rect.top().value,
                    rect.right().value,
                    page_h - rect.bottom().value,
                ],
                target,
            });
        }
        out
    }
}

/// Un enlace puede guardar su destino directamente (`/Dest`) o mediante una
/// acción (`/A`): hay que mirar ambos o se pierden muchos enlaces.
fn link_target(link: &PdfLink) -> Option<LinkTarget> {
    if let Some(dest) = link.destination() {
        if let Ok(page) = dest.page_index() {
            return Some(LinkTarget::Page(page as usize));
        }
    }

    let action = link.action()?;
    match action.action_type() {
        PdfActionType::GoToDestinationInSameDocument => {
            let local = action.as_local_destination_action()?;
            let dest = local.destination().ok()?;
            let page = dest.page_index().ok()?;
            Some(LinkTarget::Page(page as usize))
        }
        PdfActionType::Uri => {
            let uri = action.as_uri_action()?.uri().ok()?;
            // Solo enlaces web y de correo: abrir file:// u otros esquemas
            // desde un PDF desconocido sería un riesgo. Si no es seguro, ni
            // siquiera lo ofrecemos como enlace clicable.
            let lower = uri.to_lowercase();
            let safe = lower.starts_with("http://")
                || lower.starts_with("https://")
                || lower.starts_with("mailto:");
            safe.then_some(LinkTarget::Uri(uri))
        }
        _ => None,
    }
}
