//! Todo lo relacionado con abrir, renderizar, buscar e inspeccionar PDFs.
//! Es la única capa que conoce `pdfium-render`; el resto de la app solo
//! usa los tipos expuestos aquí.

mod document;
mod outline;
mod search;

pub use document::{
    LinkTarget, OpenDocument, OutlineNode, PageLink, PdfEngine, PIXELS_PER_POINT,
};
pub use search::SearchMatch;
