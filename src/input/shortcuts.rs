//! Traduce las teclas presionadas en un frame a acciones de alto nivel.
//! Centralizar esto aquí evita esparcir `ctx.input(...)` por toda la UI.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    NextPage,
    PrevPage,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleSearch,
    OpenFile,
    CloseOverlay,
}

/// Revisa el input del frame actual y devuelve las acciones disparadas.
///
/// La navegación con flechas/espacio solo se activa si ningún widget de
/// texto tiene el foco (si no, estaríamos robando esas teclas mientras el
/// usuario escribe en el buscador o en el campo de "ir a página").
pub fn poll(ctx: &egui::Context) -> Vec<ShortcutAction> {
    let mut actions = Vec::new();
    let typing = ctx.wants_keyboard_input();

    ctx.input(|i| {
        if i.key_pressed(egui::Key::Escape) {
            actions.push(ShortcutAction::CloseOverlay);
        }

        if i.modifiers.command && i.key_pressed(egui::Key::F) {
            actions.push(ShortcutAction::ToggleSearch);
        }
        if i.modifiers.command && i.key_pressed(egui::Key::O) {
            actions.push(ShortcutAction::OpenFile);
        }
        if i.modifiers.command && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
            actions.push(ShortcutAction::ZoomIn);
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Minus) {
            actions.push(ShortcutAction::ZoomOut);
        }
        if i.modifiers.command && i.key_pressed(egui::Key::Num0) {
            actions.push(ShortcutAction::ZoomReset);
        }

        if !typing {
            if i.key_pressed(egui::Key::ArrowRight)
                || i.key_pressed(egui::Key::PageDown)
                || i.key_pressed(egui::Key::Space)
            {
                actions.push(ShortcutAction::NextPage);
            }
            if i.key_pressed(egui::Key::ArrowLeft) || i.key_pressed(egui::Key::PageUp) {
                actions.push(ShortcutAction::PrevPage);
            }
        }
    });

    actions
}
