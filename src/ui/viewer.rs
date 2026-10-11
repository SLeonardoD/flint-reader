//! El visor de páginas. Dos modos:
//!
//! - **Paginado**: solo la página actual, con scroll interno si no cabe.
//! - **Continuo**: tira vertical de páginas. Solo se rasterizan las cercanas
//!   a lo visible; el resto es espacio reservado (memoria baja en PDFs largos).
//!
//! El desplazamiento programático (ir a página, flechas, zoom, restaurar
//! posición) se pide con `doc.scroll_target` y se aplica aquí al crear el
//! `ScrollArea`. Aquí también viven el autoscroll con la rueda del ratón,
//! los hipervínculos, el resaltado de búsqueda y el dibujo de la selección.
//! La lógica del ratón para seleccionar texto está en `selection.rs`.

use crate::app::DocumentState;
use crate::config::ScrollMode;
use crate::ui::{selection, theme};
use egui::{pos2, vec2, Color32, Context, CursorIcon, Rect, ScrollArea, Sense, Shape, Stroke, Ui, Vec2};

/// Espacio entre páginas y margen superior/inferior (puntos de pantalla).
pub const PAGE_GAP: f32 = 14.0;
/// Margen lateral mínimo alrededor de las páginas.
const SIDE_MARGIN: f32 = 16.0;
/// Distancia al viewport dentro de la cual se renderizan páginas nuevas.
const RENDER_MARGIN: f32 = 600.0;
/// Distancia más allá de la cual se liberan las texturas.
const KEEP_MARGIN: f32 = 2400.0;

/// Autoscroll: radio (px) alrededor del punto de partida donde no se mueve nada.
const AUTOSCROLL_DEAD_ZONE: f32 = 14.0;
/// Autoscroll: píxeles por segundo por cada píxel de distancia más allá de la zona muerta.
const AUTOSCROLL_GAIN: f32 = 7.0;
const AUTOSCROLL_MAX_SPEED: f32 = 2500.0;

pub fn show(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, mode: ScrollMode) {
    let panel_rect = ui.available_rect_before_wrap();
    doc.viewer_rect = panel_rect;
    doc.page_rects.clear();
    doc.hovering_link = false;

    handle_autoscroll(ctx, doc, panel_rect);

    // Aplicar el desplazamiento pedido (si lo hay) al crear el ScrollArea.
    let target = std::mem::take(&mut doc.scroll_target);
    let mut area = ScrollArea::both()
        // Un ID distinto por documento: así el scroll no se "contagia" de
        // un documento al siguiente.
        .id_source(("viewer", doc.id))
        .auto_shrink([false, false])
        // El arrastre con el botón izquierdo selecciona texto (ver `selection.rs`).
        .drag_to_scroll(false);
    if let Some(x) = target.x {
        area = area.horizontal_scroll_offset(x);
    }
    if let Some(y) = target.y {
        area = area.vertical_scroll_offset(y);
    }
    let jumped = target.y.is_some();

    let offset = match mode {
        ScrollMode::Continuous => area
            .show_viewport(ui, |ui, viewport| continuous(ui, ctx, doc, viewport, jumped))
            .state
            .offset,
        ScrollMode::Paginated => area
            .show_viewport(ui, |ui, viewport| paginated(ui, ctx, doc, viewport.width()))
            .state
            .offset,
    };
    doc.scroll = offset;

    // La selección se gestiona tras dibujar: necesita saber dónde quedó cada página.
    selection::handle(ctx, doc, panel_rect);
}

/// Autoscroll con la rueda: un clic central lo activa, y desde ahí basta
/// mover el ratón (el documento se desplaza más rápido cuanto más lejos del
/// punto de partida). Otro clic cualquiera, o Esc, lo desactiva.
fn handle_autoscroll(ctx: &Context, doc: &mut DocumentState, panel_rect: Rect) {
    let (middle, primary, secondary, esc, primary_down, primary_released, pos, dt) = ctx.input(|i| {
        (
            i.pointer.button_pressed(egui::PointerButton::Middle),
            i.pointer.button_pressed(egui::PointerButton::Primary),
            i.pointer.button_pressed(egui::PointerButton::Secondary),
            i.key_pressed(egui::Key::Escape),
            i.pointer.primary_down(),
            i.pointer.primary_released(),
            i.pointer.hover_pos().or_else(|| i.pointer.latest_pos()),
            i.stable_dt.min(0.1),
        )
    });

    // El clic que cierra el autoscroll no debe abrir un enlace ni empezar una
    // selección: se ignora hasta que se suelta el botón.
    if doc.ignore_click && !primary_down && !primary_released {
        doc.ignore_click = false;
    }

    let Some(origin) = doc.autoscroll_origin else {
        if middle {
            if let Some(p) = pos.filter(|p| panel_rect.contains(*p)) {
                doc.autoscroll_origin = Some(p);
                ctx.request_repaint();
            }
        }
        return;
    };

    if middle || primary || secondary || esc {
        doc.autoscroll_origin = None;
        doc.ignore_click = primary || secondary;
        ctx.request_repaint();
        return;
    }

    if let Some(p) = pos {
        let d = p - origin;
        let speed = |v: f32| {
            let beyond = (v.abs() - AUTOSCROLL_DEAD_ZONE).max(0.0);
            (beyond * AUTOSCROLL_GAIN).min(AUTOSCROLL_MAX_SPEED) * v.signum()
        };
        let (vx, vy) = (speed(d.x), speed(d.y));
        if vx != 0.0 || vy != 0.0 {
            doc.nudge_scroll(vx * dt, vy * dt);
        }
    }

    ctx.set_cursor_icon(CursorIcon::AllScroll);
    paint_autoscroll_marker(ctx, origin);
    ctx.request_repaint();
}

/// Marcador en el punto de partida del autoscroll: un círculo con cuatro flechas.
fn paint_autoscroll_marker(ctx: &Context, origin: egui::Pos2) {
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("autoscroll_marker")));
    painter.circle_filled(origin, 14.0, Color32::from_black_alpha(170));
    painter.circle_stroke(origin, 14.0, Stroke::new(1.5_f32, Color32::WHITE));
    painter.circle_filled(origin, 2.0, Color32::WHITE);
    for (dx, dy) in [(0.0_f32, -1.0_f32), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
        let tip = origin + vec2(dx, dy) * 10.0;
        let base = origin + vec2(dx, dy) * 5.5;
        let side = vec2(-dy, dx) * 3.5;
        painter.add(Shape::convex_polygon(vec![tip, base + side, base - side], Color32::WHITE, Stroke::NONE));
    }
}

fn continuous(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, viewport: Rect, jumped: bool) {
    // Sin espaciado automático: las posiciones de las páginas deben coincidir
    // exactamente con las que calcula `DocumentState::page_top`.
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    let count = doc.page_count();
    let max_w = (0..count).map(|i| doc.page_extent(i).x).fold(0.0_f32, f32::max);
    // El ancho del contenido nunca debe superar el ancho *realmente visible*
    // (`viewport`, que ya descuenta la barra vertical): si fuese unos píxeles
    // mayor, aparecería una barra horizontal permanente sin nada que desplazar.
    let content_w = (max_w + 2.0 * SIDE_MARGIN).max(viewport.width());

    ui.add_space(PAGE_GAP);
    let mut y = PAGE_GAP;
    // La "página actual" es la que más área visible ocupa (empates: la primera).
    let mut best = (doc.current_page, 0.0_f32);
    // Posición real (en esta maquetación) de la página a la que se acaba de saltar.
    let mut jump_top: Option<f32> = None;
    let jump_page = doc.jump.map(|(page, _)| page);

    for index in 0..count {
        let ext = doc.page_extent(index);
        let top = y;
        let bottom = y + ext.y;
        y = bottom + PAGE_GAP;

        if Some(index) == jump_page {
            jump_top = Some(top);
        }

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

    // Salto autocorregido: tras ir a una página, se comprueba con la posición
    // real de su borde superior que ha quedado arriba del todo; si no (por
    // redondeos, o porque el contenido aún no estaba completo), se reajusta.
    if let (Some((page, retries)), Some(top)) = (doc.jump, jump_top) {
        let desired = (top - PAGE_GAP).max(0.0);
        if (desired - viewport.min.y).abs() <= 1.5 || retries == 0 {
            doc.jump = None;
        } else {
            tracing::info!(
                "Reajustando el salto a la página {}: estaba en y = {:.0}, debía estar en {:.0}",
                page + 1,
                viewport.min.y,
                desired
            );
            doc.scroll_target.y = Some(desired);
            doc.jump = Some((page, retries - 1));
            ctx.request_repaint();
        }
    }

    if !jumped && doc.jump.is_none() && best.1 > 0.0 {
        doc.current_page = best.0;
    }
}

fn paginated(ui: &mut Ui, ctx: &Context, doc: &mut DocumentState, visible_w: f32) {
    ui.spacing_mut().item_spacing = Vec2::ZERO;

    let index = doc.current_page;
    let ext = doc.page_extent(index);
    // Igual que en el modo continuo: el ancho visible ya descuenta la barra vertical.
    let content_w = (ext.x + 2.0 * SIDE_MARGIN).max(visible_w);

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
        .rect_stroke(page_rect, 0.0, Stroke::new(1.0_f32, Color32::from_black_alpha(60)));

    doc.page_rects.push((index, page_rect));
    paint_overlays(ui, doc, index, page_rect);
}

/// Resaltado de búsqueda, selección de texto e hipervínculos clicables sobre una página.
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

    // Texto seleccionado
    if let Some(sel) = doc.selection {
        let (a, b) = sel.ordered();
        if index >= a.page && index <= b.page {
            let chars = doc.chars_for(index);
            let from = if index == a.page { a.caret } else { 0 };
            let to = if index == b.page { b.caret } else { chars.len() };
            if from < to {
                let color = theme::ACCENT.gamma_multiply(0.45);
                for r in chars.rects_for(from, to) {
                    ui.painter().rect_filled(to_screen(&r), 1.0, color);
                }
            }
        }
    }

    // Hipervínculos
    for (k, link) in doc.links_for(index).iter().enumerate() {
        let rect = to_screen(&link.rect);
        let id = egui::Id::new(("pdf-link", doc.id, index, k));
        let response = ui.interact(rect, id, Sense::click());
        if response.hovered() && !doc.ignore_click {
            doc.hovering_link = true;
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            ui.painter().rect_stroke(rect, 1.0, Stroke::new(1.0_f32, theme::ACCENT));
        }
        if response.clicked() && !doc.ignore_click {
            doc.pending_link = Some(link.target.clone());
        }
    }
}
