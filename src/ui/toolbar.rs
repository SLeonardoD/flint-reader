//! Barra de herramientas inferior, responsiva: según el ancho de la ventana
//! muestra todos los botones, solo los esenciales + un menú "⋯", o lo mínimo.

use crate::app::View;
use crate::config::{ScrollMode, ThemePreference};
use crate::ui::icons::{self, Icon};
use egui::AboveOrBelow;

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
    pub current_page: usize,
    pub page_count: usize,
    pub zoom: f32,
    pub goto_input: &'a mut String,
    pub view: View,
    pub theme: ThemePreference,
    pub scroll_mode: ScrollMode,
    pub sidebar_open: bool,
    pub sidebar_available: bool,
    pub search_open: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tier {
    /// Todo a la vista.
    Full,
    /// Navegación, zoom y búsqueda; el resto en el menú.
    Compact,
    /// Solo navegación; el resto en el menú.
    Tiny,
}

pub fn show(ctx: &egui::Context, state: &mut ToolbarState) -> Vec<ToolbarAction> {
    let mut actions = Vec::new();

    egui::TopBottomPanel::bottom("toolbar")
        .show_separator_line(true)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            let width = ui.available_width();
            let tier = if width >= 760.0 {
                Tier::Full
            } else if width >= 470.0 {
                Tier::Compact
            } else {
                Tier::Tiny
            };

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                if tier == Tier::Full {
                    if icons::button(ui, Icon::Open, "Abrir un PDF (Ctrl+O)", false).clicked() {
                        actions.push(ToolbarAction::OpenFile);
                    }
                    let recent = matches!(state.view, View::Recent);
                    if icons::button(ui, Icon::Recent, "Documentos recientes", recent).clicked() {
                        actions.push(if recent { ToolbarAction::ShowReader } else { ToolbarAction::ShowRecent });
                    }
                    if state.sidebar_available
                        && icons::button(ui, Icon::Index, "Índice del documento", state.sidebar_open).clicked()
                    {
                        actions.push(ToolbarAction::ToggleSidebar);
                    }
                    ui.separator();
                }

                nav_group(ui, state, &mut actions);

                if tier != Tier::Tiny {
                    ui.separator();
                    zoom_group(ui, state, &mut actions);
                    ui.separator();
                    if icons::button(ui, Icon::Search, "Buscar en el documento (Ctrl+F)", state.search_open)
                        .clicked()
                    {
                        actions.push(ToolbarAction::ToggleSearch);
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tier == Tier::Full {
                        if icons::button(ui, Icon::Theme, &format!("Tema: {}", state.theme.label()), false).clicked() {
                            actions.push(ToolbarAction::ToggleTheme);
                        }
                        let (icon, tip) = match state.scroll_mode {
                            ScrollMode::Continuous => (Icon::Continuous, "Desplazamiento continuo (clic para cambiar a por página)"),
                            ScrollMode::Paginated => (Icon::Paged, "Desplazamiento por página (clic para cambiar a continuo)"),
                        };
                        if icons::button(ui, icon, tip, false).clicked() {
                            actions.push(ToolbarAction::ToggleScrollMode);
                        }
                    } else {
                        overflow_menu(ui, state, tier, &mut actions);
                    }
                });
            });

            ui.add_space(4.0);
        });

    actions
}

fn nav_group(ui: &mut egui::Ui, state: &mut ToolbarState, actions: &mut Vec<ToolbarAction>) {
    let has_doc = state.page_count > 0;
    ui.add_enabled_ui(has_doc, |ui| {
        if icons::button(ui, Icon::Prev, "Página anterior", false).clicked() {
            actions.push(ToolbarAction::PrevPage);
        }

        // Mientras el usuario no esté escribiendo, el campo muestra siempre
        // la página actual (también cuando cambia al desplazarse).
        let goto_id = egui::Id::new("goto_page_input");
        let editing = ui.memory(|m| m.has_focus(goto_id));
        if !editing {
            *state.goto_input = if has_doc { (state.current_page + 1).to_string() } else { String::new() };
        }

        let response = ui.add(
            egui::TextEdit::singleline(&mut *state.goto_input)
                .id(goto_id)
                .desired_width(46.0)
                .horizontal_align(egui::Align::Center),
        );

        if response.gained_focus() {
            // Seleccionar todo al entrar, para sobreescribir sin borrar a mano.
            if let Some(mut te) = egui::text_edit::TextEditState::load(ui.ctx(), goto_id) {
                let len = state.goto_input.chars().count();
                te.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(len),
                )));
                te.store(ui.ctx(), goto_id);
            }
        }

        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            if let Ok(page) = state.goto_input.trim().parse::<usize>() {
                if page >= 1 {
                    actions.push(ToolbarAction::GotoPage(page - 1));
                }
            }
        }

        ui.label(format!("/ {}", state.page_count));

        if icons::button(ui, Icon::Next, "Página siguiente", false).clicked() {
            actions.push(ToolbarAction::NextPage);
        }
    });
}

fn zoom_group(ui: &mut egui::Ui, state: &mut ToolbarState, actions: &mut Vec<ToolbarAction>) {
    let has_doc = state.page_count > 0;
    ui.add_enabled_ui(has_doc, |ui| {
        if icons::button(ui, Icon::Minus, "Alejar (Ctrl -)", false).clicked() {
            actions.push(ToolbarAction::ZoomOut);
        }
        if ui
            .add(egui::Button::new(format!("{:.0}%", state.zoom * 100.0)).min_size(egui::vec2(52.0, 28.0)))
            .on_hover_text("Restablecer zoom (Ctrl 0)")
            .clicked()
        {
            actions.push(ToolbarAction::ZoomReset);
        }
        if icons::button(ui, Icon::Plus, "Acercar (Ctrl +)", false).clicked() {
            actions.push(ToolbarAction::ZoomIn);
        }
    });
}

/// Menú "⋯" con lo que no cabe en la barra cuando la ventana es estrecha.
fn overflow_menu(ui: &mut egui::Ui, state: &mut ToolbarState, tier: Tier, actions: &mut Vec<ToolbarAction>) {
    let popup_id = ui.make_persistent_id("toolbar_overflow_menu");
    let button = icons::button(ui, Icon::Menu, "Más opciones", false);
    if button.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup_id));
    }

    let has_doc = state.page_count > 0;
    egui::popup::popup_above_or_below_widget(ui, popup_id, &button, AboveOrBelow::Above, |ui| {
        ui.set_min_width(190.0);
        let mut chosen: Option<ToolbarAction> = None;

        let mut item = |ui: &mut egui::Ui, label: &str, enabled: bool, selected: bool, action: ToolbarAction| {
            if ui.add_enabled(enabled, egui::SelectableLabel::new(selected, label)).clicked() {
                chosen = Some(action);
            }
        };

        item(ui, "Abrir un PDF…", true, false, ToolbarAction::OpenFile);
        let recent = matches!(state.view, View::Recent);
        item(
            ui,
            if recent { "Volver al lector" } else { "Documentos recientes" },
            true,
            recent,
            if recent { ToolbarAction::ShowReader } else { ToolbarAction::ShowRecent },
        );
        if state.sidebar_available {
            item(ui, "Índice del documento", true, state.sidebar_open, ToolbarAction::ToggleSidebar);
        }
        ui.separator();
        if tier == Tier::Tiny {
            item(ui, "Buscar…", has_doc, state.search_open, ToolbarAction::ToggleSearch);
            item(ui, "Acercar", has_doc, false, ToolbarAction::ZoomIn);
            item(ui, "Alejar", has_doc, false, ToolbarAction::ZoomOut);
            ui.separator();
        }
        let mode_label = match state.scroll_mode {
            ScrollMode::Continuous => "Desplazamiento: continuo",
            ScrollMode::Paginated => "Desplazamiento: por página",
        };
        item(ui, mode_label, true, false, ToolbarAction::ToggleScrollMode);
        item(ui, &format!("Tema: {}", state.theme.label()), true, false, ToolbarAction::ToggleTheme);

        if let Some(action) = chosen {
            actions.push(action);
            ui.memory_mut(|m| m.close_popup());
        }
    });
}
