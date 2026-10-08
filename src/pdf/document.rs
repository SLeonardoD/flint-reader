//! Envoltorio sobre `pdfium-render` que expone solo lo que la app necesita:
//! abrir un documento, conocer su número de páginas, renderizar una página
//! a una imagen que egui pueda mostrar, y leer sus dimensiones en puntos.

use anyhow::{anyhow, Context, Result};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// Motor PDF compartido por toda la aplicación. Se crea una sola vez al
/// iniciar el programa; la librería nativa de PDFium queda enlazada (vía
/// `libloading`, dinámicamente) durante toda la vida del proceso.
///
/// `Pdfium` se filtra deliberadamente a una referencia `'static` (vive para
/// siempre, nunca se libera) porque así cada `PdfDocument` que abrimos puede
/// tener lifetime `'static` también, lo que evita problemas de referencias
/// cruzadas al guardarlo dentro del estado de la app `App`.
pub struct PdfEngine {
    pdfium: &'static Pdfium,
}

impl PdfEngine {
    /// Intenta enlazar con PDFium: primero busca la librería nativa junto al
    /// ejecutable (`pdfium.dll` / `libpdfium.so`), y si no la encuentra,
    /// prueba con una instalación del sistema.
    pub fn new() -> Result<Self> {
        let bindings = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("."))
            .or_else(|_| Pdfium::bind_to_system_library())
            .map_err(|err| {
                anyhow!(
                    "No se pudo cargar la librería nativa de PDFium \
                     (pdfium.dll / libpdfium.so). Colócala junto al ejecutable. \
                     Detalle: {err}"
                )
            })?;

        let pdfium: &'static Pdfium = Box::leak(Box::new(Pdfium::new(bindings)));
        Ok(Self { pdfium })
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

        Ok(OpenDocument {
            path: path.to_path_buf(),
            title,
            doc,
        })
    }
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

/// Documento abierto en memoria, con acceso a sus páginas.
pub struct OpenDocument {
    pub path: PathBuf,
    pub title: String,
    doc: PdfDocument<'static>,
}

/// Cuántos píxeles renderizamos por punto PDF a zoom 1.0. 1.4 equivale
/// aproximadamente a 100 DPI, nítido en pantallas normales sin gastar
/// memoria de más; a mayor zoom se renderiza más grande proporcionalmente.
/// Pública porque la UI la necesita para calcular el tamaño en píxeles de
/// una página *antes* de haberla renderizado (para reservar su espacio en
/// el modo de scroll continuo).
pub const PIXELS_PER_POINT: f32 = 1.4;

impl OpenDocument {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn page_count(&self) -> usize {
        self.doc.pages().len() as usize
    }

    /// Dimensiones de una página en puntos PDF (sin aplicar zoom).
    pub fn page_size_points(&self, index: usize) -> Result<(f32, f32)> {
        let page = self
            .doc
            .pages()
            .get(index as u16)
            .map_err(|e| anyhow!("Página {index} fuera de rango: {e}"))?;
        Ok((page.width().value, page.height().value))
    }

    /// Renderiza una página a una imagen RGBA en memoria, al nivel de zoom dado.
    pub fn render_page(&self, index: usize, zoom: f32) -> Result<RenderedPage> {
        let page = self
            .doc
            .pages()
            .get(index as u16)
            .map_err(|e| anyhow!("Página {index} fuera de rango: {e}"))?;

        let width_points = page.width().value;

        let target_width =
            ((width_points * zoom.max(0.05) * PIXELS_PER_POINT).round() as i32).max(1);

        let config = PdfRenderConfig::new()
            .set_target_width(target_width)
            .set_maximum_height(target_width * 10) // margen generoso; el ancho ya fija la escala
            .rotate_if_landscape(PdfPageRenderRotation::None, false);

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
}
