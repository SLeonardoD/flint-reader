//! El visor de páginas. Dos modos:
//!
//! - **Paginado**: solo la página actual, con scroll interno si no cabe.
//! - **Continuo**: tira vertical de páginas. Solo se rasterizan las cercanas
//!   a lo visible; el resto es espacio reservado (memoria baja en PDFs largos).
//!
//! El desplazamiento programático (ir a página, flechas, zoom, restaurar
//! posición) se pide con `doc.scroll_target` y se aplica aquí al crear el
//! `ScrollArea`. También se maneja el paneo con el botón central del ratón,
//! los hipervínculos del PDF y el resaltado de búsqueda.

use crate::app::DocumentState;
use crate::config::ScrollMode;
use egui::{pos2, vec2, Color32, Context, CursorIcon, Rect, ScrollArea, Sense, Stroke, Ui, Vec2};

/// Espacio entre páginas y margen superior/inferior (puntos de pantalla).
pub const PAGE_GAP: f32 = 14.0;
/// Margen lateral mínimo alrededor de las páginas.
const SIDE_MARGIN: f32 = 16.0;
/// Distancia al viewport dentro de la cual se renderizan páginas nuevas.
const RENDER_MARGIN: f32 = 600.0;
/// Distancia más allá de la cual se liberan las texturas.
const KEEP_MARGIN: f32 = 2400.0;

pub fn show(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, mode: ScrollMode) {
    let panel_rect = ui.available_rect_before_wrap();
    doc.viewer_rect = panel_rect;

    handle_middle_button_pan(ctx, doc, panel_rect);

    // Aplicar el desplazamiento pedido (si lo hay) al crear el ScrollArea.
    let target = std::mem::take(&mut doc.scroll_target);
    let mut area = ScrollArea::both()
        // Un ID distinto por documento: así el scroll no se "contagia" de
        // un documento al siguiente.
        .id_source(("viewer", doc.id))
        .auto_shrink([false, false]);
    if let Some(x) = target.x {
        area = area.horizontal_scroll_offset(x);
    }
    if let Some(y) = target.y {
        area = area.vertical_scroll_offset(y);
    }
    let jumped = target.y.is_some();
    let avail_w = panel_rect.width();

    let offset = match mode {
        ScrollMode::Continuous => area
            .show_viewport(ui, |ui, viewport| continuous(ui, ctx, doc, viewport, jumped, avail_w))
            .state
            .offset,
        ScrollMode::Paginated => area
            .show(ui, |ui| paginated(ui, ctx, doc, avail_w))
            .state
            .offset,
    };
    doc.scroll = offset;
}

/// Mantener pulsada la rueda del ratón y arrastrar desplaza el documento.
fn handle_middle_button_pan(ctx: &Context, doc: &mut DocumentState, panel_rect: Rect) {
    let (middle, delta, hover) = ctx.input(|i| (i.pointer.middle_down(), i.pointer.delta(), i.pointer.hover_pos()));

    if middle {
        let inside = hover.map_or(false, |p| panel_rect.contains(p));
        if doc.panning || inside {
            doc.panning = true;
            doc.nudge_scroll(-delta.x, -delta.y);
            ctx.set_cursor_icon(CursorIcon::Grabbing);
        }
    } else {
        doc.panning = false;
    }
}

fn continuous(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, viewport: Rect, jumped: bool, avail_w: f32) {
    // Sin espaciado automático: las posiciones de las páginas deben coincidir
    // exactamente con las que calcula `DocumentState::page_top`.
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    let count = doc.page_count();
    let max_w = (0..count).map(|i| doc.page_extent(i).x).fold(0.0_f32, f32::max);
    let content_w = (max_w + 2.0 * SIDE_MARGIN).max(avail_w);

    ui.add_space(PAGE_GAP);
    let mut y = PAGE_GAP;
    // La "página actual" es la que más área visible ocupa (empates: la primera).
    let mut best = (doc.current_page, 0.0_f32);

    for index in 0..count {
        let ext = doc.page_extent(index);
        let top = y;
        let bottom = y + ext.y;
        y = bottom + PAGE_GAP;

        let (row, _) = ui.allocate_exact_size(vec2(content_w, ext.y + PAGE_GAP), Sense::hover());

        let overlap = bottom.min(viewport.max.y) - top.max(viewport.min.y);
        if overlap > best.1 + 0.5 {
            best = (index, overlap);
        }

        let near = bottom >= viewport.min.y - RENDER_MARGIN && top <= viewport.max.y + RENDER_MARGIN;
        if near {
            let page_rect = Rect::from_min_size(pos2(row.min.x + (content_w - ext.x) / 2.0, row.min.y), ext);
            draw_page(ui, ctx, doc, index, page_rect, false);
        } else if bottom < viewport.min.y - KEEP_MARGIN || top > viewport.max.y + KEEP_MARGIN {
            doc.release_texture(index);
        }
    }

    if !jumped && best.1 > 0.0 {
        doc.current_page = best.0;
    }
}

fn paginated(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, avail_w: f32) {
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    let index = doc.current_page;
    let ext = doc.page_extent(index);
    let content_w = (ext.x + 2.0 * SIDE_MARGIN).max(avail_w);

    ui.add_space(PAGE_GAP);
    let (row, _) = ui.allocate_exact_size(vec2(content_w, ext.y), Sense::hover());
    let page_rect = Rect::from_min_size(pos2(row.min.x + (content_w - ext.x) / 2.0, row.min.y), ext);
    draw_page(ui, ctx, doc, index, page_rect, true);
    ui.add_space(PAGE_GAP);

    doc.keep_textures_near(index, 1);
}

fn draw_page(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, index: usize, page_rect: Rect, force: bool) {
    match doc.texture_for(ctx, index, force) {
        Some(tex) => {
            let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            ui.painter().image(tex.id(), page_rect, uv, Color32::WHITE);
        }
        // Mientras se renderiza: hoja en blanco, para que no parpadee.
        None => {
            ui.painter().rect_filled(page_rect, 0.0, Color32::from_gray(245));
        }
    }
    ui.painter()
        .rect_stroke(page_rect, 0.0, Stroke::new(1.0, Color32::from_black_alpha(60)));

    paint_overlays(ui, doc, index, page_rect);
}

/// Resaltado de búsqueda e hipervínculos clicables sobre una página.
fn paint_overlays(ui: &mut Ui, doc: &mut DocumentState, index: usize, page_rect: Rect) {
    let (w_pts, _) = doc.page_size_points(index);
    if w_pts <= 0.0 {
        return;
    }
    let s = page_rect.width() / w_pts; // puntos de pantalla por punto PDF
    let to_screen = |r: &[f32; 4]| {
        Rect::from_min_max(
            pos2(page_rect.min.x + r[0] * s, page_rect.min.y + r[1] * s),
            pos2(page_rect.min.x + r[2] * s, page_rect.min.y + r[3] * s),
        )
    };

    // Coincidencias de búsqueda
    for r in doc.highlights_for(index).iter() {
        ui.painter()
            .rect_filled(to_screen(r).expand(1.0), 2.0, Color32::from_rgba_unmultiplied(255, 190, 0, 105));
    }

    // Hipervínculos
    for (k, link) in doc.links_for(index).iter().enumerate() {
        let rect = to_screen(&link.rect);
        let id = egui::Id::new(("pdf-link", doc.id, index, k));
        let response = ui.interact(rect, id, Sense::click());
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            ui.painter().rect_stroke(rect, 1.0, Stroke::new(1.0, crate::ui::theme::ACCENT));
        }
        if response.clicked() {
            doc.pending_link = Some(link.target.clone());
        }
    }
}
