# Pruebas

- `cargo test` ejecuta los tests unitarios (por ejemplo `src/pdf/text.rs`) y
  `tests/pdf_engine.rs`, que abre `tests/fixtures/sample.pdf` con PDFium real y
  comprueba páginas, índice, enlaces, búsqueda, texto seleccionable y render.
  Necesita la librería de PDFium en `pdfium/<plataforma>/` (ya incluida).
- `cargo run --example smoke_test -- archivo.pdf "palabra"` imprime lo que el
  motor ve en cualquier PDF y guarda `render_page0.png`: útil para depurar un
  documento concreto sin abrir la ventana.
- El `sample.pdf` tiene 12 páginas: índice con 11 enlaces internos (formatos
  `/Dest` y `/A GoTo`), 12 marcadores (el 3.º con su destino en una acción) y la
  palabra MANGOLIBRE en las páginas 2 a 12.
