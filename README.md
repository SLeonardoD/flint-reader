# Lector PDF Minimalista

Un lector de PDF multiplataforma (Windows y Linux) enfocado en **bajo
consumo de recursos**, funciones esenciales bien hechas, y una estética
limpia inspirada en Foxit Reader.

Este código ya fue **compilado y probado** (ver "Qué se verificó" más
abajo) en un entorno Linux. Lo que falta es que lo compiles en tu propia
máquina y lo pruebes con interfaz gráfica real.

---

## Funcionalidades implementadas

- Abrir PDF (diálogo nativo, o arrastrando el archivo a la ventana)
- Recordar página y zoom por documento, entre sesiones (`config.toml`)
- Búsqueda de texto en todo el documento, con lista de resultados y salto
  directo a la página
- Ir a página específica, contador actual/total
- Zoom (botones + atajos `Ctrl +` / `Ctrl -` / `Ctrl 0`)
- Modo de scroll **continuo** (virtualizado: solo renderiza las páginas
  cercanas a lo visible) y modo **paginado** (una página a la vez)
- Panel lateral con el índice/marcadores del PDF, si lo trae
- Pestaña de "Recientes" (no es la pantalla de inicio — se navega a ella)
- Tema claro/oscuro
- Atajos de teclado: flechas/espacio/Re Pág/Av Pág para navegar,
  `Ctrl+F` buscar, `Ctrl+O` abrir, `Esc` cerrar la barra de búsqueda

## Lo que falta (próximas iteraciones)

- Resaltado **pixel-perfecto** de las coincidencias de búsqueda sobre la
  página (hoy la búsqueda salta a la página correcta, pero no dibuja un
  rectángulo exacto sobre la palabra — ver nota en `src/pdf/search.rs`)
- Ícono propio de la aplicación (`assets/icons/`)
- Empaquetado final (`.msi` / `.AppImage` / `.deb`)

---

## Cómo compilar

### 1. Instala Rust

Necesitas una toolchain de Rust **relativamente reciente** (1.85 o más
nueva). Esto importa: durante el desarrollo, la versión de `rustc` que
trae por defecto el gestor de paquetes de una distro LTS (ej. Ubuntu
24.04 trae 1.75) resultó **demasiado vieja** para varias dependencias
transitivas modernas y dio errores de `edition2024`. La forma más segura
de evitarte este dolor de cabeza es instalar Rust con
[rustup](https://rustup.rs) en vez de con el gestor de paquetes del
sistema:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

En Windows, descarga e instala `rustup-init.exe` desde la misma página.

### 2. Dependencias de sistema (solo Linux)

```bash
sudo apt install pkg-config libx11-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libwayland-dev libxrandr-dev libxi-dev libxcursor-dev libgl1-mesa-dev \
    libatk1.0-dev libgtk-3-dev build-essential
```

(En Fedora/Arch/openSUSE son los paquetes `-devel`/equivalentes de esta
misma lista.) En Windows no hace falta nada adicional: `rustup` instala
el enlazador (MSVC) la primera vez que lo necesita.

### 3. Coloca la librería nativa de PDFium

El proyecto usa PDFium (el mismo motor que usa Chrome y que originalmente
viene de Foxit) a través de `pdfium-render`, que la carga **dinámicamente
en tiempo de ejecución** — no se compila, se copia.

Ya te dejé las dos que vas a necesitar en la carpeta `pdfium/` de este
mismo repo (descargadas de
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries),
licencia incluida en cada carpeta):

- `pdfium/linux-x64/libpdfium.so`
- `pdfium/windows-x64/pdfium.dll`

**Antes de compilar**, copia la que corresponda a la **raíz del
proyecto** (junto a `Cargo.toml`) y, cuando compiles, también junto al
ejecutable generado:

```bash
# Linux
cp pdfium/linux-x64/libpdfium.so .

# Windows (PowerShell)
copy pdfium\windows-x64\pdfium.dll .
```

Después de `cargo build`, cópiala también a `target/debug/` (o
`target/release/`) para que el ejecutable la encuentre al correr desde
ahí. El código busca primero junto al ejecutable y, si no la encuentra,
intenta con una instalación de PDFium ya presente en el sistema.

### 4. Compila

```bash
cargo build            # build de desarrollo, rápido
cargo build --release  # build optimizado (más lento de compilar: usa LTO)
cargo run               # compila y ejecuta de una
```

---

## Probar sin interfaz gráfica

Si quieres verificar que el motor de PDF funciona (abrir, renderizar,
extraer índice, buscar texto) sin necesidad de un entorno gráfico, usa el
ejemplo incluido:

```bash
cargo run --example smoke_test -- /ruta/a/tu.pdf "palabra a buscar"
```

Esto imprime en consola el número de páginas, el índice del documento,
los resultados de búsqueda, y guarda `render_page0.png` con la primera
página renderizada — ábrelo para confirmar visualmente que el render se
ve bien.

---

## Qué se verificó ya (en esta sesión de desarrollo)

- `cargo check` y `cargo build` (perfil de desarrollo) pasan **sin
  errores ni warnings**, incluido el enlazado contra X11/GTK/OpenGL.
- El motor PDF se probó de verdad: se descargó `libpdfium.so`, se generó
  un PDF de 3 páginas con marcadores, y se confirmó que la apertura, el
  conteo de páginas, la extracción de índice, la búsqueda de texto (con
  snippets correctos) y el renderizado a imagen funcionan — ver captura
  de ese render en la conversación donde se construyó este proyecto.
- Lo único que **no** se pudo probar en ese entorno es la ventana en sí
  (el contenedor de desarrollo no tenía servidor gráfico), así que la
  primera vez que vas a ver la interfaz real es cuando la corras tú.

---

## Estructura del proyecto

```
pdf-lector/
├── Cargo.toml
├── Cargo.lock              # versiones ya validadas — no lo borres a la ligera
├── pdfium/                 # librerías nativas de PDFium listas para usar
│   ├── linux-x64/libpdfium.so
│   └── windows-x64/pdfium.dll
├── assets/icons/           # ícono de la app (pendiente)
├── examples/
│   └── smoke_test.rs       # prueba del motor PDF sin GUI
├── packaging/
│   ├── windows/
│   └── linux/app.desktop
├── tests/                  # (ver tests/README.md)
└── src/
    ├── main.rs             # arranque: logging, config, PDFium, ventana
    ├── app.rs              # estado principal + ciclo de eventos de eframe
    ├── utils.rs
    ├── config/              # persistencia de config.toml y recientes
    │   ├── mod.rs
    │   ├── settings.rs
    │   └── recent.rs
    ├── pdf/                 # todo lo que toca pdfium-render
    │   ├── mod.rs
    │   ├── document.rs      # abrir/renderizar/medir páginas
    │   ├── outline.rs       # árbol de marcadores
    │   └── search.rs        # búsqueda de texto
    ├── ui/                  # todo lo visual (egui)
    │   ├── mod.rs
    │   ├── theme.rs
    │   ├── toolbar.rs
    │   ├── viewer.rs
    │   ├── sidebar.rs
    │   ├── recent_tab.rs
    │   └── search_bar.rs
    └── input/
        └── shortcuts.rs
```

## Notas sobre decisiones tomadas al compilar

El `Cargo.lock` incluido ya tiene fijadas algunas versiones puntuales de
dependencias transitivas (`proc-macro-crate`, `idna_adapter`, `home`,
`image`, `smithay-clipboard`, `indexmap`, `async-lock`, `libloading`,
`quick-xml`...) para que el proyecto compile también en toolchains algo
más antiguas. Si usas `rustup` con la versión estable más reciente (como
se recomienda arriba), no deberías necesitar tocar nada de esto; si en
el futuro quieres las versiones más nuevas de todo, `cargo update` las
traerá sin problema una vez que tu `rustc` sea reciente.

También se fijó `eframe` con las features `default_fonts`, `glow` y
`x11` (sin `wayland`): esto evita arrastrar `smithay-client-toolkit` y
toda su cadena de dependencias, y en la práctica no se nota — las
aplicaciones X11 corren igual de bien en un escritorio Wayland gracias a
XWayland, que está presente en todas las distros populares.
