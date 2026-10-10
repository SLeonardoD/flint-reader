//! Prueba del módulo `pdf` sin ventana gráfica.
//! Uso: cargo run --example smoke_test -- archivo.pdf "palabra"

use flint_reader::pdf::{LinkTarget, PdfEngine};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let pdf_path = args.get(1).cloned().unwrap_or_else(|| "test.pdf".into());
    let query = args.get(2).cloned().unwrap_or_else(|| "MANGOLIBRE".into());

    let engine = PdfEngine::new()?;
    println!("✓ PDFium enlazado");
    let doc = engine.open(Path::new(&pdf_path))?;
    println!("✓ Abierto: {} ({} páginas)", doc.title(), doc.page_count());

    for i in 0..doc.page_count().min(5) {
        let (w, h) = doc.page_size_points(i);
        println!("  pág {i}: {w:.0} x {h:.0} pts");
    }

    fn print_tree(nodes: &[flint_reader::pdf::OutlineNode], depth: usize) {
        for n in nodes {
            println!("    {}- \"{}\" -> {:?}", "  ".repeat(depth), n.title, n.page_index);
            print_tree(&n.children, depth + 1);
        }
    }
    let outline = doc.outline();
    println!("✓ Índice: {} nodo(s) raíz", outline.len());
    print_tree(&outline, 0);

    for i in 0..doc.page_count() {
        let links = doc.page_links(i);
        for l in &links {
            let t = match &l.target {
                LinkTarget::Page(p) => format!("-> página {}", p + 1),
                LinkTarget::Uri(u) => format!("-> URL {u}"),
            };
            println!("  enlace en pág {}: rect {:?} {t}", i + 1, l.rect.map(|v| v.round()));
        }
    }

    for p in 0..doc.page_count().min(2) {
        let links = doc.page_links(p);
        println!("✓ Enlaces en la página {}: {}", p + 1, links.len());
        for l in links.iter().take(14) {
            println!("    - rect {:?} -> {:?}", l.rect.map(|v| v.round()), l.target);
        }
    }

    let rects = doc.search_rects(1, &query);
    println!("✓ Rectángulos de resaltado en la pág. 2: {} -> {:?}", rects.len(), rects.first());

    let results = doc.search(&query);
    println!("✓ Búsqueda \"{query}\": {} página(s)", results.len());
    for r in &results {
        let rects = doc.search_rects(r.page_index, &query);
        println!("  pág {} ({}×) {} rect(s) de resaltado: {:?}", r.page_index + 1, r.count, rects.len(), rects.first().map(|v| v.map(|x| x.round())));
    }

    let t = std::time::Instant::now();
    let img = doc.render_page(0, 1.5)?.image;
    println!("✓ Render pág 0 a escala 1.5: {}x{} px en {:?}", img.size[0], img.size[1], t.elapsed());
    Ok(())
}
