//! Barra de búsqueda: aparece sobre la barra de herramientas (Ctrl+F).

use crate::pdf::SearchMatch;
use crate::ui::icons::{self, Icon};

#[derive(Debug, Clone)]
pub enum SearchAction {
    /// El texto cambió (la búsqueda se lanza con un pequeño retardo).
    Edited,
    /// Enter: ir al siguiente resultado (o buscar ya si había cambios).
    Submit { backwards: bool },
    Prev,
    Next,
    JumpTo(usize), // índice en la lista de resultados
    Close,
}

pub struct SearchBarState<'a> {
    pub query: &'a mut String,
    pub results: &'a [SearchMatch],
    pub selected: usize,
    pub request_focus: bool,
}

pub fn show(ctx: &egui::Context, state: &mut SearchBarState) -> Vec<SearchAction> {
    let mut actions = Vec::new();

    egui::TopBottomPanel::bottom("search_bar").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            icons::inline(ui, Icon::Search);

            let response = ui.add(
                egui::TextEdit::singleline(&mut *state.query)
                    .hint_text("Buscar en el documento…")
                    .desired_width(240.0),
            );
            if state.request_focus {
                response.request_focus();
            }
            if response.changed() {
                actions.push(SearchAction::Edited);
            }
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let backwards = ui.input(|i| i.modifiers.shift);
                actions.push(SearchAction::Submit { backwards });
                response.request_focus();
            }

            let has_results = !state.results.is_empty();
            ui.add_enabled_ui(has_results, |ui| {
                if icons::button(ui, Icon::Prev, "Resultado anterior (Mayús+Enter)", false).clicked() {
                    actions.push(SearchAction::Prev);
                }
                if icons::button(ui, Icon::Next, "Resultado siguiente (Enter)", false).clicked() {
                    actions.push(SearchAction::Next);
                }
            });

            if !state.query.trim().is_empty() {
                let total: usize = state.results.iter().map(|r| r.count).sum();
                let text = if has_results {
                    format!("{}/{} páginas · {} coincidencias", state.selected + 1, state.results.len(), total)
                } else {
                    "Sin resultados".to_string()
                };
                ui.label(egui::RichText::new(text).weak());
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::button(ui, Icon::Close, "Cerrar búsqueda (Esc)", false).clicked() {
                    actions.push(SearchAction::Close);
                }
            });
        });

        if !state.results.is_empty() {
            ui.add_space(2.0);
            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for (i, result) in state.results.iter().enumerate() {
                    let text = format!(
                        "Pág. {} ({}×) — {}",
                        result.page_index + 1,
                        result.count,
                        result.snippet
                    );
                    if ui.selectable_label(i == state.selected, text).clicked() {
                        actions.push(SearchAction::JumpTo(i));
                    }
                }
            });
        }
        ui.add_space(4.0);
    });

    actions
}
