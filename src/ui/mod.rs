//! Todos los componentes visuales de la aplicación. Cada submódulo dibuja
//! una región de la pantalla y devuelve una lista de "acciones" (enums)
//! que `app.rs` interpreta y aplica al estado — la UI nunca muta el
//! estado directamente, salvo los campos puramente visuales (texto del
//! cuadro de búsqueda, etc.) que se pasan por referencia mutable.

pub mod recent_tab;
pub mod search_bar;
pub mod sidebar;
pub mod theme;
pub mod toolbar;
pub mod viewer;
