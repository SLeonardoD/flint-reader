//! El visor de páginas en sí. Dos modos:
//!
//! - **Paginado**: muestra solo la página actual, centrada.
//! - **Continuo**: todas las páginas en una tira vertical, pero solo se
//!   rasteriza (y se mantiene en memoria como textura) la ventana de
//!   páginas cerca de lo visible; el resto es espacio reservado vacío.
//!   Esto es lo que mantiene el consumo de memoria bajo en documentos largos.

use crate::app::DocumentState;
use crate::config::ScrollMode;
use crate::pdf::PIXELS_PER_POINT;
use egui::{pos2, vec2, Color32, Rect, Sense};

/// Margen (en píxeles de pantalla) por encima/debajo del viewport dentro
/// del cual seguimos manteniendo páginas renderizadas, para que el scroll
/// no muestre un parpadeo al entrar una página nueva en pantalla.
const PRELOAD_MARGIN: f32 = 400.0;

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, doc: &mut DocumentState, scroll_mode: ScrollMode) {
    match scroll_mode {
        ScrollMode::Paginated => show_paginated(ui, ctx, doc),
        ScrollMode::Continuous => show_continuous(ui, ctx, doc),
    }
}

fn show_paginated(ui: &mut egui::Ui, ctx: &egui::Context, doc: &mut DocumentState) {
    let index = doc.current_page;

    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(16.0);
                if let Some(tex) = doc.texture_for(ctx, index) {
                    let size = tex.size_vec2();
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    paint_texture(ui, &tex, rect);
                } else {
                    ui.colored_label(Color32::LIGHT_RED, "No se pudo renderizar esta página.");
                }
                ui.add_space(16.0);
            });
        });
}

fn show_continuous(ui: &mut egui::Ui, ctx: &egui::Context, doc: &mut DocumentState) {
    let page_count = doc.page_count();
    const SPACING: f32 = 14.0;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(SPACING);

                let mut y = SPACING;
                let mut new_current_page = doc.current_page;
                let viewport_center = (viewport.min.y + viewport.max.y) / 2.0;

                for index in 0..page_count {
                    let (w_pts, h_pts) = doc.page_size_points(index);
                    let w_px = w_pts * doc.zoom * PIXELS_PER_POINT;
                    let h_px = h_pts * doc.zoom * PIXELS_PER_POINT;

                    let top = y;
                    let bottom = y + h_px;
                    let visible = bottom >= viewport.min.y - PRELOAD_MARGIN
                        && top <= viewport.max.y + PRELOAD_MARGIN;

                    let (rect, _) = ui.allocate_exact_size(vec2(w_px, h_px), Sense::hover());

                    if visible {
                        if let Some(tex) = doc.texture_for(ctx, index) {
                            paint_texture(ui, &tex, rect);
                        }
                        if viewport_center >= top && viewport_center <= bottom {
                            new_current_page = index;
                        }
                    } else {
                        // Fuera de la ventana de precarga: liberamos su textura
                        // (si la tenía) para no acumular memoria en PDFs largos.
                        doc.release_texture(index);
                        ui.painter().rect_filled(rect, 2.0, ui.visuals().extreme_bg_color);
                    }

                    y = bottom + SPACING;
                }

                doc.current_page = new_current_page;
                ui.add_space(SPACING);
            });
        });
}

fn paint_texture(ui: &egui::Ui, tex: &egui::TextureHandle, rect: Rect) {
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    ui.painter().image(tex.id(), rect, uv, Color32::WHITE);
}
