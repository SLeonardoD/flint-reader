//! Panel lateral con el índice (marcadores) del documento, cuando el PDF
//! trae uno embebido. Es un árbol colapsable; al hacer clic en un nodo,
//! se salta a su página de destino.

use crate::pdf::OutlineNode;

/// Muestra el árbol de marcadores y devuelve `Some(página)` si el usuario
/// hizo clic en alguno que tenga una página de destino.
pub fn show(ui: &mut egui::Ui, nodes: &[OutlineNode]) -> Option<usize> {
    let mut goto: Option<usize> = None;

    if nodes.is_empty() {
        ui.label("Este documento no tiene índice.");
        return None;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for node in nodes {
            show_node(ui, node, &mut goto);
        }
    });

    goto
}

fn show_node(ui: &mut egui::Ui, node: &OutlineNode, goto: &mut Option<usize>) {
    if node.children.is_empty() {
        if ui.selectable_label(false, &node.title).clicked() {
            if let Some(page) = node.page_index {
                *goto = Some(page);
            }
        }
        return;
    }

    let header = egui::CollapsingHeader::new(&node.title)
        .default_open(false)
        .show(ui, |ui| {
            for child in &node.children {
                show_node(ui, child, goto);
            }
        });

    // Clicar el título del grupo también navega a su propio destino, si tiene.
    if header.header_response.clicked() {
        if let Some(page) = node.page_index {
            *goto = Some(page);
        }
    }
}
