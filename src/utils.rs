//! Constantes y utilidades pequeñas compartidas.

pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 4.0;
/// Cada pulsación de "acercar/alejar" multiplica/divide el zoom por este factor.
pub const ZOOM_FACTOR: f32 = 1.2;

pub fn clamp_zoom(zoom: f32) -> f32 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}
/// Cuántos píxeles se desplaza el documento con las flechas arriba/abajo.
pub const SCROLL_STEP: f32 = 80.0;
