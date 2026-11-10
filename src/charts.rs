//! Grafici disegnati con primitive WinUI (Line, Rectangle, Ellipse) dentro un Canvas.

use windows_reactor::*;

pub const GRID: Color = Color::rgb(27, 39, 51);
pub const LABEL: Color = Color::rgb(100, 116, 139);

pub struct Axis {
    pub min: f64,
    pub max: f64,
    pub log: bool,
    pub ticks: Vec<(f64, String)>,
}

impl Axis {
    pub fn linear(min: f64, max: f64, ticks: Vec<(f64, String)>) -> Self {
        Self { min, max, log: false, ticks }
    }

    pub fn log(min: f64, max: f64, ticks: Vec<(f64, String)>) -> Self {
        Self { min, max, log: true, ticks }
    }

    fn norm(&self, value: f64) -> f64 {
        if self.log {
            let lo = self.min.max(1e-15).log10();
            let hi = self.max.max(1e-14).log10();
            let v = value.max(1e-15).log10();
            ((v - lo) / (hi - lo)).clamp(-0.05, 1.05)
        } else {
            ((value - self.min) / (self.max - self.min)).clamp(-0.05, 1.05)
        }
    }
}

pub struct Series {
    pub points: Vec<(f64, f64)>,
    pub color: Color,
    pub thickness: f64,
    /// Distanza massima tra due punti consecutivi prima di interrompere la linea.
    pub max_gap: f64,
}

pub struct Marker {
    pub x: f64,
    pub y: f64,
    pub color: Color,
    pub label: String,
}

const PAD_L: f64 = 56.0;
const PAD_R: f64 = 14.0;
const PAD_T: f64 = 14.0;
const PAD_B: f64 = 26.0;

/// Grafico a linee con assi, griglia ed eventuali marcatori.
pub fn plot(
    width: f64,
    height: f64,
    x: Axis,
    y: Axis,
    series: Vec<Series>,
    markers: Vec<Marker>,
    zero_line: bool,
) -> View {
    let inner_w = (width - PAD_L - PAD_R).max(10.0);
    let inner_h = (height - PAD_T - PAD_B).max(10.0);
    let px = |v: f64| PAD_L + inner_w * x.norm(v);
    let py = |v: f64| PAD_T + inner_h * (1.0 - y.norm(v));

    let mut items: Vec<KeyedView> = Vec::new();
    let mut key = 0usize;

    macro_rules! push {
        ($view:expr, $key:expr $(,)?) => {{
            items.push(KeyedView::new(*$key, $view));
            *$key += 1;
        }};
    }

    // griglia orizzontale + etichette Y
    for (value, label) in &y.ticks {
        let yy = py(*value);
        push!(
            Line::new()
                .x1(PAD_L)
                .x2(width - PAD_R)
                .y1(yy)
                .y2(yy)
                .stroke(GRID)
                .stroke_thickness(1.0),
            &mut key,
        );
        push!(
            TextBlock::new()
                .text(label.clone())
                .font_size(10.0)
                .foreground(LABEL)
                .canvas_left(0.0)
                .canvas_top(yy - 7.0)
                .width(PAD_L - 8.0),
            &mut key,
        );
    }

    // etichette X
    for (value, label) in &x.ticks {
        let xx = px(*value);
        push!(
            Line::new()
                .x1(xx)
                .x2(xx)
                .y1(PAD_T)
                .y2(PAD_T + inner_h)
                .stroke(GRID)
                .stroke_thickness(0.6),
            &mut key,
        );
        push!(
            TextBlock::new()
                .text(label.clone())
                .font_size(10.0)
                .foreground(LABEL)
                .canvas_left((xx - 26.0).clamp(0.0, (width - 52.0).max(0.0)))
                .canvas_top(height - 20.0)
                .width(52.0),
            &mut key,
        );
    }

    if zero_line && y.min < 0.0 && y.max > 0.0 {
        let yy = py(0.0);
        push!(
            Line::new()
                .x1(PAD_L)
                .x2(width - PAD_R)
                .y1(yy)
                .y2(yy)
                .stroke(Color::rgb(71, 85, 105))
                .stroke_thickness(1.0),
            &mut key,
        );
    }

    // serie
    for s in &series {
        for pair in s.points.windows(2) {
            let (x0, y0) = pair[0];
            let (x1, y1) = pair[1];
            if (x1 - x0).abs() > s.max_gap {
                continue;
            }
            push!(
                Line::new()
                    .x1(px(x0))
                    .y1(py(y0))
                    .x2(px(x1))
                    .y2(py(y1))
                    .stroke(s.color)
                    .stroke_thickness(s.thickness),
                &mut key,
            );
        }
        // punti isolati
        if s.points.len() == 1 {
            let (x0, y0) = s.points[0];
            push!(
                Ellipse::new()
                    .width(7.0)
                    .height(7.0)
                    .fill(s.color)
                    .canvas_left(px(x0) - 3.5)
                    .canvas_top(py(y0) - 3.5),
                &mut key,
            );
        }
    }

    // marcatori (linea verticale + punto + etichetta), sopra le serie
    for marker in &markers {
        let xx = px(marker.x);
        let yy = py(marker.y);
        push!(
            Line::new()
                .x1(xx)
                .x2(xx)
                .y1(PAD_T)
                .y2(PAD_T + inner_h)
                .stroke(Color::argb(110, marker.color.r, marker.color.g, marker.color.b))
                .stroke_thickness(1.0),
            &mut key,
        );
        push!(
            Ellipse::new()
                .width(11.0)
                .height(11.0)
                .fill(marker.color)
                .canvas_left(xx - 5.5)
                .canvas_top(yy - 5.5),
            &mut key,
        );
        let label_x = if xx > width - PAD_R - 200.0 {
            xx - 206.0
        } else {
            xx + 8.0
        };
        push!(
            TextBlock::new()
                .text(marker.label.clone())
                .font_size(10.0)
                .foreground(marker.color)
                .canvas_left(label_x)
                .canvas_top((yy - 22.0).max(PAD_T))
                .width(200.0),
            &mut key,
        );
    }

    Canvas::new().width(width).height(height).keyed_children(items)
}

/// Barra Kp: etichetta (vuota = nessuna), valore, previsto (disegnato tenue).
pub struct KpBar {
    pub label: String,
    pub kp: f64,
    pub predicted: bool,
}

/// Barre verticali per l'indice Kp, con scala 0..9, colori per livello e una linea
/// verticale prima della prima barra prevista ("adesso").
pub fn kp_bars(width: f64, height: f64, points: &[KpBar]) -> View {
    let inner_w = (width - PAD_L - PAD_R).max(10.0);
    let inner_h = (height - PAD_T - PAD_B).max(10.0);
    let mut items: Vec<KeyedView> = Vec::new();
    let mut key = 0usize;

    for level in [0.0_f64, 3.0, 6.0, 9.0] {
        let yy = PAD_T + inner_h * (1.0 - level / 9.0);
        items.push(KeyedView::new(
            key,
            Line::new()
                .x1(PAD_L)
                .x2(width - PAD_R)
                .y1(yy)
                .y2(yy)
                .stroke(GRID)
                .stroke_thickness(1.0),
        ));
        key += 1;
        items.push(KeyedView::new(
            key,
            TextBlock::new()
                .text(format!("{level:.0}"))
                .font_size(10.0)
                .foreground(LABEL)
                .canvas_left(0.0)
                .canvas_top(yy - 7.0)
                .width(PAD_L - 8.0),
        ));
        key += 1;
    }

    if points.is_empty() {
        return Canvas::new()
            .width(width)
            .height(height)
            .keyed_children(items);
    }

    let slot = inner_w / points.len() as f64;
    let bar_w = (slot * 0.62).max(2.0);
    let mut now_drawn = false;
    for (index, bar) in points.iter().enumerate() {
        let cx = PAD_L + slot * (index as f64 + 0.5);
        let kp = bar.kp.clamp(0.0, 9.0);
        let h = inner_h * (kp / 9.0);
        let (r, g, b) = crate::model::kp_color(kp);
        if bar.predicted && !now_drawn && index > 0 {
            now_drawn = true;
            let xx = cx - slot / 2.0;
            items.push(KeyedView::new(
                key,
                Line::new()
                    .x1(xx)
                    .x2(xx)
                    .y1(PAD_T - 6.0)
                    .y2(PAD_T + inner_h)
                    .stroke(Color::rgb(148, 163, 184))
                    .stroke_thickness(1.0),
            ));
            key += 1;
            items.push(KeyedView::new(
                key,
                TextBlock::new()
                    .text("previsione →")
                    .font_size(10.0)
                    .foreground(Color::rgb(148, 163, 184))
                    .canvas_left(xx + 4.0)
                    .canvas_top(PAD_T - 12.0)
                    .width(90.0),
            ));
            key += 1;
        }
        let alpha = if bar.predicted { 95 } else { 255 };
        items.push(KeyedView::new(
            key,
            Rectangle::new()
                .width(bar_w)
                .height(h.max(1.5))
                .fill(Color::argb(alpha, r, g, b))
                .radius_x(2.0)
                .radius_y(2.0)
                .canvas_left(cx - bar_w / 2.0)
                .canvas_top(PAD_T + inner_h - h.max(1.5)),
        ));
        key += 1;
        if !bar.label.is_empty() {
            items.push(KeyedView::new(
                key,
                TextBlock::new()
                    .text(bar.label.clone())
                    .font_size(10.0)
                    .foreground(LABEL)
                    .canvas_left((cx - 18.0).clamp(0.0, (width - 40.0).max(0.0)))
                    .canvas_top(height - 20.0)
                    .width(40.0),
            ));
            key += 1;
        }
    }

    Canvas::new().width(width).height(height).keyed_children(items)
}
