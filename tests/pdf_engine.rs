//! Prueba de extremo a extremo del motor PDF con un documento real.
//!
//! `tests/fixtures/sample.pdf` tiene 12 páginas: la primera es un índice con
//! 11 enlaces internos (mezcla de `/Dest` y `/A GoTo`), cada marcador apunta a
//! su página (el 3.º lo hace mediante una acción), y en las páginas 2 a 12
//! aparece la palabra MANGOLIBRE.
//!
//! Es una sola función a propósito: PDFium se inicializa una vez por proceso.

use flint_reader::pdf::{LinkTarget, PdfEngine};
use std::path::Path;

#[test]
fn pdf_engine_end_to_end() {
    let engine = PdfEngine::new().expect("PDFium debe encontrarse en pdfium/<plataforma>/ del proyecto");
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sample.pdf");
    let doc = engine.open(&path).expect("el PDF de prueba debe abrirse");

    // Páginas y tamaños
    assert_eq!(doc.page_count(), 12);
    let (w, h) = doc.page_size_points(0);
    assert!((w - 595.0).abs() < 2.0 && (h - 842.0).abs() < 2.0, "A4: {w} x {h}");

    // Índice, incluido un marcador cuyo destino está en una acción
    let outline = doc.outline();
    assert_eq!(outline.len(), 12);
    assert_eq!(outline[0].title, "Seccion 1");
    assert_eq!(outline[1].page_index, Some(1));
    assert_eq!(outline[2].page_index, Some(9), "marcador con destino en una acción /A");

    // Enlaces internos (en ambos formatos) y su destino
    let links = doc.page_links(0);
    let targets: Vec<usize> = links
        .iter()
        .filter_map(|l| match l.target {
            LinkTarget::Page(p) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(targets.len(), 11, "enlaces internos de la página 1");
    assert!(targets.contains(&5) && targets.contains(&11));

    // Búsqueda: páginas con coincidencias y cajas de resaltado
    let results = doc.search("MANGOLIBRE");
    assert_eq!(results.len(), 11);
    assert_eq!(results[0].page_index, 1);
    assert!(!doc.search_rects(1, "MANGOLIBRE").is_empty());
    assert!(doc.search("palabra-que-no-existe").is_empty());

    // Texto seleccionable: caracteres con caja, y copia del texto
    let chars = doc.page_chars(1);
    assert!(!chars.is_empty());
    let text = chars.text_of(0, chars.len());
    assert!(text.contains("MANGOLIBRE"), "texto extraído: {text:?}");
    assert!(doc.page_chars(0).len() > 20);

    // Render
    let rendered = doc.render_page(0, 1.0).expect("render");
    assert!(rendered.image.size[0] > 500 && rendered.image.size[1] > 700);
}
