//! `App` es el struct raíz que implementa `eframe::App`. Mantiene el
//! estado de la aplicación (documento abierto, configuración, vista
//! activa) y en cada frame: procesa atajos de teclado, dibuja la barra de
//! herramientas y el resto de paneles, y aplica las acciones que estos
//! devuelven. Los módulos de `ui` nunca mutan `App` directamente — solo
//! dibujan y devuelven qué acción quiere el usuario.

use crate::config::{AppConfig, RecentList, ThemePreference};
use crate::input::shortcuts::{self, ShortcutAction};
use crate::pdf::{OpenDocument, OutlineNode, PdfEngine, SearchMatch};
use crate::ui::recent_tab::RecentAction;
use crate::ui::search_bar::SearchAction;
use crate::ui::toolbar::ToolbarAction;
use crate::ui::{recent_tab, search_bar, sidebar, theme, toolbar, viewer};
use crate::utils::{clamp_zoom, ZOOM_STEP};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Reader,
    Recent,
}

/// Estado del documento actualmente abierto: página, zoom, índice y la
/// caché de texturas ya renderizadas (ver `texture_for`).
pub struct DocumentState {
    doc: OpenDocument,
    pub current_page: usize,
    pub zoom: f32,
    pub outline: Vec<OutlineNode>,
    pub goto_input: String,
    textures: HashMap<usize, (f32, egui::TextureHandle)>,
}

impl DocumentState {
    fn new(doc: OpenDocument, current_page: usize, zoom: f32) -> Self {
        let outline = doc.outline();
        Self {
            doc,
            current_page,
            zoom,
            outline,
            goto_input: (current_page + 1).to_string(),
            textures: HashMap::new(),
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
        self.doc.page_size_points(index).unwrap_or((612.0, 792.0))
    }

    /// Devuelve la textura ya renderizada de la página `index`, renderizándola
    /// (y cacheándola) primero si hace falta o si el zoom cambió desde la
    /// última vez.
    pub fn texture_for(&mut self, ctx: &egui::Context, index: usize) -> Option<egui::TextureHandle> {
        if let Some((zoom, tex)) = self.textures.get(&index) {
            if (*zoom - self.zoom).abs() < f32::EPSILON {
                return Some(tex.clone());
            }
        }

        let rendered = self.doc.render_page(index, self.zoom).ok()?;
        let tex = ctx.load_texture(
            format!("page-{index}"),
            rendered.image,
            egui::TextureOptions::LINEAR,
        );
        self.textures.insert(index, (self.zoom, tex.clone()));
        Some(tex)
    }

    /// Libera la textura de una página que salió de la ventana de precarga.
    pub fn release_texture(&mut self, index: usize) {
        self.textures.remove(&index);
    }

    fn invalidate_textures(&mut self) {
        self.textures.clear();
    }

    pub fn search(&self, query: &str) -> Vec<SearchMatch> {
        self.doc.search(query)
    }
}

pub struct App {
    engine: PdfEngine,
    config: AppConfig,
    view: View,
    document: Option<DocumentState>,

    search_open: bool,
    search_query: String,
    search_results: Vec<SearchMatch>,
    search_selected: usize,
    search_request_focus: bool,

    status: Option<String>,
    theme_applied: Option<ThemePreference>,
}

impl App {
    pub fn new(engine: PdfEngine, config: AppConfig) -> Self {
        Self {
            engine,
            config,
            view: View::Reader,
            document: None,
            search_open: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_request_focus: false,
            status: None,
            theme_applied: None,
        }
    }

    fn open_path(&mut self, path: PathBuf) {
        match self.engine.open(&path) {
            Ok(doc) => {
                let (page, zoom) = self
                    .config
                    .recent
                    .find(&path)
                    .map(|r| (r.last_page, r.zoom))
                    .unwrap_or((0, self.config.settings.default_zoom));

                let page_count = doc.page_count().max(1);
                let page = page.min(page_count - 1);
                let state = DocumentState::new(doc, page, clamp_zoom(zoom));

                self.config
                    .recent
                    .touch(path, state.current_page, state.zoom);
                let _ = self.config.save();

                self.document = Some(state);
                self.view = View::Reader;
                self.search_open = false;
                self.search_results.clear();
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

    /// Guarda en `config.toml` la página/zoom actuales del documento abierto.
    fn save_current_state(&mut self) {
        if let Some(doc) = &self.document {
            let path = doc.path().to_path_buf();
            self.config
                .recent
                .update_state(&path, doc.current_page, doc.zoom);
            let _ = self.config.save();
        }
    }

    fn go_to_relative(&mut self, delta: isize) {
        if let Some(doc) = &mut self.document {
            let count = doc.page_count().max(1);
            let current = doc.current_page as isize;
            let target = (current + delta).clamp(0, count as isize - 1) as usize;
            doc.current_page = target;
            doc.goto_input = (target + 1).to_string();
        }
        self.save_current_state();
    }

    fn go_to_page(&mut self, page_index: usize) {
        if let Some(doc) = &mut self.document {
            let count = doc.page_count().max(1);
            let target = page_index.min(count - 1);
            doc.current_page = target;
            doc.goto_input = (target + 1).to_string();
        }
        self.save_current_state();
    }

    fn adjust_zoom(&mut self, delta: f32) {
        if let Some(doc) = &mut self.document {
            let new_zoom = clamp_zoom(doc.zoom + delta);
            if (new_zoom - doc.zoom).abs() > f32::EPSILON {
                doc.zoom = new_zoom;
                doc.invalidate_textures();
            }
        }
        self.save_current_state();
    }

    fn set_zoom(&mut self, zoom: f32) {
        if let Some(doc) = &mut self.document {
            doc.zoom = clamp_zoom(zoom);
            doc.invalidate_textures();
        }
        self.save_current_state();
    }

    fn toggle_search(&mut self) {
        self.search_open = !self.search_open;
        self.search_request_focus = self.search_open;
        if !self.search_open {
            self.search_results.clear();
        }
    }

    fn run_search(&mut self) {
        if let Some(doc) = &self.document {
            self.search_results = doc.search(&self.search_query);
            self.search_selected = 0;
        }
    }

    fn apply_toolbar_action(&mut self, action: ToolbarAction) {
        match action {
            ToolbarAction::OpenFile => self.pick_and_open(),
            ToolbarAction::PrevPage => self.go_to_relative(-1),
            ToolbarAction::NextPage => self.go_to_relative(1),
            ToolbarAction::GotoPage(p) => self.go_to_page(p),
            ToolbarAction::ZoomIn => self.adjust_zoom(ZOOM_STEP),
            ToolbarAction::ZoomOut => self.adjust_zoom(-ZOOM_STEP),
            ToolbarAction::ZoomReset => self.set_zoom(1.0),
            ToolbarAction::ToggleSearch => self.toggle_search(),
            ToolbarAction::ToggleTheme => {
                self.config.settings.theme = self.config.settings.theme.toggled();
                let _ = self.config.save();
            }
            ToolbarAction::ToggleScrollMode => {
                self.config.settings.scroll_mode = self.config.settings.scroll_mode.toggled();
                let _ = self.config.save();
            }
            ToolbarAction::ToggleSidebar => {
                self.config.settings.sidebar_open = !self.config.settings.sidebar_open;
                let _ = self.config.save();
            }
            ToolbarAction::ShowRecent => self.view = View::Recent,
            ToolbarAction::ShowReader => self.view = View::Reader,
        }
    }

    fn apply_shortcut(&mut self, action: ShortcutAction) {
        match action {
            ShortcutAction::NextPage => self.go_to_relative(1),
            ShortcutAction::PrevPage => self.go_to_relative(-1),
            ShortcutAction::ZoomIn => self.adjust_zoom(ZOOM_STEP),
            ShortcutAction::ZoomOut => self.adjust_zoom(-ZOOM_STEP),
            ShortcutAction::ZoomReset => self.set_zoom(1.0),
            ShortcutAction::ToggleSearch => self.toggle_search(),
            ShortcutAction::OpenFile => self.pick_and_open(),
            ShortcutAction::CloseOverlay => {
                if self.search_open {
                    self.toggle_search();
                }
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = file.path {
                let is_pdf = path
                    .extension()
                    .map(|e| e.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false);
                if is_pdf {
                    self.open_path(path);
                    break;
                }
            }
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

        for action in shortcuts::poll(ctx) {
            self.apply_shortcut(action);
        }

        self.handle_dropped_files(ctx);

        // ---------- Barra de herramientas ----------
        let title = self.document.as_ref().map(|d| d.title().to_string());
        let (current_page, page_count, zoom, sidebar_available) = match &self.document {
            Some(doc) => (doc.current_page, doc.page_count(), doc.zoom, !doc.outline.is_empty()),
            None => (0, 0, 1.0, false),
        };

        let mut goto_input_buf = self
            .document
            .as_ref()
            .map(|d| d.goto_input.clone())
            .unwrap_or_default();

        let toolbar_actions = {
            let mut toolbar_state = toolbar::ToolbarState {
                title: title.as_deref(),
                current_page,
                page_count,
                zoom,
                goto_input: &mut goto_input_buf,
                view: self.view,
                theme: self.config.settings.theme,
                scroll_mode: self.config.settings.scroll_mode,
                sidebar_open: self.config.settings.sidebar_open,
                sidebar_available,
            };
            toolbar::show(ctx, &mut toolbar_state)
        };

        if let Some(doc) = &mut self.document {
            doc.goto_input = goto_input_buf;
        }

        for action in toolbar_actions {
            self.apply_toolbar_action(action);
        }

        // ---------- Barra de búsqueda ----------
        if self.search_open {
            let search_actions = {
                let mut search_state = search_bar::SearchBarState {
                    query: &mut self.search_query,
                    results: &self.search_results,
                    selected: &mut self.search_selected,
                    request_focus: self.search_request_focus,
                };
                search_bar::show(ctx, &mut search_state)
            };
            self.search_request_focus = false;

            for action in search_actions {
                match action {
                    SearchAction::RunSearch => self.run_search(),
                    SearchAction::JumpTo(page) => self.go_to_page(page),
                    SearchAction::Close => self.toggle_search(),
                }
            }
        }

        // ---------- Panel lateral de índice ----------
        let mut outline_goto: Option<usize> = None;
        if sidebar_available && self.config.settings.sidebar_open && matches!(self.view, View::Reader) {
            let document = &self.document;
            egui::SidePanel::left("sidebar")
                .resizable(true)
                .default_width(self.config.settings.sidebar_width)
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
        let mut pending_recent_action: Option<RecentAction> = None;
        let mut open_requested = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(status) = self.status.clone() {
                ui.colored_label(egui::Color32::LIGHT_RED, status);
                ui.separator();
            }

            match self.view {
                View::Recent => {
                    pending_recent_action = recent_tab::show(ui, self.recent());
                }
                View::Reader => {
                    if let Some(doc) = &mut self.document {
                        let scroll_mode = self.config.settings.scroll_mode;
                        viewer::show(ui, ctx, doc, scroll_mode);
                    } else {
                        ui.add_space(64.0);
                        ui.vertical_centered(|ui| {
                            ui.heading("Lector PDF");
                            ui.add_space(8.0);
                            ui.label("Arrastra un PDF aquí, o ábrelo con el botón de abajo.");
                            ui.add_space(16.0);
                            if ui.button("📂 Abrir un PDF…").clicked() {
                                open_requested = true;
                            }
                        });
                    }
                }
            }
        });

        if open_requested {
            self.pick_and_open();
        }

        if let Some(action) = pending_recent_action {
            match action {
                RecentAction::Open(path) => self.open_path(path),
                RecentAction::Remove(path) => {
                    self.config.recent.remove(&path);
                    let _ = self.config.save();
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_current_state();
        let _ = self.config.save();
    }
}
