//! Selección de texto con el ratón: arrastrar selecciona, doble clic elige
//! una palabra, un clic suelto deselecciona. Ctrl+C copia (ver `App`).
//!
//! Se usa el estado crudo del puntero en vez de widgets de egui porque la
//! selección puede empezar en una página y terminar en otra, y porque el
//! arrastre tiene que seguir activo aunque el ratón salga de la página.

use crate::app::{DocumentState, DragMode, SelDrag, Selection, TextPos};
use egui::{pos2, Context, CursorIcon, PointerButton, Rect};

/// Píxeles que hay que mover el ratón para que un clic pase a ser arrastre.
const DRAG_THRESHOLD: f32 = 4.0;
/// Franja (arriba y abajo del visor) donde arrastrar desplaza el documento.
const EDGE_ZONE: f32 = 28.0;

pub fn handle(ctx: &Context, doc: &mut DocumentState, panel_rect: Rect) {
    if doc.autoscroll_origin.is_some() {
        return;
    }

    let (pressed, secondary, any_pressed, esc, down, double_click, pos, delta, dt) = ctx.input(|i| {
        (
            i.pointer.button_pressed(PointerButton::Primary),
            i.pointer.button_pressed(PointerButton::Secondary),
            i.pointer.any_pressed(),
            i.key_pressed(egui::Key::Escape),
            i.pointer.primary_down(),
            i.pointer.button_double_clicked(PointerButton::Primary),
            i.pointer.hover_pos().or_else(|| i.pointer.latest_pos()),
            i.pointer.delta(),
            i.stable_dt.min(0.1),
        )
    });
    let Some(pos) = pos else {
        return;
    };

    // Zona del visor sin la barra de desplazamiento, y sin menús flotantes encima.
    let selectable = Rect::from_min_max(panel_rect.min, pos2(panel_rect.max.x - 14.0, panel_rect.max.y));
    let over_viewer = selectable.contains(pos)
        && ctx
            .layer_id_at(pos)
            .is_none_or(|layer| layer.order == egui::Order::Background);

    // Menú contextual: clic derecho sobre texto seleccionado -> "Copiar".
    let mut just_opened = false;
    if secondary && over_viewer && !doc.ignore_click {
        doc.ctx_menu = None;
        if over_selection(doc, pos) {
            doc.ctx_menu = Some(pos);
            just_opened = true;
            ctx.request_repaint();
        }
    }
    if doc.ctx_menu.is_some() {
        copy_menu(ctx, doc, any_pressed && !just_opened, esc);
    }

    // Cursor de texto sobre texto seleccionable (los enlaces tienen su propio cursor).
    if over_viewer && doc.sel_drag.is_none() && !doc.hovering_link {
        if let Some((page, pt)) = doc.page_at(pos) {
            if doc.chars_for(page).is_over_text(pt.x, pt.y) {
                ctx.set_cursor_icon(CursorIcon::Text);
            }
        }
    }

    // Doble clic: seleccionar la palabra.
    if double_click && over_viewer && !doc.ignore_click {
        if let Some((page, pt)) = doc.page_at(pos) {
            if let Some((start, end)) = doc.chars_for(page).word_at(pt.x, pt.y) {
                doc.selection = Some(Selection {
                    anchor: TextPos { page, caret: start },
                    head: TextPos { page, caret: end },
                });
                doc.sel_drag = None;
                ctx.request_repaint();
                return;
            }
        }
    }

    // Pulsación: empieza un posible arrastre.
    if pressed && over_viewer && !doc.ignore_click {
        doc.sel_drag = None;
        match doc.page_at(pos) {
            Some((page, pt)) => {
                let chars = doc.chars_for(page);
                // Sin texto (un escaneo, por ejemplo): arrastrar mueve el documento.
                let (mode, caret) = if chars.is_empty() {
                    (DragMode::Pan, 0)
                } else {
                    (DragMode::Select, chars.caret_at(pt.x, pt.y).unwrap_or(0))
                };
                doc.sel_drag = Some(SelDrag {
                    start: pos,
                    mode,
                    moved: false,
                    anchor: TextPos { page, caret },
                });
            }
            // Clic fuera de las páginas: se quita la selección.
            None => doc.selection = None,
        }
    }

    // Arrastre en curso, o fin del mismo.
    if let Some(mut drag) = doc.sel_drag {
        if down {
            if !drag.moved && pos.distance(drag.start) > DRAG_THRESHOLD {
                drag.moved = true;
            }
            if drag.moved {
                match drag.mode {
                    DragMode::Pan => doc.nudge_scroll(-delta.x, -delta.y),
                    DragMode::Select => {
                        if let Some(head) = doc.text_pos_at(pos) {
                            doc.selection = Some(Selection { anchor: drag.anchor, head });
                        }
                        // Cerca del borde superior/inferior, el documento se desplaza solo.
                        let into_top = (panel_rect.min.y + EDGE_ZONE - pos.y).max(0.0);
                        let into_bottom = (pos.y - (panel_rect.max.y - EDGE_ZONE)).max(0.0);
                        let speed = (into_bottom - into_top).clamp(-120.0, 120.0) * 12.0;
                        if speed != 0.0 {
                            doc.nudge_scroll(0.0, speed * dt);
                        }
                        ctx.set_cursor_icon(CursorIcon::Text);
                    }
                }
                ctx.request_repaint();
            }
            doc.sel_drag = Some(drag);
        } else {
            // Un clic sin arrastre quita la selección.
            if !drag.moved && drag.mode == DragMode::Select {
                doc.selection = None;
            }
            doc.sel_drag = None;
            ctx.request_repaint();
        }
    }
}

/// `true` si `pos` (en pantalla) cae sobre texto que forma parte de la selección.
fn over_selection(doc: &mut DocumentState, pos: egui::Pos2) -> bool {
    let Some(sel) = doc.selection else {
        return false;
    };
    let Some((page, pt)) = doc.page_at(pos) else {
        return false;
    };
    let (a, b) = sel.ordered();
    if page < a.page || page > b.page {
        return false;
    }
    let chars = doc.chars_for(page);
    let from = if page == a.page { a.caret } else { 0 };
    let to = if page == b.page { b.caret } else { chars.len() };
    // Con tolerancia: se apunta al bloque azul, no a cada letra, y el hueco entre
    // líneas también cuenta (exigir caer justo dentro de la caja de una letra
    // haría que el menú "no saliera" por una diferencia de medio punto).
    const PAD_X: f32 = 3.0;
    const PAD_Y: f32 = 4.0;
    chars.rects_for(from, to).iter().any(|r| {
        pt.x >= r[0] - PAD_X && pt.x <= r[2] + PAD_X && pt.y >= r[1] - PAD_Y && pt.y <= r[3] + PAD_Y
    })
}

/// Pequeño menú flotante con una sola opción, "Copiar". Se cierra al elegirla,
/// con `Esc`, o al pulsar en cualquier otro sitio.
fn copy_menu(ctx: &Context, doc: &mut DocumentState, close_on_outside_press: bool, esc: bool) {
    let Some(menu_pos) = doc.ctx_menu else {
        return;
    };
    let mut close = esc;

    let area = egui::Area::new(egui::Id::new("flint-copy-menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(menu_pos)
        .show(ctx, |ui| {
            egui::Frame::menu(ui.style()).show(ui, |ui| {
                let copy = egui::Button::new("Copiar").shortcut_text("Ctrl+C").frame(false);
                if ui.add(copy).clicked() {
                    doc.copy_requested = true;
                    close = true;
                    ctx.request_repaint();
                }
            });
        });

    if close_on_outside_press {
        let outside = ctx
            .input(|i| i.pointer.interact_pos())
            .is_some_and(|p| !area.response.rect.contains(p));
        close |= outside;
    }
    if close {
        doc.ctx_menu = None;
        ctx.request_repaint();
    }
}
