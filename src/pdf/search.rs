//! Búsqueda de texto en el documento completo, página por página.
//!
//! Para la v1 del lector buscamos coincidencias a nivel de página (sin
//! resaltado pixel-perfecto sobre el render): es suficiente para "saltar"
//! a la página donde está la palabra buscada, que es el caso de uso
//! principal. El resaltado exacto sobre el canvas queda como mejora futura.

use pdfium_render::prelude::*;

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub page_index: usize,
    /// Cuántas veces aparece `query` en esta página.
    pub count: usize,
    /// Un fragmento de texto alrededor de la primera coincidencia, para
    /// mostrar contexto en la lista de resultados.
    pub snippet: String,
}

const SNIPPET_RADIUS: usize = 40;

pub fn search_document(doc: &PdfDocument, query: &str) -> Vec<SearchMatch> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();

    let mut results = Vec::new();

    for (page_index, page) in doc.pages().iter().enumerate() {
        let Ok(text_page) = page.text() else {
            continue;
        };
        let full_text = text_page.all();
        let haystack = full_text.to_lowercase();

        let count = haystack.matches(&needle).count();
        if count == 0 {
            continue;
        }

        let first_at = haystack.find(&needle).unwrap_or(0);
        let snippet = build_snippet(&full_text, first_at, needle.len());

        results.push(SearchMatch {
            page_index,
            count,
            snippet,
        });
    }

    results
}

fn build_snippet(text: &str, match_byte_index: usize, match_len: usize) -> String {
    let start = text
        .char_indices()
        .map(|(i, _)| i)
        .filter(|&i| i <= match_byte_index)
        .last()
        .unwrap_or(0)
        .saturating_sub(SNIPPET_RADIUS);

    let end = (match_byte_index + match_len + SNIPPET_RADIUS).min(text.len());

    // Aseguramos límites válidos en fronteras de carácter UTF-8.
    let start = floor_char_boundary(text, start);
    let end = ceil_char_boundary(text, end);

    let mut snippet = text[start..end].replace('\n', " ");
    if start > 0 {
        snippet = format!("…{snippet}");
    }
    if end < text.len() {
        snippet = format!("{snippet}…");
    }
    snippet
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}
