//! Componentes visuales. Cada submódulo dibuja una región y devuelve
//! "acciones" (enums) que `app.rs` interpreta; la UI no muta el estado
//! directamente.

pub mod icons;
pub mod recent_tab;
pub mod search_bar;
pub mod sidebar;
pub mod theme;
pub mod toolbar;
pub mod viewer;
