# Lector PDF Minimalista

Un lector de PDF multiplataforma (Windows y Linux) enfocado en **bajo
consumo de recursos**, funciones esenciales bien hechas, y una estética
limpia inspirada en Foxit Reader.

Este código ya fue **compilado y probado** (ver "Qué se verificó" más
abajo) en un entorno Linux. Lo que falta es que lo compiles en tu propia
máquina y lo pruebes con interfaz gráfica real.

---

## Funcionalidades implementadas

- Abrir PDF: diálogo nativo (`Ctrl+O`), arrastrando el archivo a la ventana,
  o como argumento: `pdf-lector archivo.pdf` ("Abrir con..." del explorador)
- **Recordar progreso por documento** (página y zoom), también mientras
  lees haciendo scroll, y se guarda al cerrar. Cada PDF tiene el suyo.
- Búsqueda en todo el documento: lista de resultados con contexto, contador,
  siguiente/anterior (`Enter` / `Shift+Enter`), salto a la coincidencia y
  **resaltado** de las palabras encontradas sobre la página
- Ir a página escribiendo el número; el campo siempre muestra la página actual
- Zoom: botones, `Ctrl +` / `Ctrl -` / `Ctrl 0`, y **`Ctrl` + rueda** (centrado
  bajo el cursor)
- Desplazamiento **continuo** (solo se dibujan las páginas cercanas a lo visible)
  o **por página**; botón de la barra para alternar
- Mover el documento **manteniendo pulsado el botón central** del ratón
- **Índice/marcadores** del PDF en panel lateral (se oculta solo en ventanas
  estrechas) y **enlaces clicables** dentro de las páginas: saltos internos y
  URLs web (`http`, `https`, `mailto`; otros esquemas como `file://` se ignoran
  a propósito por seguridad)
- Pestaña de **Recientes** (no es la pantalla de inicio)
- Tema claro/oscuro
- Barra de herramientas **abajo**, con iconos vectoriales propios, que se
  **adapta al ancho**: al achicar la ventana pasa a un menú de opciones.
  El título del documento va en la barra de título de la ventana.

### Atajos

| Tecla | Acción |
|---|---|
| `→` `PgDn` `Espacio` | Página siguiente |
| `←` `PgUp` `Shift+Espacio` | Página anterior |
| `↑` `↓` | Desplazar un poco |
| `Inicio` / `Fin` | Primera / última página |
| `Ctrl+F` | Buscar (`Esc` cierra) |
| `Ctrl+O` | Abrir |
| `Ctrl +` `Ctrl -` `Ctrl 0` / `Ctrl`+rueda | Zoom |

## Lo que falta (próximas iteraciones)

- Ícono propio de la aplicación (`assets/icons/`)
- Empaquetado final (`.msi` / `.AppImage` / `.deb`)
- Retoques estéticos (pospuestos a propósito)
- Seleccionar y copiar texto del PDF

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
ahí. El código la busca primero junto al ejecutable, luego en la carpeta desde la
que lo lanzas, y por último como librería instalada en el sistema.

### 4. Compila

```bash
cargo build            # desarrollo (dependencias optimizadas: la primera vez tarda más)
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

## Qué se verificó

Probado en Linux con una ventana real (servidor gráfico virtual, teclado y
ratón simulados), con PDFs de 3, 12 y 300 páginas:

- Navegación secuencial con flechas/PgUp/PgDn/Inicio, ir a página, zoom
  (botones, atajos y Ctrl+rueda), paneo con botón central
- Progreso: al reabrir un PDF vuelve a su página y zoom; un PDF distinto
  empieza en la página 1
- Búsqueda con resaltado, enlaces internos (`/Dest` y `/A GoTo`), índice
  (incluidos marcadores que guardan su destino como acción)
- Barra responsiva y modo por página
- Rendimiento (build de desarrollo, render por software, el peor caso):
  300 páginas abren en ~2 s, CPU en reposo 0 %, memoria estable al navegar
  (~145-160 MB; no crece con el número de páginas)

**No se pudo probar aquí:** la compilación y ejecución en **Windows**, el
build `--release`, y la apertura real de URLs externas (depende del navegador
del sistema). Esas son las pruebas que tocan en tu máquina.

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
