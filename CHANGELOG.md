# Changelog

Registro de cambios de **Flint Reader**, y del estado en que queda cada versión.

## Convención de versiones

`MAYOR.MENOR.PARCHE` (por ejemplo `0.1.2`).

- **PARCHE** y **MENOR**: iteraciones de desarrollo. Se prueban con `cargo run`
  y **no se compilan para distribución**.
- **MAYOR** (el primer número): solo cuando cambia, la versión llega a
  compilación de distribución (`cargo build --release` y empaquetado).
- La versión vive en dos sitios que deben coincidir: `Cargo.toml` y el archivo
  `VERSION`. `cargo test` falla si se desincronizan.
- Cada entrada de este archivo tiene **Cambios** y **Estado** (qué funciona,
  qué está verificado y qué queda pendiente).

---

## [0.1.2]

Desarrollo en curso. Esta es la versión que sigue a las pruebas de 0.1.1.

### Cambios
- **Nombre:** el proyecto pasa a llamarse **Flint Reader** (crate `flint-reader`).
  El título de ventana es ahora `archivo.pdf - Flint Reader` (con guion ASCII).
  La configuración se guarda en la carpeta nueva y, si no existe, se **migra
  automáticamente** la de la versión anterior (ajustes y recientes).
- **Corregido:** no se podía escribir un número en el cuadro de página. El texto
  se descartaba en cada frame; ahora se conserva mientras se escribe.
- **Cambiado:** el desplazamiento con la rueda ahora es un **autoscroll**: un
  clic central lo activa y basta con mover el ratón (más lejos del punto de
  partida, más rápido). Otro clic, o `Esc`, lo desactiva. Ya no hay que
  mantener la rueda pulsada.
- **Nuevo:** **seleccionar y copiar texto** del PDF (si es seleccionable):
  arrastrar, doble clic para una palabra, arrastre que cruza páginas (con
  desplazamiento automático en los bordes) y `Ctrl+C`. `Esc` quita la
  selección. En páginas sin texto (escaneos) arrastrar mueve el documento.
- **Nuevo:** PDFium se busca solo en `pdfium/<plataforma>/` dentro del proyecto
  (también subiendo desde el ejecutable, y con la variable `FLINT_PDFIUM_DIR`).
  Ya no hay que copiarlo a la raíz.
- **Nuevo:** salto autocorregido al ir a una página en modo continuo: tras
  saltar se comprueba que la página quedó arriba y, si no, se reajusta.
  Diagnóstico con `RUST_LOG=info`.
- **Interno:** el código es ahora una biblioteca (`lib.rs`) más un `main.rs`
  mínimo; test de integración `tests/pdf_engine.rs`; `VERSION`; versión
  visible en la pantalla de inicio; README reescrito.
- **Interno:** corregidos los 5 warnings de `cargo check`/`clippy` reportados
  (imports sin usar, dos `impl Default` derivables, un `map_or`).

### Estado
- Verificado en Linux con ventana real (servidor gráfico virtual): escribir el
  número de página despacio, selección y copia (arrastre, inverso, doble clic,
  entre páginas), autoscroll con parada, título, búsqueda de PDFium en
  `pdfium/linux-x64/`, migración de configuración.
- `cargo test`: 6 tests pasan (5 unitarios + 1 de integración).
- **Pendiente de confirmar por el usuario:**
  - *Índice en modo continuo:* no se pudo reproducir (probado con 300 páginas,
    marcadores anidados, tamaños de página mixtos y saltos lejanos). Se añadió
    el salto autocorregido y un registro; si persiste, ejecutar con
    `RUST_LOG=info` y compartir las líneas "Ir a la página ...".
  - *Título "e" en la barra de ventana (Windows):* no reproducible en Linux.
  - Compilación y ejecución en Windows; `cargo clippy` con el código nuevo.
- Limitaciones conocidas: la selección asume páginas sin rotación; no hay
  triple clic ni "seleccionar todo".

---

## [0.1.1]

### Cambios
- **Corregido** (raíz común): en modo continuo, ir a una página cambiaba un
  número pero no movía el scroll, y el scroll sobrescribía la página actual.
  Arreglaba: cuadro de página, flechas, `PgUp/PgDn` y salto desde la búsqueda.
- **Corregido:** la posición se "contagiaba" entre documentos (el scroll era
  global); ahora cada documento tiene el suyo.
- **Corregido:** el progreso de lectura no se recuperaba; ahora se guarda
  mientras se lee y al cerrar.
- **Corregido:** los índices solo miraban `/Dest`; muchos PDFs guardan el
  destino en una acción `/A GoTo`.
- **Nuevo:** iconos vectoriales propios (las fuentes de egui no traían los
  símbolos y salían cuadrados).
- **Nuevo:** `Ctrl`+rueda para el zoom; desactivado el `Ctrl +/-` de egui que
  reescalaba toda la interfaz.
- **Nuevo:** resaltado de coincidencias de búsqueda, con salto al resultado;
  enlaces clicables del PDF (saltos internos y `http`/`https`/`mailto`).
- **Nuevo:** barra de herramientas abajo y responsiva (menú de opciones en
  ventanas estrechas); título del documento en la barra de la ventana;
  panel de índice que se oculta solo en ventanas estrechas.
- **Nuevo:** abrir un PDF pasado como argumento de línea de comandos.
- **Rendimiento:** tamaños de página en caché (antes se consultaban todos en
  cada frame), render con presupuesto por frame, texturas fuera de pantalla
  liberadas, dependencias optimizadas en desarrollo.

### Estado
- Probado por el usuario: la mayoría de los fallos del informe anterior
  desaparecen; navegación y sistema "mucho más ligeros"; búsqueda, zoom por
  rueda y por botones funcionan.
- Pendiente (pasó a 0.1.2): escribir en el cuadro de página, índice en continuo,
  comportamiento de la rueda, nombre del proyecto.
- Medido en Linux (build de desarrollo, render por software): 300 páginas
  abren en ~2 s, CPU en reposo 0 %, memoria estable (~145-160 MB).

---

## [0.1.0]

Primera salida.

### Cambios
- Abrir PDF (diálogo, arrastrar y soltar), recientes en una pestaña propia.
- Recordar página y zoom por documento.
- Búsqueda de texto a nivel de página, ir a página, contador, zoom, barra de
  posición.
- Scroll continuo y por página; panel de índice; tema claro/oscuro; atajos
  de teclado básicos.
- Motor: PDFium vía `pdfium-render`; interfaz: `egui`/`eframe`.

### Estado
- Compilaba y el motor se probó sin ventana. La primera prueba real con
  ventana (informe 1) encontró los fallos que corrige 0.1.1.
