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
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
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

// ---------------------------------------------------------------------------
// Heatmap stazioni × tempo
// ---------------------------------------------------------------------------

pub const HEAT_W: usize = 1180;
pub const HEAT_ROW: usize = 15;
pub const HEAT_PAD_L: f64 = 84.0;
pub const HEAT_PAD_B: usize = 18;

/// Altezza in pixel della heatmap per un dato numero di righe.
pub fn heatmap_height(rows: usize) -> usize {
    4 + rows * HEAT_ROW + HEAT_PAD_B
}

/// Colore di una cella della heatmap: lo zero resta quasi sullo sfondo, così si
/// vedono solo le deviazioni; blu il calo, giallo e rosso l'aumento.
fn heat_color(value: f64, scale: f64) -> (u8, u8, u8) {
    let norm = (value / scale.max(0.1)).clamp(-1.0, 1.0);
    let base = (16u8, 24u8, 33u8);
    let target: (u8, u8, u8) = if norm < 0.0 {
        (56, 189, 248)
    } else if norm < 0.55 {
        (251, 191, 36)
    } else {
        (248, 113, 113)
    };
    let t = norm.abs().powf(0.65);
    let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
    (mix(base.0, target.0), mix(base.1, target.1), mix(base.2, target.2))
}

/// Una riga per stazione (ordinate per rigidità), una colonna per passo: il
/// colore è la variazione percentuale. È la vista della propagazione: il fronte
/// dell'evento si vede scendere dalle righe polari a quelle equatoriali.
pub fn heatmap_bmp(dataset: &crate::atlas::Dataset, rows: &[usize]) -> Vec<u8> {
    let height = heatmap_height(rows.len());
    let mut canvas = Canvas24::new(HEAT_W, height, bg());
    let steps = dataset.steps.max(1);
    let left = HEAT_PAD_L;
    let right = HEAT_W as f64 - 8.0;
    let cell = ((right - left) / steps as f64).max(0.5);
    let grid = (27, 39, 51);
    let label = (186, 200, 214);
    let code_color = (226, 232, 240);

    for (row, station) in rows.iter().enumerate() {
        let top = (4 + row * HEAT_ROW) as i32;
        let info = &crate::atlas::STATIONS[*station];
        canvas.text(4, top + 2, &format!("{} {:.1}", info.code, info.rc), 2, code_color);
        canvas.hline(0, HEAT_W as i32 - 1, top + HEAT_ROW as i32 - 1, (20, 29, 39));
        let Some(column) = dataset.col_of(*station) else {
            continue;
        };
        for step in 0..steps {
            let color = match dataset.pct_at(step, column) {
                // la heatmap usa una scala più larga dei grafici: l'evento
                // estremo deve restare leggibile, non diventare un blocco
                Some(value) => heat_color(value, dataset.scale_pct * 1.8),
                None => (26, 34, 43),
            };
            let x0 = (left + step as f64 * cell).round() as i32;
            let x1 = (left + (step + 1) as f64 * cell).round() as i32;
            canvas.rect(x0, top, (x1 - x0).max(1), HEAT_ROW as i32 - 1, color);
        }
    }

    // asse del tempo, in ore dall'inizio della finestra
    let step_hours = dataset.step as f64 / 3600.0;
    let total = steps as f64 * step_hours;
    let every = if step_hours <= 0.2 { 12.0 } else { 24.0 };
    let axis_y = (4 + rows.len() * HEAT_ROW + 4) as i32;
    canvas.hline(left as i32, right as i32, axis_y - 3, grid);
    let mut hour = 0.0;
    while hour <= total + 0.01 {
        let x = (left + hour / step_hours * cell).round() as i32;
        canvas.vline(x, axis_y - 6, axis_y - 3, grid);
        canvas.text_centered(x, axis_y, &format!("{hour:.0}h"), 2, label);
        hour += every;
    }
    canvas.to_bmp()
}

// ---------------------------------------------------------------------------
// Confronto fra eventi
// ---------------------------------------------------------------------------

pub const EVENTS_W: usize = HEAT_W;
pub const EVENTS_H: usize = 320;
const EVENTS_PAD_L: f64 = 46.0;
const EVENTS_PAD_R: f64 = 12.0;
const EVENTS_PAD_T: f64 = 12.0;
const EVENTS_PAD_B: f64 = 58.0;
const EVENTS_HOURS: f64 = 24.0;

/// Colore della curva dell'evento `index` (l'ordine è quello di `EVENTS`).
pub fn event_color(index: usize) -> (u8, u8, u8) {
    const GLE: [(u8, u8, u8); 4] = [
        (251, 191, 36),
        (244, 114, 182),
        (248, 113, 113),
        (253, 224, 71),
    ];
    const FORBUSH: [(u8, u8, u8); 5] = [
        (56, 189, 248),
        (94, 234, 212),
        (129, 140, 248),
        (34, 211, 238),
        (45, 212, 191),
    ];
    let gle_count = GLE.len();
    if index < gle_count {
        GLE[index]
    } else {
        FORBUSH[(index - gle_count).min(FORBUSH.len() - 1)]
    }
}

/// Tutti gli eventi sovrapposti: mediana della rete contro ore dal picco.
/// I GLE sono picchi stretti, i Forbush depressioni larghe: la differenza di
/// forma è il punto del grafico.
pub fn events_bmp(curves: &[crate::atlas::EventCurve]) -> Vec<u8> {
    let mut max = 1.0_f64;
    for curve in curves {
        for (_, value) in &curve.points {
            max = max.max(value.abs());
        }
    }
    let max = (max * 1.12).clamp(2.0, 40.0);
    let mut canvas = Canvas24::new(EVENTS_W, EVENTS_H, bg());
    let left = EVENTS_PAD_L;
    let right = EVENTS_W as f64 - EVENTS_PAD_R;
    let top = EVENTS_PAD_T;
    let bottom = EVENTS_H as f64 - EVENTS_PAD_B;
    let x_of = |hour: f64| left + (right - left) * (hour + EVENTS_HOURS) / (2.0 * EVENTS_HOURS);
    let y_of = |value: f64| {
        top + (bottom - top) * (1.0 - (value / max).clamp(-1.0, 1.0)) / 2.0
    };
    let grid = (27, 39, 51);
    let label = (110, 125, 142);

    for level in [max, max / 2.0, 0.0, -max / 2.0, -max] {
        canvas.hline(left as i32, right as i32, y_of(level) as i32, grid);
        canvas.text_right(left as i32 - 6, y_of(level) as i32 - 5,
                          &format!("{level:+.1}"), 2, label);
    }
    for hour in [-24.0, -12.0, 0.0, 12.0, 24.0] {
        let x = x_of(hour) as i32;
        canvas.vline(x, top as i32, bottom as i32, if hour == 0.0 { (45, 60, 76) } else { grid });
        canvas.text_centered(x, bottom as i32 + 7, &format!("{hour:+.0}"), 2, label);
    }

    for (index, curve) in curves.iter().enumerate() {
        let color = event_color(index);
        let mut previous: Option<(f64, f64)> = None;
        for (hour, value) in &curve.points {
            let point = (x_of(*hour), y_of(*value));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, color, 1.6);
            }
            previous = Some(point);
        }
    }

    // legenda dentro la stessa bitmap: così i colori sono per costruzione
    // quelli delle curve, senza dipendere dal riconciliatore dell'interfaccia
    let columns = curves.len().max(1);
    let slot = (right - left) / columns as f64;
    let legend_y = (EVENTS_H as f64 - 26.0) as i32;
    for (index, curve) in curves.iter().enumerate() {
        let x = left + slot * index as f64;
        canvas.rect(x as i32, legend_y + 3, 10, 4, event_color(index));
        canvas.text(x as i32 + 14, legend_y, &curve.short, 2, (186, 200, 214));
    }
    canvas.to_bmp()
}

// ---------------------------------------------------------------------------
// Spettro di rigidità
// ---------------------------------------------------------------------------

pub const SPEC_W: usize = 560;
pub const SPEC_H: usize = 250;
const SPEC_PAD_L: f64 = 40.0;
const SPEC_PAD_R: f64 = 10.0;
const SPEC_PAD_T: f64 = 10.0;
const SPEC_PAD_B: f64 = 22.0;
const SPEC_RC_MIN: f64 = 0.5;
const SPEC_RC_MAX: f64 = 20.0;

/// Pendenza del fit Δ = a + b·ln(Rc): `gamma = -b` è l'esponente dello spettro
/// (positivo = le stazioni polari vedono di più, come nei GLE).
pub struct Fit {
    pub a: f64,
    pub b: f64,
    pub r2: f64,
}

pub fn spectrum_fit(points: &[(f64, f64)]) -> Option<Fit> {
    let usable: Vec<(f64, f64)> = points
        .iter()
        .copied()
        .filter(|(rc, value)| *rc > 0.0 && rc.is_finite() && value.is_finite())
        .map(|(rc, value)| (rc.ln(), value))
        .collect();
    if usable.len() < 3 {
        return None;
    }
    let n = usable.len() as f64;
    let mean_x = usable.iter().map(|p| p.0).sum::<f64>() / n;
    let mean_y = usable.iter().map(|p| p.1).sum::<f64>() / n;
    let mut sxx = 0.0;
    let mut sxy = 0.0;
    let mut syy = 0.0;
    for (x, y) in &usable {
        let dx = x - mean_x;
        let dy = y - mean_y;
        sxx += dx * dx;
        sxy += dx * dy;
        syy += dy * dy;
    }
    if sxx <= f64::EPSILON {
        return None;
    }
    let b = sxy / sxx;
    let a = mean_y - b * mean_x;
    let r2 = if syy <= f64::EPSILON {
        1.0
    } else {
        (sxy * sxy) / (sxx * syy)
    };
    Some(Fit { a, b, r2 })
}

/// Δ% contro rigidità di taglio all'istante scelto, con la retta del fit.
pub fn spectrum_bmp(points: &[(f64, f64)], scale: f64, fit: Option<&Fit>) -> Vec<u8> {
    let mut canvas = Canvas24::new(SPEC_W, SPEC_H, bg());
    let left = SPEC_PAD_L;
    let right = SPEC_W as f64 - SPEC_PAD_R;
    let top = SPEC_PAD_T;
    let bottom = SPEC_H as f64 - SPEC_PAD_B;
    let log_min = SPEC_RC_MIN.ln();
    let log_max = SPEC_RC_MAX.ln();
    let x_of = |rc: f64| left + (right - left) * (rc.ln() - log_min) / (log_max - log_min);
    let y_of = |value: f64| {
        top + (bottom - top) * (1.0 - (value / scale).clamp(-1.0, 1.0)) / 2.0
    };
    let grid = (27, 39, 51);
    let label = (110, 125, 142);

    for rc in [1.0, 2.0, 5.0, 10.0, 20.0] {
        let x = x_of(rc) as i32;
        canvas.vline(x, top as i32, bottom as i32, grid);
        canvas.text_centered(x, bottom as i32 + 7, &format!("{rc:.0}"), 2, label);
    }
    for level in [scale, scale / 2.0, 0.0, -scale / 2.0, -scale] {
        canvas.hline(left as i32, right as i32, y_of(level) as i32, grid);
        canvas.text_right(left as i32 - 6, y_of(level) as i32 - 5,
                          &format!("{level:+.0}"), 2, label);
    }

    if let Some(fit) = fit {
        let color = (148, 163, 184);
        let mut previous = None;
        for step in 0..=60 {
            let rc = SPEC_RC_MIN * (SPEC_RC_MAX / SPEC_RC_MIN).powf(step as f64 / 60.0);
            let value = (fit.a + fit.b * rc.ln()).clamp(-scale, scale);
            let point = (x_of(rc), y_of(value));
            if let Some((px, py)) = previous {
                canvas.line(px, py, point.0, point.1, color, 1.0);
            }
            previous = Some(point);
        }
    }

    for (rc, value) in points {
        if *rc < SPEC_RC_MIN || *rc > SPEC_RC_MAX || !value.is_finite() {
            continue;
        }
        let x = x_of(*rc) as i32;
        let y = y_of(*value) as i32;
        let color = crate::atlas::variation_color(*value);
        canvas.rect(x - 2, y - 2, 5, 5, color);
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
        assert_eq!(glyph('O'), Some([0b010, 0b101, 0b101, 0b101, 0b010]));
        assert_eq!(glyph('é'), None);
        assert_eq!(text_width("1964", 2), 30);
        assert_eq!(text_width("OULU", 2), 30);
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

    #[test]
    fn colore_della_heatmap() {
        assert_eq!(heat_color(0.0, 5.0), (16, 24, 33));
        let cold = heat_color(-5.0, 5.0);
        assert!(cold.2 > cold.0);
        let hot = heat_color(5.0, 5.0);
        assert!(hot.0 > hot.2);
    }

    #[test]
    fn heatmap_valida() {
        let dataset = crate::atlas::load_live().expect("serie live");
        let rows: Vec<usize> = (0..crate::atlas::STATIONS.len())
            .filter(|index| dataset.has_data(*index))
            .collect();
        let bmp = heatmap_bmp(&dataset, &rows);
        assert_eq!(&bmp[0..2], b"BM");
        let row_bytes = (HEAT_W * 3 + 3) & !3;
        assert_eq!(bmp.len(), 54 + row_bytes * heatmap_height(rows.len()));
        assert!(rows.len() > 10);
    }

    #[test]
    fn spettro_e_fit() {
        let points = [(0.8, 8.0), (6.2, 2.0), (12.0, -1.0)];
        let fit = spectrum_fit(&points).expect("fit riuscito");
        assert!(fit.b < 0.0, "lo spettro deve calare con la rigidità");
        assert!(fit.r2 > 0.8);
        let bmp = spectrum_bmp(&points, 5.0, Some(&fit));
        assert_eq!(&bmp[0..2], b"BM");
        assert!(spectrum_fit(&[(1.0, 1.0)]).is_none());
    }

    #[test]
    fn confronto_eventi() {
        let curves = crate::atlas::event_curves();
        assert_eq!(curves.len(), crate::atlas::EVENTS.len());
        let bmp = events_bmp(&curves);
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), 54 + ((EVENTS_W * 3 + 3) & !3) * EVENTS_H);
        // la legenda deve stare dentro i margini: 9 voci in ~130 px l'una
        assert!((EVENTS_W as f64 - EVENTS_PAD_L - EVENTS_PAD_R) / curves.len() as f64 > 110.0);
    }
}
