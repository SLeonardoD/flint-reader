# Empaquetado para Windows

Este directorio es para artefactos de empaquetado específicos de Windows
(por ejemplo, un script de `cargo-wix` si más adelante quieres generar un
instalador `.msi`).

La librería nativa de PDFium para Windows ya está lista en
`../../pdfium/windows-x64/pdfium.dll` — cópiala junto al `.exe` compilado,
no hace falta nada más para distribuir la app como una carpeta portable.
