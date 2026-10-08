//! Prueba manual del módulo `pdf` sin necesidad de ventana gráfica.
//! Uso: cargo run --example smoke_test -- /ruta/al/archivo.pdf "palabra a buscar"

#[path = "../src/pdf/mod.rs"]
mod pdf;

use pdf::PdfEngine;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let pdf_path = args.get(1).cloned().unwrap_or_else(|| "test.pdf".to_string());
    let query = args.get(2).cloned().unwrap_or_else(|| "MANGOLIBRE".to_string());

    let engine = PdfEngine::new()?;
    println!("✓ PDFium enlazado correctamente");

    let doc = engine.open(Path::new(&pdf_path))?;
    println!("✓ Documento abierto: {}", doc.title());
    println!("  Páginas: {}", doc.page_count());

    for i in 0..doc.page_count() {
        let (w, h) = doc.page_size_points(i)?;
        println!("  Página {i}: {w:.1} x {h:.1} pts");
    }

    let outline = doc.outline();
    println!("✓ Índice/marcadores: {} nodo(s) raíz", outline.len());
    for item in &outline {
        println!("    - \"{}\" -> página {:?}", item.title, item.page_index);
    }

    let results = doc.search(&query);
    println!("✓ Búsqueda de \"{query}\": {} página(s) con coincidencias", results.len());
    for r in &results {
        println!("    - pág. {} ({} vez/veces): {}", r.page_index + 1, r.count, r.snippet);
    }

    let rendered = doc.render_page(0, 1.0)?;
    println!(
        "✓ Render de la página 0: {}x{} px",
        rendered.image.size[0], rendered.image.size[1]
    );
    save_png(&rendered.image, "render_page0.png")?;
    println!("✓ Guardado como render_page0.png");

    Ok(())
}

fn save_png(image: &egui::ColorImage, path: &str) -> anyhow::Result<()> {
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(path)?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, image.size[0] as u32, image.size[1] as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    let bytes: Vec<u8> = image
        .pixels
        .iter()
        .flat_map(|c| c.to_array())
        .collect();
    writer.write_image_data(&bytes)?;
    Ok(())
}
