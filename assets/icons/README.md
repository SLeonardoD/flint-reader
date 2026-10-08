# Iconos de la aplicación

Coloca aquí:

- `app.ico` — ícono para Windows (multi-resolución: 16, 32, 48, 256 px).
- `app.png` — ícono para Linux (recomendado 256×256 o 512×512 px).

Estos archivos los referencia `build.rs` (Windows, vía `winresource`) y
`packaging/linux/app.desktop` (Linux, vía el campo `Icon=`).

Mientras no tengas un ícono propio, el ejecutable usará el ícono por
defecto del sistema operativo — la app funciona igual sin esto.
