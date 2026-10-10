//! Texto de una página con la caja de cada carácter: base de la selección
//! y la copia de texto.
//!
//! Todas las coordenadas están en puntos PDF con el origen **arriba a la
//! izquierda** (como la pantalla), no abajo a la izquierda como en PDF.
//! Una "posición de cursor" (`caret`) es el hueco *entre* dos caracteres:
//! `0` es antes del primero y `len` después del último, igual que en un
//! editor de texto.

/// Caracteres de una página y su caja `[x0, y0, x1, y1]`.
#[derive(Debug, Default, Clone)]
pub struct PageChars {
    chars: Vec<char>,
    rects: Vec<[f32; 4]>,
    /// `false` para caracteres sin caja útil (saltos de línea, espacios que
    /// PDFium genera por su cuenta): no se pueden señalar con el ratón.
    solid: Vec<bool>,
}

impl PageChars {
    pub fn new(chars: Vec<char>, rects: Vec<[f32; 4]>, solid: Vec<bool>) -> Self {
        debug_assert!(chars.len() == rects.len() && chars.len() == solid.len());
        Self { chars, rects, solid }
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Una página sin caracteres seleccionables (por ejemplo, un escaneo).
    pub fn is_empty(&self) -> bool {
        !self.solid.iter().any(|&s| s)
    }

    /// Distancia (ponderada) del punto a la caja de un carácter. La vertical
    /// pesa más: así, al arrastrar entre dos líneas se elige la línea más
    /// cercana en vez de saltar a la palabra más próxima en horizontal.
    fn distance(rect: &[f32; 4], x: f32, y: f32) -> f32 {
        let dx = (rect[0] - x).max(0.0).max(x - rect[2]);
        let dy = (rect[1] - y).max(0.0).max(y - rect[3]);
        dx * dx + (dy * 3.0) * (dy * 3.0)
    }

    /// Índice del carácter más cercano al punto.
    pub fn nearest_char(&self, x: f32, y: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, rect) in self.rects.iter().enumerate() {
            if !self.solid[i] {
                continue;
            }
            let d = Self::distance(rect, x, y);
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((i, d));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Posición de cursor más cercana al punto: antes o después del carácter
    /// más próximo según el lado de su mitad en que caiga el punto.
    pub fn caret_at(&self, x: f32, y: f32) -> Option<usize> {
        let i = self.nearest_char(x, y)?;
        let r = &self.rects[i];
        Some(if x > (r[0] + r[2]) / 2.0 { i + 1 } else { i })
    }

    /// `true` si el punto está sobre (o muy cerca de) algún texto.
    pub fn is_over_text(&self, x: f32, y: f32) -> bool {
        const TOLERANCE: f32 = 2.0;
        self.rects.iter().zip(&self.solid).any(|(r, &solid)| {
            solid && x >= r[0] - TOLERANCE && x <= r[2] + TOLERANCE && y >= r[1] - TOLERANCE && y <= r[3] + TOLERANCE
        })
    }

    /// Rango de cursor `(inicio, fin)` de la palabra bajo el punto (doble clic).
    pub fn word_at(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let i = self.nearest_char(x, y)?;
        let is_word = |c: char| !c.is_whitespace() && c != '\u{FFFE}' && c != '\u{FFFF}';
        if !is_word(self.chars[i]) {
            return Some((i, i + 1));
        }
        let mut start = i;
        while start > 0 && is_word(self.chars[start - 1]) {
            start -= 1;
        }
        let mut end = i + 1;
        while end < self.chars.len() && is_word(self.chars[end]) {
            end += 1;
        }
        Some((start, end))
    }

    /// Rectángulos que cubren los caracteres `[from, to)`, uniendo los de una
    /// misma línea para dibujar pocas cajas en vez de una por letra.
    pub fn rects_for(&self, from: usize, to: usize) -> Vec<[f32; 4]> {
        let to = to.min(self.chars.len());
        let mut out: Vec<[f32; 4]> = Vec::new();
        let mut current: Option<[f32; 4]> = None;

        for i in from..to {
            if !self.solid[i] {
                continue;
            }
            let r = self.rects[i];
            current = match current {
                None => Some(r),
                Some(c) => {
                    let height = (c[3] - c[1]).max(1.0);
                    let overlap = c[3].min(r[3]) - c[1].max(r[1]);
                    let same_line = overlap > 0.5 * height.min((r[3] - r[1]).max(1.0));
                    // Se tolera un hueco de hasta ~1.5 alturas: es el espacio entre
                    // palabras, que PDFium no siempre dibuja como carácter con caja.
                    let adjacent = r[0] >= c[0] && r[0] - c[2] < height * 1.5;
                    if same_line && adjacent {
                        Some([c[0].min(r[0]), c[1].min(r[1]), c[2].max(r[2]), c[3].max(r[3])])
                    } else {
                        out.push(c);
                        Some(r)
                    }
                }
            };
        }
        out.extend(current);
        out
    }

    /// Texto de los caracteres `[from, to)`.
    pub fn text_of(&self, from: usize, to: usize) -> String {
        let to = to.min(self.chars.len());
        if from >= to {
            return String::new();
        }
        let raw: String = self.chars[from..to].iter().collect();
        raw.replace("\r\n", "\n").replace('\r', "\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "ab cd" en una línea, cada letra de 10 de ancho; el espacio sin caja.
    fn sample() -> PageChars {
        let chars = vec!['a', 'b', ' ', 'c', 'd'];
        let rects = vec![
            [0.0, 0.0, 10.0, 12.0],
            [10.0, 0.0, 20.0, 12.0],
            [0.0, 0.0, 0.0, 0.0],
            [30.0, 0.0, 40.0, 12.0],
            [40.0, 0.0, 50.0, 12.0],
        ];
        PageChars::new(chars, rects, vec![true, true, false, true, true])
    }

    #[test]
    fn caret_goes_before_or_after_by_half() {
        let p = sample();
        assert_eq!(p.caret_at(2.0, 6.0), Some(0));
        assert_eq!(p.caret_at(8.0, 6.0), Some(1));
        assert_eq!(p.caret_at(100.0, 6.0), Some(5));
    }

    #[test]
    fn double_click_selects_the_word() {
        let p = sample();
        assert_eq!(p.word_at(15.0, 6.0), Some((0, 2)));
        assert_eq!(p.word_at(45.0, 6.0), Some((3, 5)));
    }

    #[test]
    fn text_and_rects_for_a_range() {
        let p = sample();
        assert_eq!(p.text_of(0, 5), "ab cd");
        assert_eq!(p.text_of(3, 5), "cd");
        // "ab" y "cd" están a menos de 1.5 alturas: salen unidos en una caja.
        assert_eq!(p.rects_for(0, 5), vec![[0.0, 0.0, 50.0, 12.0]]);
        assert!(p.is_over_text(5.0, 5.0));
        assert!(!p.is_over_text(5.0, 200.0));
    }

    #[test]
    fn a_page_with_no_solid_chars_is_empty() {
        assert!(PageChars::default().is_empty());
        assert!(!sample().is_empty());
    }
}
