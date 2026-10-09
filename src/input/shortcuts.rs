//! Traduce las teclas presionadas en un frame a acciones de alto nivel.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutAction {
    NextPage,
    PrevPage,
    FirstPage,
    LastPage,
    ScrollUp,
    ScrollDown,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleSearch,
    OpenFile,
    CloseOverlay,
}

/// La navegación sin modificadores solo se activa si ningún campo de texto
/// tiene el foco (si no, robaríamos teclas mientras el usuario escribe).
pub fn poll(ctx: &egui::Context) -> Vec<ShortcutAction> {
    use egui::Key;
    let mut actions = Vec::new();
    let typing = ctx.wants_keyboard_input();

    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            actions.push(ShortcutAction::CloseOverlay);
        }

        let cmd = i.modifiers.command;
        if cmd && i.key_pressed(Key::F) {
            actions.push(ShortcutAction::ToggleSearch);
        }
        if cmd && i.key_pressed(Key::O) {
            actions.push(ShortcutAction::OpenFile);
        }
        if cmd && (i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals)) {
            actions.push(ShortcutAction::ZoomIn);
        }
        if cmd && i.key_pressed(Key::Minus) {
            actions.push(ShortcutAction::ZoomOut);
        }
        if cmd && i.key_pressed(Key::Num0) {
            actions.push(ShortcutAction::ZoomReset);
        }

        if !typing && !cmd {
            let shift = i.modifiers.shift;
            if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::PageDown) || (i.key_pressed(Key::Space) && !shift) {
                actions.push(ShortcutAction::NextPage);
            }
            if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::PageUp) || (i.key_pressed(Key::Space) && shift) {
                actions.push(ShortcutAction::PrevPage);
            }
            if i.key_pressed(Key::ArrowDown) {
                actions.push(ShortcutAction::ScrollDown);
            }
            if i.key_pressed(Key::ArrowUp) {
                actions.push(ShortcutAction::ScrollUp);
            }
            if i.key_pressed(Key::Home) {
                actions.push(ShortcutAction::FirstPage);
            }
            if i.key_pressed(Key::End) {
                actions.push(ShortcutAction::LastPage);
            }
        }
    });

    actions
}
