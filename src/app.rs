//! `App`: estado raíz y ciclo de eventos. En cada frame procesa atajos,
//! dibuja los paneles y aplica las acciones que estos devuelven.

use crate::config::{AppConfig, RecentList, ScrollMode, ThemePreference};
use crate::input::shortcuts::{self, ShortcutAction};
use crate::pdf::{LinkTarget, OpenDocument, OutlineNode, PageChars, PageLink, PdfEngine, SearchMatch, PIXELS_PER_POINT};
use crate::ui::recent_tab::RecentAction;
use crate::ui::search_bar::{SearchAction, SearchBarState};
use crate::ui::toolbar::{ToolbarAction, ToolbarState};
use crate::ui::viewer::PAGE_GAP;
use crate::ui::{recent_tab, search_bar, sidebar, theme, toolbar, viewer};
use crate::utils::{clamp_zoom, SCROLL_STEP, ZOOM_FACTOR};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Reader,
    Recent,
}

/// Desplazamiento pendiente de aplicar al `ScrollArea` del visor.
#[derive(Default, Clone, Copy)]
pub struct ScrollTarget {
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// Posición de cursor dentro del texto del documento: página y hueco entre
/// caracteres (`0` = antes del primero).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TextPos {
    pub page: usize,
    pub caret: usize,
}

/// Texto seleccionado: del ancla (donde empezó el arrastre) a la cabeza (donde está el ratón).
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub anchor: TextPos,
    pub head: TextPos,
}

impl Selection {
    /// `(inicio, fin)` en orden de lectura, sea cual sea la dirección del arrastre.
    pub fn ordered(&self) -> (TextPos, TextPos) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragMode {
    /// Arrastre que selecciona texto.
    Select,
    /// La página no tiene texto: el arrastre mueve el documento.
    Pan,
}

/// Arrastre con el botón izquierdo en curso.
#[derive(Debug, Clone, Copy)]
pub struct SelDrag {
    pub start: egui::Pos2,
    pub mode: DragMode,
    pub moved: bool,
    pub anchor: TextPos,
}

/// Tras un cambio de zoom, las texturas viejas se siguen mostrando
/// (estiradas) hasta que pasa este tiempo sin cambios; luego se re-renderizan.
const ZOOM_SETTLE: Duration = Duration::from_millis(180);
/// Máximo de páginas que se rasterizan por frame (mantiene la UI fluida).
const RENDERS_PER_FRAME: u32 = 2;
const SAVE_EVERY: Duration = Duration::from_millis(1500);
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(350);

pub struct DocumentState {
    doc: OpenDocument,
    /// Identificador único: separa el estado de scroll de cada documento.
    pub id: u64,
    pub current_page: usize,
    pub zoom: f32,
    pub outline: Vec<OutlineNode>,

    // --- scroll ---
    pub scroll: egui::Vec2,
    pub scroll_target: ScrollTarget,
    pub viewer_rect: egui::Rect,

    // --- autoscroll con la rueda: un clic central lo activa, otro lo desactiva ---
    pub autoscroll_origin: Option<egui::Pos2>,
    /// Verdadero el frame en que un clic cierra el autoscroll: ese clic no debe abrir un enlace.
    pub ignore_click: bool,

    // --- selección de texto ---
    pub selection: Option<Selection>,
    pub sel_drag: Option<SelDrag>,
    /// Páginas dibujadas en este frame y su rectángulo en pantalla.
    pub page_rects: Vec<(usize, egui::Rect)>,
    pub hovering_link: bool,

    /// Salto pendiente de verificar: (página destino, reintentos que quedan).
    pub jump: Option<(usize, u8)>,

    // --- cachés ---
    textures: HashMap<usize, (f32, egui::TextureHandle)>,
    links: HashMap<usize, Rc<Vec<PageLink>>>,
    highlights: HashMap<usize, Rc<Vec<[f32; 4]>>>,
    chars: HashMap<usize, Rc<PageChars>>,
    highlight_query: String,
    no_highlights: Rc<Vec<[f32; 4]>>,
    last_zoom_change: Instant,
    pub renders_left: u32,

    pub pending_link: Option<LinkTarget>,
}

impl DocumentState {
    fn new(id: u64, doc: OpenDocument, current_page: usize, zoom: f32) -> Self {
        let outline = doc.outline();
        Self {
            doc,
            id,
            current_page,
            zoom,
            outline,
            scroll: egui::Vec2::ZERO,
            scroll_target: ScrollTarget::default(),
            viewer_rect: egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0)),
            autoscroll_origin: None,
            ignore_click: false,
            selection: None,
            sel_drag: None,
            page_rects: Vec::new(),
            hovering_link: false,
            jump: None,
            chars: HashMap::new(),
            textures: HashMap::new(),
            links: HashMap::new(),
            highlights: HashMap::new(),
            highlight_query: String::new(),
            no_highlights: Rc::new(Vec::new()),
            last_zoom_change: Instant::now() - Duration::from_secs(10),
            renders_left: RENDERS_PER_FRAME,
            pending_link: None,
        }
    }

    pub fn path(&self) -> &Path {
        self.doc.path()
    }

    pub fn title(&self) -> &str {
        self.doc.title()
    }

    pub fn page_count(&self) -> usize {
        self.doc.page_count()
    }

    pub fn page_size_points(&self, index: usize) -> (f32, f32) {
        self.doc.page_size_points(index)
    }

    /// Tamaño de la página en puntos de pantalla con el zoom actual.
    pub fn page_extent(&self, index: usize) -> egui::Vec2 {
        let (w, h) = self.doc.page_size_points(index);
        egui::vec2(w, h) * (self.zoom * PIXELS_PER_POINT)
    }

    /// Coordenada Y (en el contenido del scroll continuo) del borde superior de una página.
    pub fn page_top(&self, index: usize) -> f32 {
        let mut y = PAGE_GAP;
        for j in 0..index {
            y += self.page_extent(j).y + PAGE_GAP;
        }
        y
    }

    // ---------------------------------------------------------------- scroll

    /// Desplaza el visor para mostrar `page`; con `y_pts` (posición dentro de
    /// la página, en puntos PDF) deja ese punto a ~30% desde arriba.
    pub fn scroll_to_page(&mut self, page: usize, y_pts: Option<f32>, mode: ScrollMode) {
        let page = page.min(self.page_count().saturating_sub(1));
        self.current_page = page;
        let vh = self.viewer_rect.height().max(100.0);
        let px_per_pt = self.zoom * PIXELS_PER_POINT;

        let y = match (mode, y_pts) {
            (ScrollMode::Continuous, None) => self.page_top(page) - PAGE_GAP,
            (ScrollMode::Continuous, Some(yp)) => self.page_top(page) + yp * px_per_pt - vh * 0.3,
            (ScrollMode::Paginated, None) => 0.0,
            (ScrollMode::Paginated, Some(yp)) => yp * px_per_pt + PAGE_GAP - vh * 0.3,
        };
        self.scroll_target.y = Some(y.max(0.0));
        // Salto a "arriba de la página": el visor comprueba al maquetar que quedó
        // bien colocada y, si no, la corrige (ver `viewer::continuous`).
        self.jump = if mode == ScrollMode::Continuous && y_pts.is_none() { Some((page, 3)) } else { None };
        tracing::info!("Ir a la página {} ({:?}) -> desplazamiento y = {:.0}", page + 1, mode, y.max(0.0));
    }

    /// Desplazamiento relativo (flechas, paneo con el ratón).
    pub fn nudge_scroll(&mut self, dx: f32, dy: f32) {
        let base_x = self.scroll_target.x.unwrap_or(self.scroll.x);
        let base_y = self.scroll_target.y.unwrap_or(self.scroll.y);
        self.scroll_target.x = Some((base_x + dx).max(0.0));
        self.scroll_target.y = Some((base_y + dy).max(0.0));
    }

    /// Cambia el zoom manteniendo fijo el punto `anchor` (relativo al visor).
    pub fn zoom_to(&mut self, new_zoom: f32, anchor: egui::Vec2) {
        let new_zoom = clamp_zoom(new_zoom);
        if (new_zoom - self.zoom).abs() < 1e-4 {
            return;
        }
        let ratio = new_zoom / self.zoom;
        let base_x = self.scroll_target.x.unwrap_or(self.scroll.x);
        let base_y = self.scroll_target.y.unwrap_or(self.scroll.y);
        self.scroll_target.x = Some(((base_x + anchor.x) * ratio - anchor.x).max(0.0));
        self.scroll_target.y = Some(((base_y + anchor.y) * ratio - anchor.y).max(0.0));
        self.zoom = new_zoom;
        self.last_zoom_change = Instant::now();
    }

    // -------------------------------------------------------------- texturas

    /// Textura de la página. `force` ignora el límite de renders por frame.
    /// Si hay una textura vieja (otro zoom) se devuelve mientras tanto.
    pub fn texture_for(&mut self, ctx: &egui::Context, index: usize, force: bool) -> Option<egui::TextureHandle> {
        let scale = self.zoom * ctx.pixels_per_point();

        if let Some((s, tex)) = self.textures.get(&index) {
            if (s - scale).abs() < 1e-3 {
                return Some(tex.clone());
            }
            if self.last_zoom_change.elapsed() < ZOOM_SETTLE {
                ctx.request_repaint_after(ZOOM_SETTLE);
                return Some(tex.clone());
            }
            if !force && self.renders_left == 0 {
                ctx.request_repaint();
                return Some(tex.clone());
            }
        } else if !force && self.renders_left == 0 {
            ctx.request_repaint();
            return None;
        }

        self.renders_left = self.renders_left.saturating_sub(1);
        match self.doc.render_page(index, scale) {
            Ok(rendered) => {
                let tex = ctx.load_texture(format!("page-{index}"), rendered.image, egui::TextureOptions::LINEAR);
                self.textures.insert(index, (scale, tex.clone()));
                Some(tex)
            }
            Err(err) => {
                tracing::warn!("{err}");
                None
            }
        }
    }

    pub fn release_texture(&mut self, index: usize) {
        self.textures.remove(&index);
        self.chars.remove(&index);
    }

    pub fn keep_textures_near(&mut self, index: usize, radius: usize) {
        self.textures.retain(|&i, _| i.abs_diff(index) <= radius);
    }

    // ----------------------------------------------- enlaces y búsqueda

    pub fn links_for(&mut self, index: usize) -> Rc<Vec<PageLink>> {
        if let Some(l) = self.links.get(&index) {
            return l.clone();
        }
        let l = Rc::new(self.doc.page_links(index));
        self.links.insert(index, l.clone());
        l
    }

    pub fn set_highlight_query(&mut self, query: &str) {
        if self.highlight_query != query {
            self.highlight_query = query.to_string();
            self.highlights.clear();
        }
    }

    pub fn highlights_for(&mut self, index: usize) -> Rc<Vec<[f32; 4]>> {
        if self.highlight_query.is_empty() {
            return self.no_highlights.clone();
        }
        if let Some(h) = self.highlights.get(&index) {
            return h.clone();
        }
        let h = Rc::new(self.doc.search_rects(index, &self.highlight_query));
        self.highlights.insert(index, h.clone());
        h
    }

    /// Posición Y (puntos PDF) de la primera coincidencia en la página.
    pub fn first_match_y(&mut self, index: usize) -> Option<f32> {
        self.highlights_for(index).iter().map(|r| r[1]).reduce(f32::min)
    }

    pub fn search(&self, query: &str) -> Vec<SearchMatch> {
        self.doc.search(query)
    }

    // ----------------------------------------------- selección de texto

    pub fn chars_for(&mut self, index: usize) -> Rc<PageChars> {
        if let Some(c) = self.chars.get(&index) {
            return c.clone();
        }
        let c = Rc::new(self.doc.page_chars(index));
        self.chars.insert(index, c.clone());
        c
    }

    /// Página bajo `pos` (pantalla) y el punto en coordenadas de página (puntos PDF).
    pub fn page_at(&self, pos: egui::Pos2) -> Option<(usize, egui::Vec2)> {
        let (index, rect) = self.page_rects.iter().find(|(_, r)| r.contains(pos))?;
        let (w_pts, _) = self.doc.page_size_points(*index);
        let scale = rect.width() / w_pts.max(1.0);
        Some((*index, (pos - rect.min) / scale))
    }

    /// Posición de cursor de texto más cercana a `pos`, aunque el ratón
    /// esté fuera de las páginas (arrastre que se sale del texto).
    pub fn text_pos_at(&mut self, pos: egui::Pos2) -> Option<TextPos> {
        let (index, rect) = *self.page_rects.iter().min_by(|a, b| {
            a.1.distance_to_pos(pos)
                .partial_cmp(&b.1.distance_to_pos(pos))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        let chars = self.chars_for(index);
        let (w_pts, _) = self.doc.page_size_points(index);
        let scale = rect.width() / w_pts.max(1.0);
        let caret = if pos.y < rect.min.y {
            0
        } else if pos.y > rect.max.y {
            chars.len()
        } else {
            let p = (pos - rect.min) / scale;
            chars.caret_at(p.x, p.y).unwrap_or(0)
        };
        Some(TextPos { page: index, caret })
    }

    /// Texto de la selección actual (vacío si no hay).
    pub fn selection_text(&mut self) -> String {
        let Some(sel) = self.selection else {
            return String::new();
        };
        let (a, b) = sel.ordered();
        let last_page = b.page.min(self.page_count().saturating_sub(1));
        let mut parts = Vec::new();
        for page in a.page..=last_page {
            let chars = self.chars_for(page);
            let from = if page == a.page { a.caret } else { 0 };
            let to = if page == b.page { b.caret } else { chars.len() };
            let text = chars.text_of(from, to);
            if !text.is_empty() {
                parts.push(text);
            }
        }
        parts.join("\n")
    }
}

pub struct App {
    engine: PdfEngine,
    config: AppConfig,
    view: View,
    document: Option<DocumentState>,
    next_doc_id: u64,

    search_open: bool,
    search_query: String,
    search_results: Vec<SearchMatch>,
    search_selected: usize,
    search_request_focus: bool,
    search_pending: Option<Instant>,
    /// `false` hasta que el usuario salta al primer resultado de la búsqueda actual.
    search_visited: bool,

    /// Texto del cuadro "ir a página". Vive aquí (no se recrea cada frame) para poder escribir en él.
    goto_input: String,
    /// Cuándo se copió texto por última vez (para el aviso "Texto copiado").
    copied_at: Option<Instant>,

    status: Option<String>,
    theme_applied: Option<ThemePreference>,
    last_title: String,

    // Guardado del progreso de lectura (con retardo, para no escribir a disco en cada frame).
    last_tracked: Option<(usize, f32)>,
    dirty: bool,
    last_save: Instant,
}

impl App {
    /// `initial_file`: PDF a abrir al arrancar (argumento de línea de comandos,
    /// o "Abrir con..." desde el explorador de archivos).
    pub fn new(engine: PdfEngine, config: AppConfig, initial_file: Option<PathBuf>) -> Self {
        let mut app = Self {
            engine,
            config,
            view: View::Reader,
            document: None,
            next_doc_id: 1,
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_request_focus: false,
            search_pending: None,
            search_visited: false,
            status: None,
            theme_applied: None,
            last_title: String::new(),
            last_tracked: None,
            goto_input: String::new(),
            copied_at: None,
            dirty: false,
            last_save: Instant::now(),
        };
        if let Some(path) = initial_file {
            app.open_path(path);
        }
        app
    }

    // ------------------------------------------------------------ documentos

    fn open_path(&mut self, path: PathBuf) {
        match self.engine.open(&path) {
            Ok(doc) => {
                // Posición guardada de ESTE documento (o inicio si es nuevo).
                let (page, zoom) = self
                    .config
                    .recent
                    .find(&path)
                    .map(|r| (r.last_page, r.zoom))
                    .unwrap_or((0, self.config.settings.default_zoom));

                let page = page.min(doc.page_count().max(1) - 1);
                let id = self.next_doc_id;
                self.next_doc_id += 1;

                let mut state = DocumentState::new(id, doc, page, clamp_zoom(zoom));
                state.scroll_to_page(page, None, self.config.settings.scroll_mode);
                tracing::info!(
                    "Abierto {} ({} págs, {} marcadores raíz)",
                    state.title(),
                    state.page_count(),
                    state.outline.len()
                );

                self.config.recent.touch(path, state.current_page, state.zoom);
                self.document = Some(state);
                self.last_tracked = None;
                self.dirty = true;

                self.view = View::Reader;
                self.search_open = false;
                self.search_results.clear();
                self.search_pending = None;
                self.status = None;
            }
            Err(err) => {
                self.status = Some(format!("No se pudo abrir el archivo: {err}"));
            }
        }
    }

    fn pick_and_open(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("PDF", &["pdf"]).pick_file() {
            self.open_path(path);
        }
    }

    // ------------------------------------------------------------ navegación

    fn go_to_page(&mut self, page: usize) {
        let mode = self.config.settings.scroll_mode;
        if let Some(doc) = &mut self.document {
            doc.scroll_to_page(page, None, mode);
        }
    }

    fn go_to_relative(&mut self, delta: isize) {
        let mode = self.config.settings.scroll_mode;
        if let Some(doc) = &mut self.document {
            let last = doc.page_count().max(1) as isize - 1;
            let target = (doc.current_page as isize + delta).clamp(0, last) as usize;
            doc.scroll_to_page(target, None, mode);
        }
    }

    fn zoom_by(&mut self, factor: f32) {
        if let Some(doc) = &mut self.document {
            let anchor = doc.viewer_rect.size() / 2.0;
            let target = doc.zoom * factor;
            doc.zoom_to(target, anchor);
        }
    }

    fn set_zoom(&mut self, zoom: f32) {
        if let Some(doc) = &mut self.document {
            let anchor = doc.viewer_rect.size() / 2.0;
            doc.zoom_to(zoom, anchor);
        }
    }

    fn follow_link(&mut self, target: LinkTarget) {
        match target {
            LinkTarget::Page(page) => self.go_to_page(page),
            LinkTarget::Uri(uri) => {
                let lower = uri.to_lowercase();
                let safe = lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("mailto:");
                if safe {
                    if let Err(err) = open::that(&uri) {
                        self.status = Some(format!("No se pudo abrir el enlace: {err}"));
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------- búsqueda

    fn toggle_search(&mut self) {
        self.search_open = !self.search_open;
        self.search_request_focus = self.search_open;
        if !self.search_open {
            self.search_results.clear();
            self.search_pending = None;
            if let Some(doc) = &mut self.document {
                doc.set_highlight_query("");
            }
        }
    }

    fn run_search(&mut self) {
        self.search_pending = None;
        let query = self.search_query.trim().to_string();
        if let Some(doc) = &mut self.document {
            self.search_results = doc.search(&query);
            self.search_selected = 0;
            self.search_visited = false;
            doc.set_highlight_query(&query);
        }
    }

    fn jump_to_result(&mut self, index: usize) {
        let Some(result) = self.search_results.get(index) else { return };
        let page = result.page_index;
        self.search_selected = index;
        self.search_visited = true;
        let mode = self.config.settings.scroll_mode;
        if let Some(doc) = &mut self.document {
            let y = doc.first_match_y(page);
            doc.scroll_to_page(page, y, mode);
        }
    }

    fn step_result(&mut self, backwards: bool) {
        let n = self.search_results.len();
        if n == 0 {
            return;
        }
        // Primera pulsación tras buscar: ir al primer (o último) resultado, no saltárselo.
        if !self.search_visited {
            self.jump_to_result(if backwards { n - 1 } else { 0 });
            return;
        }
        let next = if backwards { (self.search_selected + n - 1) % n } else { (self.search_selected + 1) % n };
        self.jump_to_result(next);
    }

    // ---------------------------------------------------------- acciones UI

    fn apply_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::OpenFile => self.pick_and_open(),
            ToolbarAction::PrevPage => self.go_to_relative(-1),
            ToolbarAction::NextPage => self.go_to_relative(1),
            ToolbarAction::GotoPage(p) => self.go_to_page(p),
            ToolbarAction::ZoomIn => self.zoom_by(ZOOM_FACTOR),
            ToolbarAction::ZoomOut => self.zoom_by(1.0 / ZOOM_FACTOR),
            ToolbarAction::ZoomReset => self.set_zoom(1.0),
            ToolbarAction::ToggleSearch => self.toggle_search(),
            ToolbarAction::ToggleTheme => {
                self.config.settings.theme = self.config.settings.theme.toggled();
                self.dirty = true;
            }
            ToolbarAction::ToggleScrollMode => {
                self.config.settings.scroll_mode = self.config.settings.scroll_mode.toggled();
                self.dirty = true;
                // Mantener la página actual al cambiar de modo.
                if let Some(page) = self.document.as_ref().map(|d| d.current_page) {
                    self.go_to_page(page);
                }
            }
            ToolbarAction::ToggleSidebar => {
                self.config.settings.sidebar_open = !self.config.settings.sidebar_open;
                self.dirty = true;
            }
            ToolbarAction::ShowRecent => self.view = View::Recent,
            ToolbarAction::ShowReader => self.view = View::Reader,
        }
    }

    fn copy_selection(&mut self, ctx: &egui::Context) {
        if let Some(doc) = &mut self.document {
            let text = doc.selection_text();
            if !text.is_empty() {
                ctx.copy_text(text);
                self.copied_at = Some(Instant::now());
            }
        }
    }

    fn apply_shortcut(&mut self, action: ShortcutAction) {
        match action {
            ShortcutAction::NextPage => self.go_to_relative(1),
            ShortcutAction::PrevPage => self.go_to_relative(-1),
            ShortcutAction::FirstPage => self.go_to_page(0),
            ShortcutAction::LastPage => self.go_to_page(usize::MAX),
            ShortcutAction::ScrollUp => {
                if let Some(doc) = &mut self.document {
                    doc.nudge_scroll(0.0, -SCROLL_STEP);
                }
            }
            ShortcutAction::ScrollDown => {
                if let Some(doc) = &mut self.document {
                    doc.nudge_scroll(0.0, SCROLL_STEP);
                }
            }
            ShortcutAction::ZoomIn => self.zoom_by(ZOOM_FACTOR),
            ShortcutAction::ZoomOut => self.zoom_by(1.0 / ZOOM_FACTOR),
            ShortcutAction::ZoomReset => self.set_zoom(1.0),
            ShortcutAction::ToggleSearch => self.toggle_search(),
            ShortcutAction::OpenFile => self.pick_and_open(),
            ShortcutAction::CloseOverlay => {
                if self.search_open {
                    self.toggle_search();
                } else if let Some(doc) = &mut self.document {
                    doc.selection = None;
                }
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = file.path {
                let is_pdf = path.extension().map(|e| e.eq_ignore_ascii_case("pdf")).unwrap_or(false);
                if is_pdf {
                    self.open_path(path);
                    break;
                }
            }
        }
    }

    /// Ctrl + rueda (o pellizco en trackpad): zoom centrado en el cursor.
    fn handle_wheel_zoom(&mut self, ctx: &egui::Context) {
        if self.view != View::Reader {
            return;
        }
        let zoom_delta = ctx.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() < 1e-4 {
            return;
        }
        let hover = ctx.input(|i| i.pointer.hover_pos());
        if let (Some(doc), Some(pos)) = (&mut self.document, hover) {
            if doc.viewer_rect.contains(pos) {
                let anchor = pos - doc.viewer_rect.min;
                // Suavizado: una muesca de rueda sube ~28 %, demasiado brusco para leer.
                let target = doc.zoom * zoom_delta.powf(0.75);
                doc.zoom_to(target, anchor);
            }
        }
    }

    // ------------------------------------------------- progreso y ventana

    /// Anota página/zoom actuales y los escribe a disco con retardo.
    fn track_progress(&mut self, ctx: &egui::Context) {
        if let Some(doc) = &self.document {
            let state = (doc.current_page, doc.zoom);
            if self.last_tracked != Some(state) {
                let path = doc.path().to_path_buf();
                self.config.recent.update_state(&path, state.0, state.1);
                self.last_tracked = Some(state);
                self.dirty = true;
            }
        }
        if self.dirty {
            if self.last_save.elapsed() >= SAVE_EVERY {
                if let Err(err) = self.config.save() {
                    tracing::warn!("No se pudo guardar la configuración: {err}");
                }
                self.dirty = false;
                self.last_save = Instant::now();
            } else {
                ctx.request_repaint_after(SAVE_EVERY);
            }
        }
    }

    fn update_window_title(&mut self, ctx: &egui::Context) {
        let title = match &self.document {
            Some(doc) => format!("{} - Flint Reader", doc.title()),
            None => "Flint Reader".to_string(),
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
    }

    fn recent(&self) -> &RecentList {
        &self.config.recent
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.theme_applied != Some(self.config.settings.theme) {
            theme::apply(ctx, self.config.settings.theme);
            self.theme_applied = Some(self.config.settings.theme);
        }
        if let Some(doc) = &mut self.document {
            doc.renders_left = RENDERS_PER_FRAME;
        }

        for action in shortcuts::poll(ctx) {
            self.apply_shortcut(action);
        }
        self.handle_wheel_zoom(ctx);
        self.handle_dropped_files(ctx);

        // Búsqueda con retardo: se lanza cuando el usuario deja de escribir.
        if let Some(since) = self.search_pending {
            if since.elapsed() >= SEARCH_DEBOUNCE {
                self.run_search();
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        // ---------- Copiar el texto seleccionado (Ctrl+C) ----------
        let copy_requested = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Copy)));
        if copy_requested && !ctx.wants_keyboard_input() {
            self.copy_selection(ctx);
        }

        // ---------- Barra de herramientas (abajo) ----------
        let (current_page, page_count, zoom, sidebar_available) = match &self.document {
            Some(d) => (d.current_page, d.page_count(), d.zoom, !d.outline.is_empty()),
            None => (0, 0, 1.0, false),
        };
        let toolbar_actions = {
            let mut tb = ToolbarState {
                current_page,
                page_count,
                zoom,
                goto_input: &mut self.goto_input,
                view: self.view,
                theme: self.config.settings.theme,
                scroll_mode: self.config.settings.scroll_mode,
                sidebar_open: self.config.settings.sidebar_open,
                sidebar_available,
                search_open: self.search_open,
            };
            toolbar::show(ctx, &mut tb)
        };
        for action in toolbar_actions {
            self.apply_toolbar_action(action);
        }

        // ---------- Barra de búsqueda (justo encima de la barra de herramientas) ----------
        if self.search_open {
            let actions = {
                let mut st = SearchBarState {
                    query: &mut self.search_query,
                    results: &self.search_results,
                    selected: self.search_selected,
                    request_focus: self.search_request_focus,
                };
                search_bar::show(ctx, &mut st)
            };
            self.search_request_focus = false;

            for action in actions {
                match action {
                    SearchAction::Edited => self.search_pending = Some(Instant::now()),
                    SearchAction::Submit { backwards } => {
                        if self.search_pending.is_some() {
                            self.run_search();
                        }
                        self.step_result(backwards);
                    }
                    SearchAction::Prev => self.step_result(true),
                    SearchAction::Next => self.step_result(false),
                    SearchAction::JumpTo(i) => self.jump_to_result(i),
                    SearchAction::Close => self.toggle_search(),
                }
            }
        }

        // ---------- Panel lateral de índice ----------
        let mut outline_goto: Option<usize> = None;
        let screen_w = ctx.screen_rect().width();
        let sidebar_fits = screen_w >= 560.0;
        if sidebar_available && self.config.settings.sidebar_open && self.view == View::Reader && sidebar_fits {
            let document = &self.document;
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(self.config.settings.sidebar_width.min(screen_w * 0.4))
                .max_width(screen_w * 0.45)
                .show(ctx, |ui| {
                    if let Some(doc) = document {
                        outline_goto = sidebar::show(ui, &doc.outline);
                    }
                });
        }
        if let Some(page) = outline_goto {
            self.go_to_page(page);
        }

        // ---------- Panel central ----------
        let mut recent_action: Option<RecentAction> = None;
        let mut open_requested = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(status) = self.status.clone() {
                ui.colored_label(egui::Color32::LIGHT_RED, status);
                ui.separator();
            }

            match self.view {
                View::Recent => {
                    recent_action = recent_tab::show(ui, self.recent());
                }
                View::Reader => {
                    if let Some(doc) = &mut self.document {
                        let mode = self.config.settings.scroll_mode;
                        viewer::show(ui, ctx, doc, mode);
                    } else {
                        ui.add_space(64.0);
                        ui.vertical_centered(|ui| {
                            ui.heading("Flint Reader");
                            ui.add_space(8.0);
                            ui.label("Arrastra un PDF aquí, o ábrelo con el botón de abajo.");
                            ui.add_space(16.0);
                            if ui.button("Abrir un PDF…").clicked() {
                                open_requested = true;
                            }
                            ui.add_space(24.0);
                            ui.weak(format!("Versión {}", crate::utils::VERSION));
                        });
                    }
                }
            }
        });

        if let Some(t) = self.copied_at {
            if t.elapsed() < Duration::from_millis(1600) {
                egui::Area::new(egui::Id::new("copied_toast"))
                    .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -64.0])
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.label("Texto copiado");
                        });
                    });
                ctx.request_repaint_after(Duration::from_millis(200));
            } else {
                self.copied_at = None;
            }
        }

        if open_requested {
            self.pick_and_open();
        }
        if let Some(action) = recent_action {
            match action {
                RecentAction::Open(path) => self.open_path(path),
                RecentAction::Remove(path) => {
                    self.config.recent.remove(&path);
                    self.dirty = true;
                }
            }
        }
        if let Some(link) = self.document.as_mut().and_then(|d| d.pending_link.take()) {
            self.follow_link(link);
        }

        self.track_progress(ctx);
        self.update_window_title(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(doc) = &self.document {
            let path = doc.path().to_path_buf();
            self.config.recent.update_state(&path, doc.current_page, doc.zoom);
        }
        let _ = self.config.save();
    }
}
