//! Define los temas claro y oscuro de la aplicación. La paleta busca el
//! mismo espíritu que Foxit Reader: barras de herramientas planas y
//! neutras, un azul de acento para selección/resaltado, y el área de
//! lectura en un gris claramente diferenciado del resto de la interfaz.

use crate::config::ThemePreference;
use egui::{Color32, Stroke, Visuals};

/// Azul de acento usado para selección, enlaces activos y el resaltado
/// de búsqueda — el mismo rol de color que cumple el azul de Foxit.
pub const ACCENT: Color32 = Color32::from_rgb(0x2f, 0x7b, 0xe0);

pub fn apply(ctx: &egui::Context, pref: ThemePreference) {
    let mut visuals = match pref {
        ThemePreference::Dark => dark_visuals(),
        ThemePreference::Light => light_visuals(),
    };

    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.hyperlink_color = ACCENT;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.indent = 16.0;
    ctx.set_style(style);
}

fn dark_visuals() -> Visuals {
    let mut v = Visuals::dark();
    v.panel_fill = Color32::from_rgb(0x23, 0x25, 0x29);
    v.window_fill = Color32::from_rgb(0x2b, 0x2d, 0x32);
    // El área donde se muestra la página es más oscura que el resto de la
    // UI, igual que en Foxit: la página "flota" sobre un fondo neutro.
    v.extreme_bg_color = Color32::from_rgb(0x17, 0x18, 0x1b);
    v.widgets.noninteractive.bg_fill = Color32::from_rgb(0x2b, 0x2d, 0x32);
    v
}

fn light_visuals() -> Visuals {
    let mut v = Visuals::light();
    v.panel_fill = Color32::from_rgb(0xf2, 0xf3, 0xf5);
    v.window_fill = Color32::WHITE;
    v.extreme_bg_color = Color32::from_rgb(0xd7, 0xd9, 0xdc);
    v
}
