# Flint Reader

Lector de PDF **minimalista y de bajo consumo**, para Windows y Linux. Hecho en
Rust con [`egui`](https://github.com/emilk/egui) y el motor
[PDFium](https://pdfium.googlesource.com/pdfium/) (el mismo de Chrome).

Nace de una idea sencilla: leer en un portátil sin cargar un lector con decenas
de funciones que no se usan. Flint Reader hace pocas cosas, las hace rápido y
no pesa: un solo ejecutable, sin navegador embebido, y casi 0 % de CPU en reposo.

Versión actual: ver [`VERSION`](VERSION). Historial: [`CHANGELOG.md`](CHANGELOG.md).

---

## Qué hace

- Abrir PDF (`Ctrl+O`, arrastrar y soltar, o `flint-reader archivo.pdf`)
- Recordar **página y zoom por documento**, y una pestaña de **Recientes**
- **Búsqueda** en todo el documento, con resaltado y salto al resultado
- **Seleccionar y copiar texto** (si el PDF lo permite)
- Ir a página, zoom (botones, atajos y `Ctrl`+rueda), barra de posición
- Desplazamiento **continuo** o **por página**
- **Índice** (marcadores) y **enlaces** del PDF
- Autoscroll con la rueda del ratón
- Tema claro/oscuro; la barra de herramientas se adapta al ancho de la ventana

Lo que **no** hace, a propósito: anotar, firmar, editar, convertir formatos,
sincronizar en la nube, pestañas múltiples.

## Uso

| Acción | Cómo |
|---|---|
| Página siguiente / anterior | `→` `PgDn` `Espacio` / `←` `PgUp` `Shift+Espacio` |
| Desplazar un poco | `↑` `↓` |
| Primera / última página | `Inicio` / `Fin` |
| Ir a una página | escribir el número en el cuadro de la barra y `Enter` |
| Buscar | `Ctrl+F`; `Enter` / `Shift+Enter` = siguiente / anterior; `Esc` cierra |
| Zoom | `Ctrl +` / `Ctrl -` / `Ctrl 0`, o `Ctrl`+rueda |
| Seleccionar texto | arrastrar; doble clic = palabra; `Esc` quita la selección |
| Copiar | `Ctrl+C` |
| Autoscroll | **un clic con la rueda**, mover el ratón (más lejos = más rápido); otro clic o `Esc` lo detiene |
| Abrir | `Ctrl+O` |

Los ajustes y la lista de recientes se guardan en `config.toml`
(`~/.config/flint-reader/` en Linux, `%APPDATA%\flint-reader\...` en Windows).
Si existe la configuración del nombre anterior del proyecto (`pdf-lector`), se
migra sola la primera vez.

---

## Compilar y ejecutar

### 1. Rust
Se necesita **Rust 1.85 o más nuevo**. Instálalo con [rustup](https://rustup.rs)
y no con el gestor de paquetes de la distro (suele traer versiones demasiado
viejas para las dependencias). En Windows también hacen falta las
*Build Tools de Visual Studio* con "Desarrollo de escritorio con C++".

### 2. Dependencias del sistema (solo Linux)
```bash
sudo apt install pkg-config libx11-dev libxkbcommon-dev libxkbcommon-x11-dev \
    libwayland-dev libxrandr-dev libxi-dev libxcursor-dev libgl1-mesa-dev \
    libatk1.0-dev libgtk-3-dev build-essential
```
(En Fedora/Arch/openSUSE son los equivalentes `-devel`.)

### 3. PDFium
La librería nativa ya viene en el repositorio, **no hay que copiar nada**:

```
pdfium/linux-x64/libpdfium.so
pdfium/windows-x64/pdfium.dll
```

El programa la busca solo, en este orden: la variable de entorno
`FLINT_PDFIUM_DIR`; junto al ejecutable; `pdfium/<plataforma>/` subiendo desde
el ejecutable (por eso funciona con `cargo run`); el directorio actual; la
carpeta del código fuente; y, por último, una instalación del sistema. Si no la
encuentra, el error lista todas las rutas probadas.

Para distribuir una carpeta portable: el ejecutable y la librería de la
plataforma juntos.

### 4. Ejecutar
```bash
cargo run                       # desarrollo (la primera vez tarda varios minutos)
cargo run -- archivo.pdf        # abrir un archivo directamente
cargo test                      # tests unitarios y de integración
RUST_LOG=info cargo run         # con registro de lo que hace (saltos de página, PDFium...)
```
`cargo build --release` genera el binario optimizado, pero según la
[política de versiones](#versionado) solo se hace en versiones que cambian el
primer número.

---

## Para quien quiera modificarlo

### Mapa del código

```
src/
├── main.rs            # llama a flint_reader::run()
├── lib.rs             # run(): logging, config, PDFium, ventana
├── app.rs             # App (estado raíz + ciclo de eventos) y DocumentState
├── config/            # config.toml: ajustes y recientes
│   ├── settings.rs    #   tema, modo de scroll, zoom por defecto
│   └── recent.rs      #   documentos recientes con página y zoom
├── pdf/               # ÚNICA capa que conoce pdfium-render
│   ├── document.rs    #   abrir, tamaños, render, enlaces, búsqueda, caracteres
│   ├── outline.rs     #   árbol de marcadores
│   ├── search.rs      #   búsqueda por página con fragmentos de contexto
│   └── text.rs        #   caracteres con caja: selección y copia
├── ui/                # componentes visuales (egui)
│   ├── viewer.rs      #   páginas, scroll, autoscroll, resaltado, enlaces
│   ├── selection.rs   #   lógica del ratón para seleccionar texto
│   ├── toolbar.rs     #   barra inferior responsiva
│   ├── search_bar.rs, sidebar.rs, recent_tab.rs
│   ├── icons.rs       #   iconos dibujados con trazos (sin fuentes)
│   └── theme.rs
├── input/shortcuts.rs # atajos de teclado -> acciones
└── utils.rs           # constantes de zoom, VERSION
examples/smoke_test.rs # el motor PDF sin ventana
tests/                 # test de integración y PDF de prueba
pdfium/                # librerías nativas por plataforma
packaging/             # .desktop de Linux, notas de Windows
```

### Cómo fluye un frame

`App::update` (en `app.rs`) hace, en orden: aplica el tema, procesa atajos y
archivos arrastrados, gestiona `Ctrl+C` y el zoom con la rueda, dibuja la barra
de herramientas y la de búsqueda, el panel de índice y por último el visor.

Regla de diseño: **los módulos de `ui/` dibujan y devuelven acciones**
(`ToolbarAction`, `SearchAction`, `RecentAction`...); es `App` quien las aplica
al estado. Así la lógica queda en un solo sitio. Las excepciones son los campos
puramente visuales que se pasan por referencia (el texto de un cuadro).

### Decisiones que conviene conocer

- **`Pdfium` se filtra a `&'static`** (`Box::leak`) una sola vez. Así cada
  documento abierto tiene lifetime `'static` y puede vivir dentro de
  `DocumentState` sin referencias cruzadas. Es un único objeto por proceso.
- **Scroll programático = `scroll_target`.** Cualquier cosa que quiera mover el
  documento (ir a página, flechas, zoom, restaurar posición, autoscroll) escribe
  en `DocumentState::scroll_target`; el visor lo aplica al crear el `ScrollArea`.
  Nunca se toca el scroll directamente.
- **Cada documento tiene su `id`**, usado como `id_source` del `ScrollArea`:
  así el scroll no se contagia de un PDF a otro.
- **Maquetación exacta.** En el modo continuo no hay espaciado automático:
  `page_top()` y el bucle del visor calculan las mismas posiciones. Tras un
  salto, el visor verifica la posición real de la página y se autocorrige.
- **Memoria acotada.** Solo se rasterizan las páginas cercanas a lo visible
  (con un máximo de 2 por frame), y las lejanas se liberan. Los tamaños de
  página se leen una vez al abrir.
- **Coordenadas.** PDF usa el origen abajo a la izquierda; todo el código de
  `pdf/` convierte a **puntos PDF con origen arriba a la izquierda**. Pantalla =
  `origen_página + punto * zoom * PIXELS_PER_POINT`.
- **Selección con "cursor entre caracteres"** (`caret`), como un editor de
  texto, y el estado del ratón leído en crudo (no con widgets) para poder
  arrastrar entre páginas.
- **Iconos vectoriales** en `ui/icons.rs`: se dibujan con el `painter`.

### Recetas

- **Añadir un atajo:** variante en `ShortcutAction` (`input/shortcuts.rs`), la
  tecla en `poll()`, y su caso en `App::apply_shortcut`.
- **Añadir un botón:** variante en `ToolbarAction`, el botón en `ui/toolbar.rs`
  (con un `Icon` de `icons.rs`), y su caso en `App::apply_toolbar_action`.
- **Añadir un ajuste que se recuerde:** campo con `#[serde(default)]` en
  `config/settings.rs`; así los `config.toml` antiguos siguen cargando.
- **Algo nuevo del motor PDF:** un método en `OpenDocument` (`pdf/document.rs`),
  re-exportado en `pdf/mod.rs`, y un caso en `tests/pdf_engine.rs`.

### Depurar
`cargo run --example smoke_test -- archivo.pdf "palabra"` muestra lo que el
motor ve en un PDF concreto. `RUST_LOG=info` (o `debug`) activa el registro.

---

## Versionado

`MAYOR.MENOR.PARCHE`, empezando en `0.1.0`. Las versiones que cambian
**MENOR** o **PARCHE** son iteraciones de desarrollo que se prueban con
`cargo run`; **solo las que cambian el primer número llegan a compilación de
distribución**. La versión se escribe en `Cargo.toml` y en `VERSION` (un test
vigila que coincidan), y cada cambio se anota en [`CHANGELOG.md`](CHANGELOG.md)
con su estado.

## Pendiente

Ícono propio, empaquetado final (`.msi` / `.AppImage` / `.deb`), retoques
estéticos, probar en Windows, triple clic y "seleccionar todo".

## Licencias

Flint Reader aún no tiene licencia definida. Las librerías de PDFium incluidas
en `pdfium/` son los binarios de
[bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) con
su licencia en cada carpeta.
