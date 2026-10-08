//! Vista de "Recientes": una lista de los últimos PDFs abiertos, con la
//! página en la que se quedó cada uno. Es una pestaña separada a la que
//! el usuario navega desde la barra de herramientas, no la pantalla de
//! inicio de la app.

use crate::config::RecentList;
use std::path::PathBuf;

pub enum RecentAction {
    Open(PathBuf),
    Remove(PathBuf),
}

pub fn show(ui: &mut egui::Ui, recent: &RecentList) -> Option<RecentAction> {
    let mut action = None;

    ui.add_space(12.0);
    ui.heading("Documentos recientes");
    ui.add_space(4.0);
    ui.separator();

    if recent.items.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            ui.label("Todavía no has abierto ningún PDF.");
        });
        return None;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in &recent.items {
            let missing = item.is_missing();

            ui.horizontal(|ui| {
                ui.add_space(4.0);

                let label = if missing {
                    format!("⚠ {} (no encontrado)", item.title())
                } else {
                    item.title()
                };

                let button = egui::Button::new(label).frame(false);
                if ui.add_enabled(!missing, button).clicked() {
                    action = Some(RecentAction::Open(item.path.clone()));
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("✕").on_hover_text("Quitar de recientes").clicked() {
                        action = Some(RecentAction::Remove(item.path.clone()));
                    }
                    ui.label(
                        egui::RichText::new(format!("pág. {}  ·  zoom {:.0}%", item.last_page + 1, item.zoom * 100.0))
                            .weak(),
                    );
                });
            });
            ui.separator();
        }
    });

    action
}
