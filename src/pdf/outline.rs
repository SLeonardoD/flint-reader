//! Extrae el árbol de marcadores (bookmarks / tabla de contenido) embebido
//! en el PDF, si el documento lo trae, para mostrarlo en el panel lateral.

use super::document::OutlineNode;
use pdfium_render::prelude::*;

pub fn build_outline(doc: &PdfDocument) -> Vec<OutlineNode> {
    let bookmarks = doc.bookmarks();
    let mut roots = Vec::new();

    if let Some(first) = bookmarks.root() {
        collect_siblings(&first, &mut roots);
    }

    tracing::info!("Índice del documento: {} entrada(s) de primer nivel", roots.len());
    roots
}

/// Página de destino de un marcador. Muchos PDFs guardan el destino en una
/// *acción* (`/A GoTo`) en vez de en `/Dest`, así que se prueban ambos.
fn bookmark_page(bookmark: &PdfBookmark) -> Option<usize> {
    if let Some(dest) = bookmark.destination() {
        if let Ok(page) = dest.page_index() {
            return Some(page as usize);
        }
    }

    let action = bookmark.action()?;
    let local = action.as_local_destination_action()?;
    let dest = local.destination().ok()?;
    dest.page_index().ok().map(|page| page as usize)
}

/// Recorre `node` y todos sus hermanos (`next_sibling`), añadiéndolos a `out`;
/// para cada uno, recurre en sus hijos (`first_child`) para armar el árbol.
fn collect_siblings(node: &PdfBookmark, out: &mut Vec<OutlineNode>) {
    let mut current = Some(node.clone());

    // Límite defensivo: algunos PDFs mal formados tienen ciclos en sus
    // marcadores; sin este límite el recorrido no terminaría nunca.
    let mut guard = 0usize;
    const MAX_SIBLINGS: usize = 5000;

    while let Some(bookmark) = current {
        guard += 1;
        if guard > MAX_SIBLINGS {
            break;
        }

        let title = bookmark
            .title()
            .unwrap_or_else(|| "(sin título)".to_string());
        let page_index = bookmark_page(&bookmark);

        let mut children = Vec::new();
        if let Some(child) = bookmark.first_child() {
            collect_siblings(&child, &mut children);
        }

        out.push(OutlineNode {
            title,
            page_index,
            children,
        });

        current = bookmark.next_sibling();
    }
}
