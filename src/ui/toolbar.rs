//! Barra de herramientas superior. Concentra lo que el usuario necesita
//! para navegar un PDF: abrir archivo, página actual / total con salto
//! directo, zoom, búsqueda, y los interruptores de tema / modo de scroll /
//! panel de marcadores / vista de recientes.

use crate::app::View;
use crate::config::{ScrollMode, ThemePreference};

#[derive(Debug, Clone)]
pub enum ToolbarAction {
    OpenFile,
    PrevPage,
    NextPage,
    GotoPage(usize),
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleSearch,
    ToggleTheme,
    ToggleScrollMode,
    ToggleSidebar,
    ShowRecent,
    ShowReader,
}

pub struct ToolbarState<'a> {
    pub title: Option<&'a str>,
    pub current_page: usize,
    pub page_count: usize,
    pub zoom: f32,
    pub goto_input: &'a mut String,
    pub view: View,
    pub theme: ThemePreference,
    pub scroll_mode: ScrollMode,
    pub sidebar_open: bool,
    pub sidebar_available: bool,
}

pub fn show(ctx: &egui::Context, state: &mut ToolbarState) -> Vec<ToolbarAction> {
    let mut actions = Vec::new();

    egui::TopBottomPanel::top("toolbar")
        .show_separator_line(true)
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("📂 Abrir").on_hover_text("Abrir un PDF (Ctrl+O)").clicked() {
                    actions.push(ToolbarAction::OpenFile);
                }

                ui.separator();

                let recent_selected = matches!(state.view, View::Recent);
                if ui.selectable_label(recent_selected, "🕑 Recientes").clicked() {
                    actions.push(if recent_selected {
                        ToolbarAction::ShowReader
                    } else {
                        ToolbarAction::ShowRecent
                    });
                }

                if state.sidebar_available {
                    if ui
                        .selectable_label(state.sidebar_open, "📑 Índice")
                        .clicked()
                    {
                        actions.push(ToolbarAction::ToggleSidebar);
                    }
                }

                ui.separator();

                // --- Navegación de páginas ---
                let has_doc = state.page_count > 0;
                ui.add_enabled_ui(has_doc, |ui| {
                    if ui.button("◀").on_hover_text("Página anterior").clicked() {
                        actions.push(ToolbarAction::PrevPage);
                    }

                    let response = ui.add(
                        egui::TextEdit::singleline(state.goto_input)
                            .desired_width(42.0)
                            .horizontal_align(egui::Align::Center),
                    );
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(page) = state.goto_input.trim().parse::<usize>() {
                            if page >= 1 {
                                actions.push(ToolbarAction::GotoPage(page - 1));
                            }
                        }
                    }

                    ui.label(format!("/ {}", state.page_count))
                        .on_hover_text(format!("Estás en la página {}", state.current_page + 1));

                    if ui.button("▶").on_hover_text("Página siguiente").clicked() {
                        actions.push(ToolbarAction::NextPage);
                    }
                });

                ui.separator();

                // --- Zoom ---
                ui.add_enabled_ui(has_doc, |ui| {
                    if ui.button("－").on_hover_text("Alejar (Ctrl -)").clicked() {
                        actions.push(ToolbarAction::ZoomOut);
                    }
                    if ui
                        .button(format!("{:.0}%", state.zoom * 100.0))
                        .on_hover_text("Restablecer zoom")
                        .clicked()
                    {
                        actions.push(ToolbarAction::ZoomReset);
                    }
                    if ui.button("＋").on_hover_text("Acercar (Ctrl +)").clicked() {
                        actions.push(ToolbarAction::ZoomIn);
                    }
                });

                ui.separator();

                if ui
                    .button("🔍 Buscar")
                    .on_hover_text("Buscar en el documento (Ctrl+F)")
                    .clicked()
                {
                    actions.push(ToolbarAction::ToggleSearch);
                }

                // --- Controles a la derecha ---
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(theme_icon(state.theme))
                        .on_hover_text(format!("Tema: {}", state.theme.label()))
                        .clicked()
                    {
                        actions.push(ToolbarAction::ToggleTheme);
                    }
                    if ui
                        .button(scroll_mode_icon(state.scroll_mode))
                        .on_hover_text(format!("Desplazamiento: {}", state.scroll_mode.label()))
                        .clicked()
                    {
                        actions.push(ToolbarAction::ToggleScrollMode);
                    }
                    if let Some(title) = state.title {
                        ui.label(egui::RichText::new(title).strong());
                    }
                });
            });
            ui.add_space(4.0);
        });

    actions
}

fn theme_icon(theme: ThemePreference) -> &'static str {
    match theme {
        ThemePreference::Dark => "🌙",
        ThemePreference::Light => "☀",
    }
}

fn scroll_mode_icon(mode: ScrollMode) -> &'static str {
    match mode {
        ScrollMode::Continuous => "⬍",
        ScrollMode::Paginated => "▯",
    }
}
