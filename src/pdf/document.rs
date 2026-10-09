//! Envoltorio sobre `pdfium-render` que expone solo lo que la app necesita:
//! abrir un documento, conocer el tamaño de sus páginas (en caché),
//! renderizar una página a una imagen que egui pueda mostrar, y consultar
//! enlaces, marcadores y coincidencias de búsqueda.

use anyhow::{anyhow, Context, Result};
use pdfium_render::prelude::*;
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
    /// Busca la librería nativa (`pdfium.dll` / `libpdfium.so`) primero junto
    /// al ejecutable, luego en el directorio de trabajo, y por último como
    /// librería instalada en el sistema.
    pub fn new() -> Result<Self> {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                dirs.push(dir.to_path_buf());
            }
        }
        dirs.push(PathBuf::from("."));

        for dir in &dirs {
            let lib = Pdfium::pdfium_platform_library_name_at_path(dir);
            if let Ok(bindings) = Pdfium::bind_to_library(lib) {
                return Ok(Self::from_bindings(bindings));
            }
        }

        match Pdfium::bind_to_system_library() {
            Ok(bindings) => Ok(Self::from_bindings(bindings)),
            Err(err) => Err(anyhow!(
                "No se pudo cargar la librería nativa de PDFium \
                 (pdfium.dll / libpdfium.so). Colócala junto al ejecutable \
                 o en la carpeta desde la que lo lanzas. Detalle: {err}"
            )),
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
