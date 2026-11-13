//! Disegno di piccoli multipli e del ciclo solare in bitmap RGB.
//!
//! I grafici dell'atlante sono centinaia di linee e vengono ridisegnati a ogni
//! fotogramma durante la riproduzione: disegnarli in una bitmap una volta sola
//! e aggiornare solo cursore e pallini tiene l'interfaccia fluida.

pub const SPARK_W: usize = 196;
pub const SPARK_H: usize = 58;

pub const SOLAR_W: usize = 1500;
pub const SOLAR_H: usize = 236;
pub const SOLAR_PAD_L: f64 = 48.0;
pub const SOLAR_PAD_R: f64 = 50.0;
pub const SOLAR_PAD_T: f64 = 16.0;
pub const SOLAR_PAD_B: f64 = 24.0;
pub const SOLAR_YEAR_MIN: f64 = 1964.0;
pub const SOLAR_YEAR_MAX: f64 = 2027.0;
pub const SOLAR_OULU_MIN: f64 = -14.0;
pub const SOLAR_OULU_MAX: f64 = 14.0;
pub const SOLAR_SSN_MAX: f64 = 300.0;

struct Canvas24 {
    w: usize,
    h: usize,
    px: Vec<u8>,
}

impl Canvas24 {
    fn new(w: usize, h: usize, bg: (u8, u8, u8)) -> Self {
        let mut px = Vec::with_capacity(w * h * 3);
        for _ in 0..w * h {
            px.extend_from_slice(&[bg.0, bg.1, bg.2]);
        }
        Self { w, h, px }
    }

    fn set(&mut self, x: i32, y: i32, color: (u8, u8, u8)) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let at = (y as usize * self.w + x as usize) * 3;
        self.px[at] = color.0;
        self.px[at + 1] = color.1;
        self.px[at + 2] = color.2;
    }


    fn hline(&mut self, x0: i32, x1: i32, y: i32, color: (u8, u8, u8)) {
        for x in x0.min(x1)..=x0.max(x1) {
            self.set(x, y, color);
        }
    }

    fn vline(&mut self, x: i32, y0: i32, y1: i32, color: (u8, u8, u8)) {
        for y in y0.min(y1)..=y0.max(y1) {
            self.set(x, y, color);
        }
    }

    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: (u8, u8, u8), thickness: f64) {
        let (mut x0, mut y0) = (x0.round() as i32, y0.round() as i32);
        let (x1, y1) = (x1.round() as i32, y1.round() as i32);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if thickness <= 1.2 {
                self.set(x0, y0, color);
            } else {
                let r = (thickness / 2.0).round() as i32;
                for oy in -r..=r {
                    for ox in -r..=r {
                        if ox * ox + oy * oy <= r * r {
                            self.set(x0 + ox, y0 + oy, color);
                        }
                    }
                }
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: (u8, u8, u8)) {
        for yy in y.max(0)..(y + h).min(self.h as i32) {
            self.hline(x, x + w - 1, yy, color);
        }
    }

    /// Scrive una stringa col font 3x5, ingrandita di `scale`.
    fn text(&mut self, x: i32, y: i32, text: &str, scale: i32, color: (u8, u8, u8)) {
        let mut cursor = x;
        for c in text.chars() {
            if let Some(rows) = glyph(c) {
                for (row_index, row) in rows.iter().enumerate() {
                    for col in 0..3 {
                        if row & (1 << (2 - col)) != 0 {
                            let px = cursor + col * scale;
                            let py = y + row_index as i32 * scale;
                            for oy in 0..scale {
                                for ox in 0..scale {
                                    self.set(px + ox, py + oy, color);
                                }
                            }
                        }
                    }
                }
            }
            cursor += 4 * scale;
        }
    }

    /// Come `text`, ma centrata orizzontalmente su `center_x`.
    fn text_centered(&mut self, center_x: i32, y: i32, text: &str, scale: i32, color: (u8, u8, u8)) {
        self.text(center_x - text_width(text, scale) / 2, y, text, scale, color);
    }

    /// Come `text`, ma allineata a destra su `right_x`.
    fn text_right(&mut self, right_x: i32, y: i32, text: &str, scale: i32, color: (u8, u8, u8)) {
        self.text(right_x - text_width(text, scale), y, text, scale, color);
    }

    fn to_bmp(&self) -> Vec<u8> {
        let row_bytes = (self.w * 3 + 3) & !3;
        let data_len = row_bytes * self.h;
        let mut out = Vec::with_capacity(54 + data_len);
        let le32 = |v: u32| v.to_le_bytes();
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&le32((54 + data_len) as u32));
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&le32(54));
        out.extend_from_slice(&le32(40));
        out.extend_from_slice(&le32(self.w as u32));
        out.extend_from_slice(&le32(self.h as u32));
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        for row in (0..self.h).rev() {
            let start = out.len();
            for col in 0..self.w {
                let at = (row * self.w + col) * 3;
                // BMP vuole BGR
                out.extend_from_slice(&[self.px[at + 2], self.px[at + 1], self.px[at]]);
            }
            while out.len() - start < row_bytes {
                out.push(0);
            }
        }
        out
    }
}

fn bg() -> (u8, u8, u8) {
    (14, 21, 29)
}

/// Font minimo 3x5 per le etichette numeriche degli assi (niente testo, solo
/// cifre e pochi segni: il resto delle didascalie sta nell'interfaccia).
fn glyph(c: char) -> Option<[u8; 5]> {
    Some(match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        ' ' => [0b000; 5],
        _ => return None,
    })
}

/// Larghezza in pixel di una scritta con `text` alla scala data.
fn text_width(text: &str, scale: i32) -> i32 {
    let chars = text.chars().count() as i32;
    if chars == 0 {
        0
    } else {
        chars * 4 * scale - scale
    }
}

/// Piccolo multiplo di una stazione: variazione percentuale nel tempo.
pub fn sparkline(pcts: &[Option<f64>], scale: f64) -> Vec<u8> {
    let mut canvas = Canvas24::new(SPARK_W, SPARK_H, bg());
    let top = 4.0;
    let bottom = SPARK_H as f64 - 4.0;
    let span = (bottom - top).max(1.0);
    let y_of = |value: f64| -> f64 {
        let norm = (value / scale).clamp(-1.0, 1.0);
        top + span * (1.0 - (norm + 1.0) / 2.0)
    };
    let grid = (27, 39, 51);
    canvas.hline(0, SPARK_W as i32 - 1, y_of(0.0) as i32, (45, 60, 76));
    for level in [-0.5, 0.5] {
        canvas.hline(0, SPARK_W as i32 - 1, y_of(scale * level) as i32, grid);
    }
    let mut min_value = 0.0_f64;
    let mut max_value = 0.0_f64;
    for value in pcts.iter().flatten() {
        min_value = min_value.min(*value);
        max_value = max_value.max(*value);
    }
    let color = if min_value < -4.0 {
        (56, 189, 248)
    } else if max_value > 4.0 {
        (251, 191, 36)
    } else {
        (94, 234, 212)
    };
    let n = pcts.len();
    if n >= 2 {
        let mut previous: Option<(f64, f64)> = None;
        for (i, value) in pcts.iter().enumerate() {
            let x = i as f64 / (n - 1) as f64 * (SPARK_W as f64 - 1.0);
            match value {
                Some(value) => {
                    let y = y_of(*value);
                    if let Some((px, py)) = previous {
                        canvas.line(px, py, x, y, color, 1.4);
                    }
                    previous = Some((x, y));
                }
                None => previous = None,
            }
        }
    }
    canvas.to_bmp()
}

/// Ciclo solare: barre del numero di macchie (scala destra) e variazione di
/// Oulu (scala sinistra), dal 1964 a oggi. I colori restano tenui perché il
/// pannello deve convivere con il resto della dashboard.
pub fn solar_bmp(months: &[crate::atlas::SolarMonth]) -> Vec<u8> {
    let mut canvas = Canvas24::new(SOLAR_W, SOLAR_H, bg());
    let inner_w = SOLAR_W as f64 - SOLAR_PAD_L - SOLAR_PAD_R;
    let inner_h = SOLAR_H as f64 - SOLAR_PAD_T - SOLAR_PAD_B;
    let x_of = |year: f64| {
        SOLAR_PAD_L + inner_w * (year - SOLAR_YEAR_MIN) / (SOLAR_YEAR_MAX - SOLAR_YEAR_MIN)
    };
    let y_oulu = |value: f64| {
        SOLAR_PAD_T + inner_h * (1.0 - (value - SOLAR_OULU_MIN) / (SOLAR_OULU_MAX - SOLAR_OULU_MIN))
    };
    let y_ssn = |value: f64| {
        SOLAR_PAD_T + inner_h * (1.0 - (value / SOLAR_SSN_MAX).clamp(0.0, 1.0))
    };
    let grid = (27, 39, 51);
    let label_color = (110, 125, 142);
    for level in [-12.0, -8.0, -4.0, 0.0, 4.0, 8.0, 12.0] {
        canvas.hline(SOLAR_PAD_L as i32,
                     (SOLAR_W as f64 - SOLAR_PAD_R) as i32,
                     y_oulu(level) as i32, grid);
        canvas.text_right(SOLAR_PAD_L as i32 - 6,
                          y_oulu(level) as i32 - 5,
                          &format!("{level:+.0}"),
                          2, label_color);
    }
    // asse destro: numero di macchie solari
    for level in [0.0, 100.0, 200.0] {
        canvas.text((SOLAR_W as f64 - SOLAR_PAD_R) as i32 + 7,
                    y_ssn(level) as i32 - 5,
                    &format!("{level:.0}"),
                    2, label_color);
    }
    // asse orizzontale: anni
    let mut year = SOLAR_YEAR_MIN as i64;
    while (year as f64) <= SOLAR_YEAR_MAX - 5.0 {
        let x = x_of(year as f64) as i32;
        canvas.vline(x, (SOLAR_PAD_T + inner_h) as i32,
                     (SOLAR_PAD_T + inner_h) as i32 + 4, grid);
        canvas.text_centered(x, (SOLAR_PAD_T + inner_h) as i32 + 8,
                             &format!("{year}"), 2, label_color);
        year += 8;
    }
    // barre delle macchie solari, media di 3 mesi per non affollare
    let bar_color = (82, 60, 24);
    let start = SOLAR_PAD_L as i32;
    let end = (SOLAR_W as f64 - SOLAR_PAD_R) as i32;
    let total_w = (end - start).max(1) as f64;
    let slot = total_w / (SOLAR_YEAR_MAX - SOLAR_YEAR_MIN) / 12.0;
    let bar_w = (slot * 0.9).max(1.0);
    let mut i = 0;
    while i + 2 < months.len() {
        let mut sum = 0.0;
        let mut count = 0;
        for month in &months[i..i + 3] {
            if let Some(value) = month.ssn {
                sum += value;
                count += 1;
            }
        }
        if count > 0 {
            let value = sum / count as f64;
            let x = x_of(months[i].year);
            let y = y_ssn(value);
            canvas.rect(x as i32, y as i32, bar_w as i32,
                        (SOLAR_PAD_T + inner_h - y) as i32, bar_color);
        }
        i += 3;
    }
    // Oulu
    let mut previous: Option<(f64, f64)> = None;
    for month in months {
        if let Some(value) = month.oulu {
            let x = x_of(month.year);
            let y = y_oulu(value.clamp(SOLAR_OULU_MIN, SOLAR_OULU_MAX));
            if let Some((px, py)) = previous {
                canvas.line(px, py, x, y, (34, 211, 238), 1.6);
            }
            previous = Some((x, y));
        } else {
            previous = None;
        }
    }
    canvas.to_bmp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_valido() {
        let bmp = sparkline(&[Some(-2.0), None, Some(3.0)], 10.0);
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), 54 + SPARK_W * SPARK_H * 3);
        assert_eq!(u32::from_le_bytes([bmp[18], bmp[19], bmp[20], bmp[21]]), SPARK_W as u32);
    }

    #[test]
    fn cifre_del_font() {
        assert_eq!(glyph('7'), Some([0b111, 0b001, 0b001, 0b001, 0b001]));
        assert_eq!(glyph('é'), None);
        assert_eq!(text_width("1964", 2), 30);
        assert_eq!(text_width("", 2), 0);
    }

    #[test]
    fn disegno_righe_e_colonne() {
        let mut canvas = Canvas24::new(10, 10, (0, 0, 0));
        canvas.line(0.0, 0.0, 9.0, 9.0, (255, 255, 255), 1.0);
        canvas.set(5, 5, (255, 0, 0));
        let bmp = canvas.to_bmp();
        // 10 pixel da 3 byte = 30, arrotondati a 32 per riga
        assert_eq!(bmp.len(), 54 + 32 * 10);
    }
}
