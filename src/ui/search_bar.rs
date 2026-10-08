//! Barra de búsqueda: aparece bajo la barra de herramientas cuando el
//! usuario pulsa "Buscar" o Ctrl+F. Busca en todo el documento y permite
//! saltar entre los resultados.

use crate::pdf::SearchMatch;

#[derive(Debug, Clone)]
pub enum SearchAction {
    RunSearch,
    JumpTo(usize),
    Close,
}

pub struct SearchBarState<'a> {
    pub query: &'a mut String,
    pub results: &'a [SearchMatch],
    pub selected: &'a mut usize,
    pub request_focus: bool,
}

pub fn show(ctx: &egui::Context, state: &mut SearchBarState) -> Vec<SearchAction> {
    let mut actions = Vec::new();

    egui::TopBottomPanel::top("search_bar").show(ctx, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("🔍");
            let edit = egui::TextEdit::singleline(state.query)
                .hint_text("Buscar en el documento…")
                .desired_width(260.0);
            let response = ui.add(edit);

            if state.request_focus {
                response.request_focus();
            }

            if response.changed() {
                actions.push(SearchAction::RunSearch);
            }

            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
            if enter_pressed && !state.results.is_empty() {
                *state.selected = (*state.selected + 1) % state.results.len();
                actions.push(SearchAction::JumpTo(state.results[*state.selected].page_index));
            }

            ui.label(format!(
                "{} página(s) con coincidencias",
                state.results.len()
            ));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Cerrar").clicked() {
                    actions.push(SearchAction::Close);
                }
            });
        });

        if !state.results.is_empty() {
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .max_height(160.0)
                .show(ui, |ui| {
                    for (i, result) in state.results.iter().enumerate() {
                        let is_selected = i == *state.selected;
                        let text = format!(
                            "Pág. {} ({}×) — {}",
                            result.page_index + 1,
                            result.count,
                            result.snippet
                        );
                        if ui.selectable_label(is_selected, text).clicked() {
                            *state.selected = i;
                            actions.push(SearchAction::JumpTo(result.page_index));
                        }
                    }
                });
        }
        ui.add_space(4.0);
    });

    actions
}
