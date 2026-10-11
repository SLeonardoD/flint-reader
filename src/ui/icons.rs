//! Íconos dibujados como vectores con el `Painter` de egui.
//!
//! La fuente por defecto de egui no trae glifos como `＋`, `✕` o `▯`, y se
//! veían como cuadrados. Dibujarlos a mano elimina esa dependencia: se ven
//! igual en cualquier sistema y se adaptan solos al tema claro/oscuro.

use crate::ui::theme::ACCENT;
use egui::{pos2, vec2, Color32, Painter, Rect, Response, Sense, Shape, Stroke, Ui, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Open,
    Recent,
    Index,
    Search,
    Close,
    Prev,
    Next,
    Plus,
    Minus,
    Theme,
    Continuous,
    Paged,
    Menu,
}

pub const BUTTON_SIZE: Vec2 = vec2(30.0, 28.0);

/// Botón cuadrado con un ícono. `selected` lo dibuja resaltado (interruptor).
pub fn button(ui: &mut Ui, icon: Icon, tooltip: &str, selected: bool) -> Response {
    let enabled = ui.is_enabled();
    let (rect, response) = ui.allocate_exact_size(BUTTON_SIZE, Sense::click());

    if ui.is_rect_visible(rect) {
        let visuals = ui.visuals();
        let bg = if selected {
            ACCENT.gamma_multiply(0.40)
        } else if enabled && response.is_pointer_button_down_on() {
            visuals.widgets.active.bg_fill
        } else if enabled && response.hovered() {
            visuals.widgets.hovered.bg_fill
        } else {
            Color32::TRANSPARENT
        };
        let color = if !enabled {
            visuals.weak_text_color()
        } else if response.hovered() || selected {
            visuals.strong_text_color()
        } else {
            visuals.text_color()
        };
        ui.painter().rect_filled(rect, 5.0, bg);
        paint(ui.painter(), rect, icon, color);
    }
    response.on_hover_text(tooltip)
}

/// Ícono decorativo (sin interacción), p. ej. la lupa dentro de la búsqueda.
pub fn inline(ui: &mut Ui, icon: Icon) {
    let (rect, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
    paint(ui.painter(), rect, icon, ui.visuals().weak_text_color());
}

pub fn paint(p: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let r = 7.0_f32;
    let s = Stroke::new(1.7_f32, color);
    let pt = |x: f32, y: f32| pos2(c.x + x * r, c.y + y * r);

    match icon {
        Icon::Prev => {
            p.add(Shape::line(vec![pt(0.45, -1.0), pt(-0.45, 0.0), pt(0.45, 1.0)], s));
        }
        Icon::Next => {
            p.add(Shape::line(vec![pt(-0.45, -1.0), pt(0.45, 0.0), pt(-0.45, 1.0)], s));
        }
        Icon::Plus => {
            p.line_segment([pt(-0.9, 0.0), pt(0.9, 0.0)], s);
            p.line_segment([pt(0.0, -0.9), pt(0.0, 0.9)], s);
        }
        Icon::Minus => {
            p.line_segment([pt(-0.9, 0.0), pt(0.9, 0.0)], s);
        }
        Icon::Close => {
            p.line_segment([pt(-0.75, -0.75), pt(0.75, 0.75)], s);
            p.line_segment([pt(-0.75, 0.75), pt(0.75, -0.75)], s);
        }
        Icon::Search => {
            p.circle_stroke(pt(-0.2, -0.2), r * 0.62, s);
            p.line_segment([pt(0.27, 0.27), pt(0.95, 0.95)], s);
        }
        Icon::Menu => {
            for y in [-0.65, 0.0, 0.65] {
                p.line_segment([pt(-0.9, y), pt(0.9, y)], s);
            }
        }
        Icon::Open => {
            // Carpeta
            let pts = vec![
                pt(-1.0, -0.65),
                pt(-0.3, -0.65),
                pt(0.0, -0.3),
                pt(1.0, -0.3),
                pt(1.0, 0.75),
                pt(-1.0, 0.75),
            ];
            p.add(Shape::closed_line(pts, s));
        }
        Icon::Recent => {
            // Reloj
            p.circle_stroke(c, r * 0.95, s);
            p.line_segment([c, pt(0.0, -0.6)], s);
            p.line_segment([c, pt(0.45, 0.0)], s);
        }
        Icon::Index => {
            // Lista con viñetas
            for y in [-0.7, 0.0, 0.7] {
                p.circle_filled(pt(-0.8, y), 1.3, color);
                p.line_segment([pt(-0.3, y), pt(1.0, y)], s);
            }
        }
        Icon::Theme => {
            // Círculo con la mitad rellena (contraste claro/oscuro)
            let radius = r * 0.92;
            p.circle_stroke(c, radius, s);
            let pts: Vec<_> = (0..=12)
                .map(|i| {
                    let t = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * (i as f32 / 12.0);
                    pos2(c.x + radius * t.cos(), c.y + radius * t.sin())
                })
                .collect();
            p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        Icon::Continuous => {
            // Dos páginas apiladas
            p.rect_stroke(Rect::from_min_max(pt(-0.6, -1.0), pt(0.6, -0.1)), 1.0, s);
            p.rect_stroke(Rect::from_min_max(pt(-0.6, 0.1), pt(0.6, 1.0)), 1.0, s);
        }
        Icon::Paged => {
            // Una sola página
            p.rect_stroke(Rect::from_min_max(pt(-0.6, -1.0), pt(0.6, 1.0)), 1.0, s);
            p.line_segment([pt(-0.25, -0.35), pt(0.25, -0.35)], Stroke::new(1.2_f32, color));
            p.line_segment([pt(-0.25, 0.1), pt(0.25, 0.1)], Stroke::new(1.2_f32, color));
        }
    }
}
