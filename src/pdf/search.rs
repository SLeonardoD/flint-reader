//! Búsqueda de texto por página. El conteo usa el buscador de PDFium (el
//! mismo que se usa luego para resaltar), así lista y resaltado coinciden.

use pdfium_render::prelude::*;

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub page_index: usize,
    pub count: usize,
    pub snippet: String,
}

const SNIPPET_RADIUS: usize = 40;

pub fn search_document(doc: &PdfDocument, query: &str) -> Vec<SearchMatch> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    let needle = query.to_lowercase();
    let options = PdfSearchOptions::new();
    let mut results = Vec::new();

    for (page_index, page) in doc.pages().iter().enumerate() {
        let Ok(text_page) = page.text() else { continue };
        let Ok(search) = text_page.search(query, &options) else { continue };

        let count = search.iter(PdfSearchDirection::SearchForward).count();
        if count == 0 {
            continue;
        }

        let full_text = text_page.all();
        let first_at = full_text.to_lowercase().find(&needle).unwrap_or(0);
        let snippet = build_snippet(&full_text, first_at, needle.len());

        results.push(SearchMatch { page_index, count, snippet });
    }
    results
}

fn build_snippet(text: &str, match_byte_index: usize, match_len: usize) -> String {
    let start = floor_char_boundary(text, match_byte_index.saturating_sub(SNIPPET_RADIUS));
    let end = ceil_char_boundary(text, (match_byte_index + match_len + SNIPPET_RADIUS).min(text.len()));

    let mut snippet = text[start..end].replace(['\n', '\r'], " ");
    if start > 0 {
        snippet = format!("…{snippet}");
    }
    if end < text.len() {
        snippet = format!("{snippet}…");
    }
    snippet
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
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
