//! Pagina «Atlante dei raggi cosmici»: mappa animata della rete NMDB, piccoli
//! multipli ordinati per rigidità di taglio e ciclo solare dal 1964.
//!
//! Lo stato vive qui e viene aggiornato dai messaggi del componente; le viste
//! sono funzioni pure. I grafici sono bitmap già pronte (una per stazione):
//! durante la riproduzione cambiano solo i colori dei punti, il cursore e le
//! etichette, non le curve.

use std::collections::HashSet;
use std::sync::Arc;

use crate::api;
use crate::atlas::{self, Dataset, EVENTS, STATIONS};
use crate::atlas_draw;
use crate::{card, color3, it, it_signed, legend_item, scaled};
use windows_reactor::*;

const MAP_W: f64 = 1200.0;
const MAP_H: f64 = 600.0;
const TILE_COLUMNS: usize = 7;

/// Velocità di riproduzione: etichetta e moltiplicatore.
pub const SPEEDS: [(&str, f64); 4] = [("0,5×", 0.5), ("1×", 1.0), ("2×", 2.0), ("4×", 4.0)];

pub struct AtlasState {
    /// 0 = serie live, `1 + i` = evento `EVENTS[i]`.
    pub source: usize,
    pub cursor: usize,
    pub playing: bool,
    pub speed: usize,
    pub selected: Option<usize>,
    /// Stato dell'ultimo scaricamento live, mostrato in testa alla pagina.
    pub live_status: String,
    /// Ultimo errore di scaricamento della rete (se c'è stato).
    pub live_error: Option<String>,
    /// Istante dell'ultimo tentativo di scaricamento live.
    pub live_last: i64,
    pub live_in_flight: bool,
    live: Option<Dataset>,
    events: Vec<Option<Dataset>>,
    /// Un BMP per stazione, nello stesso ordine di `order`.
    sprites: Vec<Arc<[u8]>>,
    /// Indici delle stazioni ordinati per rigidità di taglio crescente.
    order: Vec<usize>,
    /// Heatmap dell'evento attivo (righe = stazioni con dato, colonne = tempo).
    heat: Arc<[u8]>,
    heat_rows: Vec<usize>,
    heat_h: f64,
    /// Confronto fra gli eventi: bitmap statica e legenda.
    events_bmp: Arc<[u8]>,
    curves: Vec<atlas::EventCurve>,
    /// Spettro dell'istante: memoizzato per (sorgente, cursore).
    spectrum: std::cell::RefCell<Option<(usize, usize, Arc<[u8]>)>>,
    solar: Arc<[u8]>,
}

pub struct AtlasCallbacks {
    pub source: Callback<Option<usize>>,
    pub select_station: Callback<Option<usize>>,
    pub scrub: Callback<f64>,
    pub toggle_play: Callback<()>,
    pub speed: Callback<Option<usize>>,
    pub refresh: Callback<()>,
    pub help_map: Callback<()>,
    pub help_multiples: Callback<()>,
    pub help_solar: Callback<()>,
    pub help_heatmap: Callback<()>,
    pub help_spectrum: Callback<()>,
    pub help_events: Callback<()>,
    pub open_dashboard: Callback<()>,
}

impl AtlasState {
    pub fn new() -> Self {
        let mut order: Vec<usize> = (0..STATIONS.len()).collect();
        order.sort_by(|a, b| STATIONS[*a].rc.total_cmp(&STATIONS[*b].rc));
        let events = EVENTS.iter().map(atlas::load_event).collect();
        let live = atlas::load_live();
        let solar = Arc::<[u8]>::from(atlas_draw::solar_bmp(&atlas::solar_series()));
        let curves = atlas::event_curves();
        let events_bmp = Arc::<[u8]>::from(atlas_draw::events_bmp(&curves));
        let live_status = if live.is_some() {
            "istantanea incorporata".to_string()
        } else {
            "serie live non disponibile".to_string()
        };
        let mut state = Self {
            source: 0,
            cursor: 0,
            playing: false,
            speed: 1,
            selected: None,
            live_status,
            live_error: None,
            live_last: 0,
            live_in_flight: false,
            live,
            events,
            sprites: Vec::new(),
            order,
            heat: Arc::<[u8]>::from(Vec::new()),
            heat_rows: Vec::new(),
            heat_h: 0.0,
            events_bmp,
            curves,
            spectrum: std::cell::RefCell::new(None),
            solar,
        };
        state.reset_cursor();
        state.rebuild();
        state
    }

    pub fn dataset(&self) -> Option<&Dataset> {
        if self.source == 0 {
            self.live.as_ref()
        } else {
            self.events.get(self.source - 1).and_then(|event| event.as_ref())
        }
    }

    pub fn source_items() -> Vec<String> {
        let mut items = vec!["Adesso · ultimi 7 giorni".to_string()];
        for event in EVENTS {
            items.push(event.label.to_string());
        }
        items
    }

    pub fn station_items() -> Vec<String> {
        let mut items = vec!["nessuna stazione in evidenza".to_string()];
        items.extend(STATIONS.iter().map(|s| format!("{} · {}", s.name, s.code)));
        items
    }

    /// Etichetta della sorgente attiva, per la testata.
    pub fn source_label(&self) -> String {
        if self.source == 0 {
            "Rete completa · ultimi 7 giorni".to_string()
        } else {
            EVENTS[self.source - 1].label.to_string()
        }
    }

    pub fn set_source(&mut self, index: usize) {
        if index > EVENTS.len() || self.source == index {
            return;
        }
        self.source = index;
        self.playing = false;
        self.reset_cursor();
        self.rebuild();
    }

    pub fn set_selected(&mut self, index: Option<usize>) {
        self.selected = index.filter(|i| *i < STATIONS.len() && *i > 0).map(|i| i - 1);
    }

    fn steps(&self) -> usize {
        self.dataset().map(|d| d.steps).unwrap_or(0)
    }

    fn reset_cursor(&mut self) {
        let steps = self.steps();
        self.cursor = if self.source == 0 {
            steps.saturating_sub(1)
        } else {
            0
        };
    }

    /// Ricalcola le bitmap dei piccoli multipli per la serie attiva.
    fn rebuild(&mut self) {
        self.sprites.clear();
        self.spectrum.replace(None);
        let Some(ds) = self.dataset() else {
            return;
        };
        let mut sprites = Vec::with_capacity(self.order.len());
        for station in &self.order {
            let column = ds.col_of(*station);
            let values: Vec<Option<f64>> = match column {
                Some(column) => (0..ds.steps).map(|step| ds.pct_at(step, column)).collect(),
                None => Vec::new(),
            };
            sprites.push(Arc::<[u8]>::from(atlas_draw::sparkline(&values, ds.scale_pct)));
        }
        let heat_rows: Vec<usize> = self
            .order
            .iter()
            .copied()
            .filter(|station| ds.has_data(*station))
            .collect();
        let heat_h = atlas_draw::heatmap_height(heat_rows.len()) as f64;
        let heat = Arc::<[u8]>::from(atlas_draw::heatmap_bmp(ds, &heat_rows));
        self.sprites = sprites;
        self.heat_rows = heat_rows;
        self.heat_h = heat_h;
        self.heat = heat;
    }

    /// Sostituisce la serie live con quella appena scaricata.
    pub fn set_live(&mut self, dataset: Dataset, status: String) {
        let was_at_end = self.source == 0 && self.cursor + 1 >= self.steps();
        self.live = Some(dataset);
        self.live_status = status;
        if self.source == 0 {
            self.cursor = if was_at_end { self.steps().saturating_sub(1) } else { self.cursor };
            self.playing = false;
            self.rebuild();
        }
    }

    pub fn scrub(&mut self, value: f64) {
        let last = self.steps().saturating_sub(1);
        self.cursor = (value.round().max(0.0) as usize).min(last);
        self.playing = false;
    }

    pub fn toggle_play(&mut self) {
        let steps = self.steps();
        if steps < 2 {
            return;
        }
        if self.playing {
            self.playing = false;
            return;
        }
        if self.cursor + 1 >= steps {
            self.cursor = 0;
        }
        self.playing = true;
    }

    /// Avanza di un fotogramma; ritorna `true` se la riproduzione continua.
    pub fn advance(&mut self) -> bool {
        let steps = self.steps();
        if !self.playing || steps < 2 {
            self.playing = false;
            return false;
        }
        let factor = SPEEDS[self.speed.min(SPEEDS.len() - 1)].1;
        let delta = (steps as f64 / 220.0 * factor).round().max(1.0) as usize;
        if self.cursor + delta >= steps - 1 {
            self.cursor = steps - 1;
            self.playing = false;
        } else {
            self.cursor += delta;
        }
        self.playing
    }

    pub fn station_value(&self, station: usize) -> Option<f64> {
        let ds = self.dataset()?;
        let column = ds.col_of(station)?;
        ds.pct_at(self.cursor, column)
    }

    // -----------------------------------------------------------------------
    // Viste
    // -----------------------------------------------------------------------

    pub fn page(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        let Some(ds) = self.dataset() else {
            return crate::help_panel(
                "Nessuna serie disponibile: gli asset dell'atlante non sono leggibili.",
            );
        };
        StackPanel::new().spacing(16.0).children((
            self.title(callbacks),
            self.controls(callbacks),
            self.map_card(ds, callbacks, help),
            self.heatmap_card(callbacks, help),
            self.spectrum_card(callbacks, help),
            self.multiples_card(callbacks, help),
            self.events_card(callbacks, help),
            self.solar_card(callbacks, help),
            self.back_row(callbacks),
        ))
    }

    /// Pulsante blu per tornare alla dashboard: la pagina è lunga e in cima
    /// ci si arriva solo scorrendo.
    fn back_button(&self, callbacks: &AtlasCallbacks) -> View {
        blue_button("← Torna alla dashboard", callbacks.open_dashboard.clone())
    }

    fn back_row(&self, callbacks: &AtlasCallbacks) -> View {
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .children((self.back_button(callbacks),))
    }

    fn title(&self, callbacks: &AtlasCallbacks) -> View {
        let title = StackPanel::new().spacing(1.0).children((
            TextBlock::new()
                .text("ATLANTE DEI RAGGI COSMICI")
                .font_size(10.5)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(crate::CYAN),
            TextBlock::new()
                .text("LA RETE MONDIALE DEI NEUTRON MONITOR")
                .font_size(22.0)
                .font_weight(FontWeight::BOLD)
                .foreground(crate::TEXT),
            TextBlock::new()
                .text("quaranta stazioni, un Sole e un campo magnetico in mezzo")
                .font_size(12.0)
                .foreground(crate::MUTED),
        ));

        let stations = self.dataset().map(|d| d.active_count()).unwrap_or(0);
        let status_color = if self.live_in_flight {
            crate::AMBER
        } else if self.live_error.is_some() && self.source == 0 {
            crate::ROSE
        } else {
            crate::GREEN
        };
        let status = StackPanel::new()
            .spacing(2.0)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                TextBlock::new()
                    .text(format!("{} · {} stazioni con dato", self.source_label(), stations))
                    .font_size(11.0)
                    .foreground(crate::SOFT)
                    .horizontal_alignment(HorizontalAlignment::Right),
                TextBlock::new()
                    .text(if self.source == 0 {
                        match &self.live_error {
                            Some(error) => format!("{} · live non raggiungibile ({error})", self.live_status),
                            None => format!("{} · aggiornamento automatico ogni 30 minuti", self.live_status),
                        }
                    } else {
                        "evento storico · dati NMDB, passo 10 minuti".to_string()
                    })
                    .font_size(10.5)
                    .foreground(status_color)
                    .horizontal_alignment(HorizontalAlignment::Right),
            ));

        Grid::new()
            .columns([GridLength::STAR, GridLength::Auto, GridLength::Auto])
            .column_spacing(16.0)
            .children((
                Border::new().vertical_alignment(VerticalAlignment::Center).content(title),
                Border::new().vertical_alignment(VerticalAlignment::Center).content(status),
                Border::new()
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(self.back_button(callbacks)),
            ))
    }

    fn controls(&self, callbacks: &AtlasCallbacks) -> View {
        let kind_badge: View = match self.source {
            0 => TextBlock::new().into(),
            index => {
                let event = &EVENTS[index - 1];
                let accent = color3(atlas::kind_color(event.kind));
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(6.0)
                    .vertical_alignment(VerticalAlignment::Center)
                    .children((
                        Rectangle::new()
                            .width(4.0)
                            .height(18.0)
                            .radius_x(2.0)
                            .radius_y(2.0)
                            .fill(accent),
                        TextBlock::new()
                            .text(format!(
                                "{} · {}",
                                match event.kind {
                                    atlas::EventKind::Gle => "GLE",
                                    atlas::EventKind::Forbush => "Forbush",
                                },
                                event.date
                            ))
                            .font_size(10.5)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .foreground(accent),
                    ))
                    .into()
            }
        };
        let source = ComboBox::new()
            .items_source(AtlasState::source_items())
            .selected_index(Some(self.source.min(EVENTS.len())))
            .on_selection_changed(callbacks.source.clone())
            .width(320.0);
        let play = Button::new()
            .on_click(callbacks.toggle_play.clone())
            .style(ButtonStyle::Accent)
            .width(118.0)
            .content(if self.playing { "❚❚ Pausa" } else { "▶ Riproduci" });
        let speed = ComboBox::new()
            .items_source(SPEEDS.iter().map(|(label, _)| label.to_string()))
            .selected_index(Some(self.speed.min(SPEEDS.len() - 1)))
            .on_selection_changed(callbacks.speed.clone())
            .width(84.0);
        let station = ComboBox::new()
            .items_source(AtlasState::station_items())
            .selected_index(Some(self.selected.map(|i| i + 1).unwrap_or(0)))
            .on_selection_changed(callbacks.select_station.clone())
            .width(250.0);

        let refresh: View = if self.source == 0 {
            if self.live_in_flight {
                TextBlock::new()
                    .text("↻ scaricamento della rete in corso…")
                    .font_size(10.5)
                    .foreground(crate::AMBER)
                    .vertical_alignment(VerticalAlignment::Center)
                    .into()
            } else {
                Button::new()
                    .on_click(callbacks.refresh.clone())
                    .content("Aggiorna adesso")
                    .tooltip("Riscarica gli ultimi 7 giorni di tutte le stazioni NMDB")
                    .into()
            }
        } else {
            TextBlock::new().into()
        };

        let steps = self.steps().max(2);
        let slider = Slider::new()
            .minimum(0.0)
            .maximum((steps - 1) as f64)
            .step_frequency(1.0)
            .value(self.cursor as f64)
            .on_value_changed(callbacks.scrub.clone());
        let time = TextBlock::new()
            .text(format!("{} UTC", api::iso_minute(self.time())))
            .font_size(15.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .foreground(crate::CYAN)
            .width(196.0)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center);

        let blurb = match self.source {
            0 => "La rete intera, ora per ora: è la stessa mappa che si aggiorna ogni mezz'ora dal servizio NMDB. Trascina il cursore per ripercorrere la settimana; premi Riproduci per vederla scorrere.".to_string(),
            index => EVENTS[index - 1].blurb.to_string(),
        };

        Border::new()
            .background(crate::CARD_BG)
            .border_brush(crate::CARD_BORDER)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(12.0)
            .padding(Thickness::uniform(14.0))
            .content(StackPanel::new().spacing(10.0).children((
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(10.0)
                    .vertical_alignment(VerticalAlignment::Center)
                    .children((kind_badge, source, play, speed, station, refresh)),
                Grid::new()
                    .columns([GridLength::STAR, GridLength::Auto])
                    .column_spacing(14.0)
                    .children((
                        Border::new().vertical_alignment(VerticalAlignment::Center).content(slider),
                        cell_v(1, time),
                    )),
                TextBlock::new()
                    .text(blurb)
                    .font_size(11.0)
                    .foreground(crate::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
            )))
    }

    fn time(&self) -> i64 {
        self.dataset()
            .map(|d| d.step_time(self.cursor.min(d.steps.saturating_sub(1))))
            .unwrap_or(0)
    }

    fn map_card(&self, ds: &Dataset, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "MAPPA MONDIALE · VARIAZIONE ALL'ISTANTE SCELTO",
            crate::GREEN,
            "Ogni punto è un neutron monitor della rete NMDB, colorato secondo la variazione percentuale dei conteggi rispetto alla base dell'evento (la mediana delle prime sei ore della finestra). Blu: calo, come durante un effetto Forbush. Grigio: nessuna variazione. Giallo e rosso: aumento, come nei GLE. I punti vuoti sono stazioni senza dato in quell'ora. La mappa è equirettangolare: alle alte latitudini le distanze sono dilatate, ma è la proiezione in cui le stazioni polari restano leggibili.",
            self.map_body(ds),
            help.contains("atl-map"),
            callbacks.help_map.clone(),
        )
    }

    fn map_body(&self, ds: &Dataset) -> View {
        let mut items: Vec<KeyedView> = Vec::new();
        let mut key = 0usize;
        items.push(KeyedView::new(
            key,
            Image::new()
                .source_data(EncodedImage::from_static(crate::WORLD_MAP))
                .stretch(Stretch::Fill)
                .width(MAP_W)
                .height(MAP_H)
                .canvas_left(0.0)
                .canvas_top(0.0),
        ));
        key += 1;

        let project = |lat: f64, lon: f64| {
            (
                ((lon + 180.0) / 360.0 * MAP_W).clamp(6.0, MAP_W - 6.0),
                ((90.0 - lat) / 180.0 * MAP_H).clamp(6.0, MAP_H - 6.0),
            )
        };

        for (index, station) in STATIONS.iter().enumerate() {
            let value = ds
                .col_of(index)
                .and_then(|column| ds.pct_at(self.cursor, column));
            let (fill, stroke, size) = match value {
                Some(value) => (
                    color3(atlas::variation_color(value)),
                    Color::rgb(8, 12, 18),
                    if self.selected == Some(index) { 18.0 } else { 13.0 },
                ),
                None => (
                    Color::rgb(30, 41, 53),
                    Color::rgb(71, 85, 105),
                    if self.selected == Some(index) { 18.0 } else { 12.0 },
                ),
            };
            let (x, y) = project(station.lat, station.lon);
            items.push(KeyedView::new(
                key,
                Ellipse::new()
                    .width(size)
                    .height(size)
                    .fill(fill)
                    .stroke(stroke)
                    .stroke_thickness(1.5)
                    .canvas_left(x - size / 2.0)
                    .canvas_top(y - size / 2.0)
                    .tooltip(format!(
                        "{} ({}) · Rc {} GV · quota {:.0} m · {}",
                        station.name,
                        station.country,
                        it(station.rc, 2),
                        station.altitude_m,
                        value
                            .map(|v| format!("{} %", it_signed(v, 2)))
                            .unwrap_or_else(|| "senza dato".into())
                    )),
            ));
            key += 1;
        }

        // etichetta della stazione in evidenza
        if let Some(index) = self.selected {
            let station = &STATIONS[index];
            let (x, y) = project(station.lat, station.lon);
            items.push(KeyedView::new(
                key,
                TextBlock::new()
                    .text(format!(
                        "{} · {} GV · {}",
                        station.name,
                        it(station.rc, 1),
                        self.station_value(index)
                            .map(|v| format!("{} %", it_signed(v, 1)))
                            .unwrap_or_else(|| "senza dato".into())
                    ))
                    .font_size(13.0)
                    .font_weight(FontWeight::SEMI_BOLD)
                    .foreground(crate::CYAN)
                    .canvas_left(if x > MAP_W - 300.0 { x - 296.0 } else { x + 12.0 })
                    .canvas_top((y - 24.0).max(0.0))
                    .width(290.0),
            ));
        }

        // legenda continua delle variazioni
        let swatches: Vec<KeyedView> = (0..40usize)
            .map(|i| {
                let value = -20.0 + 50.0 * i as f64 / 39.0;
                KeyedView::new(
                    i,
                    Rectangle::new()
                        .width(14.0)
                        .height(8.0)
                        .fill(color3(atlas::variation_color(value))),
                )
            })
            .collect();
        let legend = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                TextBlock::new().text("−20 %").font_size(9.5).foreground(crate::MUTED),
                StackPanel::new().orientation(Orientation::Horizontal).keyed_children(swatches),
                TextBlock::new().text("+30 %").font_size(9.5).foreground(crate::MUTED),
                TextBlock::new()
                    .text("calo ← variazione rispetto alla base → aumento")
                    .font_size(9.5)
                    .foreground(crate::SLATE),
            ));

        let (mean, min, max, count) = ds.stats(self.cursor);
        let stats = TextBlock::new()
            .text(if count == 0 {
                "Nessuna stazione con dato in questo istante.".to_string()
            } else {
                format!(
                    "{} UTC · {} stazioni su {} · media {} % · minima {} % · massima {} %",
                    api::iso_minute(self.time()),
                    count,
                    ds.ncol(),
                    it_signed(mean, 1),
                    it_signed(min, 1),
                    it_signed(max, 1)
                )
            })
            .font_size(10.5)
            .foreground(crate::MUTED)
            .text_wrapping(TextWrapping::Wrap);

        StackPanel::new().spacing(8.0).children((
            scaled(Canvas::new().width(MAP_W).height(MAP_H).keyed_children(items)),
            legend,
            stats,
        ))
    }

    fn heatmap_card(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "HEATMAP · LA PROPAGAZIONE IN UN COLPO D'OCCHIO",
            crate::GREEN,
            "Una riga per stazione, ordinate dall'alto in basso per rigidità di taglio crescente; una colonna per passo. Il colore è la variazione percentuale: blu il calo, giallo e rosso l'aumento, grigio scuro il buco di dati. È la vista che i piccoli multipli non danno: qui si vede il fronte dell'evento mentre scende dalle stazioni polari a quelle equatoriali, e si vede dove la rete tace. La linea chiara verticale è l'istante scelto con il cursore.",
            self.heatmap_body(),
            help.contains("atl-heat"),
            callbacks.help_heatmap.clone(),
        )
    }

    fn heatmap_body(&self) -> View {
        let width = atlas_draw::HEAT_W as f64;
        let height = self.heat_h.max(1.0);
        let mut items: Vec<KeyedView> = Vec::new();
        items.push(KeyedView::new(
            2000usize,
            Image::new()
                .source_data(EncodedImage::new(self.heat.clone()))
                .stretch(Stretch::Fill)
                .width(width)
                .height(height)
                .canvas_left(0.0)
                .canvas_top(0.0),
        ));
        let steps = self.steps().max(1);
        if steps > 1 {
            let inner = width - 8.0 - atlas_draw::HEAT_PAD_L;
            let x = atlas_draw::HEAT_PAD_L + self.cursor as f64 / (steps - 1) as f64 * inner;
            items.push(KeyedView::new(
                2001usize,
                Rectangle::new()
                    .width(2.0)
                    .height(height)
                    .fill(Color::rgb(226, 232, 240))
                    .canvas_left(x - 1.0)
                    .canvas_top(0.0),
            ));
        }
        let step = self
            .dataset()
            .map(|d| if d.step <= 900 { "10 minuti" } else { "un'ora" })
            .unwrap_or("un passo");
        StackPanel::new().spacing(6.0).children((
            scaled(Canvas::new().width(width).height(height).keyed_children(items)),
            TextBlock::new()
                .text(format!(
                    "{} stazioni con dato · passo di {step}. In alto le polari (Rc bassa), in basso le equatoriali: la scala delle righe è la stessa dei piccoli multipli qui sotto."
                , self.heat_rows.len()))
                .font_size(10.5)
                .foreground(crate::MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn spectrum_card(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "SPETTRO DI RIGIDITÀ · CHI RISPONDE E QUANTO",
            crate::AMBER,
            "Ogni quadratino è una stazione all'istante scelto: sull'asse orizzontale la rigidità di taglio in scala logaritmica, su quello verticale la variazione percentuale. La retta grigia è il fit Δ% = a − γ·ln(Rc). Se γ è positivo e grande, l'evento ha una forte componente di particelle poco energetiche e la risposta crolla passando alle stazioni equatoriali: è la firma dei GLE. Nei cali di Forbush γ resta piccolo, perché la depressione colpisce quasi tutta la rete. Il fit è indicativo quando le stazioni con dato sono poche.",
            self.spectrum_body(),
            help.contains("atl-spectrum"),
            callbacks.help_spectrum.clone(),
        )
    }

    fn spectrum_body(&self) -> View {
        let Some(dataset) = self.dataset() else {
            return crate::help_panel("Nessuna serie attiva.");
        };
        let points: Vec<(f64, f64)> = (0..STATIONS.len())
            .filter_map(|station| {
                let column = dataset.col_of(station)?;
                let value = dataset.pct_at(self.cursor, column)?;
                Some((STATIONS[station].rc, value))
            })
            .collect();
        let fit = atlas_draw::spectrum_fit(&points);
        let bmp = {
            let mut cache = self.spectrum.borrow_mut();
            match cache.as_ref() {
                Some((source, cursor, bytes)) if *source == self.source && *cursor == self.cursor => {
                    bytes.clone()
                }
                _ => {
                    let bytes: Arc<[u8]> =
                        atlas_draw::spectrum_bmp(&points, dataset.scale_pct, fit.as_ref()).into();
                    *cache = Some((self.source, self.cursor, bytes.clone()));
                    bytes
                }
            }
        };
        let summary = match &fit {
            Some(fit) if points.len() >= 3 => format!(
                "γ = {} · R² {} · {} stazioni con dato · scala verticale ±{} %",
                it(-fit.b, 2),
                it(fit.r2, 2),
                points.len(),
                it(dataset.scale_pct, 0)
            ),
            _ => "Meno di tre stazioni con dato: il fit non è significativo.".to_string(),
        };
        StackPanel::new().spacing(6.0).children((
            Image::new()
                .source_data(EncodedImage::new(bmp))
                .stretch(Stretch::Uniform)
                .width(atlas_draw::SPEC_W as f64)
                .horizontal_alignment(HorizontalAlignment::Left),
            TextBlock::new()
                .text(summary)
                .font_size(11.0)
                .foreground(crate::SOFT)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn events_card(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "EVENTI A CONFRONTO · GLE E CALI DI FORBUSH",
            crate::ROSE,
            "I nove eventi dell'atlante sovrapposti, ognuno come mediana della rete e allineato al proprio picco. In ambra e rosa i GLE: picchi stretti, salita rapida, discesa più lenta. In azzurro e verde i cali di Forbush: depressioni larghe e quasi simmetriche, che durano un giorno o più. La forma racconta il fenomeno meglio di qualunque numero: i due tipi di evento non sono la stessa curva con il segno cambiato.",
            self.events_body(),
            help.contains("atl-events"),
            callbacks.help_events.clone(),
        )
    }

    fn events_body(&self) -> View {
        let gle_count = self.curves.iter().filter(|curve| curve.gle).count();
        let forbush_count = self.curves.len() - gle_count;
        StackPanel::new().spacing(6.0).children((
            scaled(
                Image::new()
                    .source_data(EncodedImage::new(self.events_bmp.clone()))
                    .stretch(Stretch::Fill),
            ),
            TextBlock::new()
                .text(format!("{gle_count} GLE e {forbush_count} cali di Forbush. Le curve sono mediane della rete, non di una singola stazione: servono a confrontare le forme, non le ampiezze assolute. Gli eventi del 1991 e del 2003 hanno meno stazioni, quindi una mediana più fragile."))
                .font_size(10.5)
                .foreground(crate::MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn multiples_card(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "PICCOLI MULTIPLI · UNA STAZIONE PER PANNELLO",
            crate::CYAN,
            "Le stazioni con dato nella serie attiva, una curva ciascuna, ordinate per rigidità di taglio crescente: in alto le stazioni polari (Rc sotto 1 GV), che raccolgono anche i protoni meno energetici, in basso quelle equatoriali (Rc oltre 10 GV), che rispondono solo agli eventi più duri. Non tutte le stazioni coprono tutti gli eventi: i pannelli mostrati sono solo quelli che hanno misure, e per gli eventi più vecchi la rete era molto più piccola. La scala verticale è comune a tutti i pannelli: le differenze di ampiezza sono reali, non un effetto del disegno. Il numero è la variazione all'istante scelto: clicca un pannello per evidenziare la stazione sulla mappa.",
            self.multiples_body(callbacks),
            help.contains("atl-multiples"),
            callbacks.help_multiples.clone(),
        )
    }

    fn multiples_body(&self, callbacks: &AtlasCallbacks) -> View {
        let mut tiles: Vec<KeyedView> = Vec::new();
        let mut slot = 0usize;
        for (index_in_order, station) in self.order.iter().enumerate() {
            let Some(ds) = self.dataset() else {
                break;
            };
            if !ds.has_data(*station) {
                continue;
            }
            let row = (slot / TILE_COLUMNS) as i32;
            let column = (slot % TILE_COLUMNS) as i32;
            slot += 1;
            let index = *station;
            let selected = self.selected == Some(index);
            let tile = Button::new()
                .style(ButtonStyle::Subtle)
                .horizontal_content_alignment(HorizontalAlignment::Stretch)
                .on_click({
                    let select = callbacks.select_station.clone();
                    move || {
                        let _ = select.call(Some(index + 1));
                    }
                })
                .content(self.tile(index_in_order, index, selected));
            tiles.push(KeyedView::new(index, at(column, row, tile)));
        }
        Grid::new()
            .columns(vec![GridLength::STAR; TILE_COLUMNS])
            .column_spacing(8.0)
            .row_spacing(6.0)
            .keyed_children(tiles)
    }

    fn tile(&self, slot: usize, station: usize, selected: bool) -> View {
        let info = &STATIONS[station];
        let value = self.station_value(station);
        let accent = value
            .map(|v| color3(atlas::variation_color(v)))
            .unwrap_or(crate::MUTED);
        let head = Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                TextBlock::new()
                    .text(info.name)
                    .font_size(9.5)
                    .font_weight(FontWeight::SEMI_BOLD)
                    .foreground(if selected { crate::CYAN } else { crate::SOFT }),
                cell_r(
                    1,
                    TextBlock::new()
                        .text(format!("{} GV", it(info.rc, 1)))
                        .font_size(9.0)
                        .foreground(crate::MUTED),
                ),
            ));
        let image: View = match self.sprites.get(slot) {
            Some(bytes) => Image::new()
                .source_data(EncodedImage::new(bytes.clone()))
                .stretch(Stretch::Fill)
                .height(58.0)
                .into(),
            None => TextBlock::new()
                .text("senza dato")
                .font_size(9.0)
                .foreground(crate::SLATE)
                .height(58.0)
                .into(),
        };
        let foot = Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                TextBlock::new()
                    .text(match value {
                        Some(v) => format!("{} %", it_signed(v, 1)),
                        None => "—".to_string(),
                    })
                    .font_size(10.0)
                    .font_weight(FontWeight::SEMI_BOLD)
                    .foreground(accent),
                cell_r(
                    1,
                    TextBlock::new()
                        .text(info.country.to_string())
                        .font_size(8.5)
                        .foreground(crate::SLATE),
                ),
            ));

        Border::new()
            .background(Color::rgb(11, 17, 24))
            .border_brush(if selected { crate::CYAN } else { crate::CARD_BORDER })
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(7.0)
            .padding(Thickness::new(7.0, 5.0, 7.0, 5.0))
            .content(StackPanel::new().spacing(2.0).children((head, image, foot)))
    }

    fn solar_card(&self, callbacks: &AtlasCallbacks, help: &HashSet<String>) -> View {
        card(
            "CICLO SOLARE · 1964-OGGI",
            crate::AMBER,
            "In azzurro la variazione percentuale del neutron monitor di Oulu, mese per mese dal 1964 (media su dodici mesi per togliere il rumore); in ambra il numero di macchie solari mensile del SILSO, la serie di riferimento del ciclo undecennale. I raggi cosmici sono massimi quando il Sole è quieto e minimi qualche mese dopo il massimo delle macchie: è la modulazione solare, la stessa che si vede nelle ultime 24 ore della dashboard, qui su scala di sessant'anni.",
            self.solar_body(),
            help.contains("atl-solar"),
            callbacks.help_solar.clone(),
        )
    }

    fn solar_body(&self) -> View {
        StackPanel::new().spacing(6.0).children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(16.0)
                .children((
                    legend_item(crate::CYAN, "Oulu · variazione % (scala sinistra)"),
                    legend_item(Color::rgb(190, 145, 60), "macchie solari · numero mensile (scala destra)"),
                )),
            scaled(
                Image::new()
                    .source_data(EncodedImage::new(self.solar.clone()))
                    .stretch(Stretch::Fill),
            ),
            TextBlock::new()
                .text("Le due scale sono indipendenti: conta la forma, non l'altezza. A ogni massimo solare Oulu scende di dieci-quindici punti percentuali, a ogni minimo risale; le irregolarità fra un ciclo e l'altro sono una delle cose che i modelli di modulazione ancora non spiegano del tutto.")
                .font_size(10.5)
                .foreground(crate::MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }
}

impl Default for AtlasState {
    fn default() -> Self {
        Self::new()
    }
}

/// Pulsante blu pieno: il tema WinUI non espone il colore del pulsante, quindi
/// si sovrascrivono le risorse del controllo (sfondo, testo, hover, premuto).
fn blue_button(label: &str, on_click: Callback<()>) -> View {
    Button::new()
        .style(ButtonStyle::Default)
        .resource_overrides(
            ResourceOverrides::new()
                .set("ButtonBackground", Color::rgb(37, 99, 235))
                .set("ButtonForeground", Color::rgb(255, 255, 255))
                .set("ButtonBorderBrush", Color::rgb(37, 99, 235))
                .set("ButtonBackgroundPointerOver", Color::rgb(29, 78, 216))
                .set("ButtonBorderBrushPointerOver", Color::rgb(29, 78, 216))
                .set("ButtonBackgroundPressed", Color::rgb(30, 64, 175))
                .set("ButtonBorderBrushPressed", Color::rgb(30, 64, 175)),
        )
        .on_click(on_click)
        .content(label.to_string())
}

/// Cella di griglia, allineata a destra.
fn cell_r(column: i32, view: impl Into<View>) -> View {
    Border::new()
        .grid_column(column)
        .horizontal_alignment(HorizontalAlignment::Right)
        .content(view.into())
}

fn cell_v(column: i32, view: impl Into<View>) -> View {
    Border::new()
        .grid_column(column)
        .vertical_alignment(VerticalAlignment::Center)
        .content(view.into())
}

/// Cella di griglia in una posizione esplicita.
fn at(column: i32, row: i32, view: impl Into<View>) -> View {
    Border::new()
        .grid_column(column)
        .grid_row(row)
        .content(view.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stato_iniziale_sul_live() {
        let state = AtlasState::new();
        assert_eq!(state.source, 0);
        let dataset = state.dataset().expect("serie live");
        assert_eq!(state.cursor + 1, dataset.steps);
        assert_eq!(state.order.len(), STATIONS.len());
        assert_eq!(state.sprites.len(), STATIONS.len());
        assert!(state.sprites.iter().all(|bmp| bmp.starts_with(b"BM")));
    }

    #[test]
    fn ordine_per_rigidita() {
        let state = AtlasState::new();
        let rigidities: Vec<f64> = state.order.iter().map(|i| STATIONS[*i].rc).collect();
        assert!(rigidities.windows(2).all(|w| w[0] <= w[1]), "{rigidities:?}");
    }

    #[test]
    fn eventi_e_riproduzione() {
        let mut state = AtlasState::new();
        for index in 1..=EVENTS.len() {
            state.set_source(index);
            let steps = state.dataset().expect("evento").steps;
            assert_eq!(state.cursor, 0, "{}", EVENTS[index - 1].id);
            assert!(steps > 100);
            state.toggle_play();
            assert!(state.playing);
            let mut guard = 0;
            while state.advance() {
                guard += 1;
                assert!(guard < 10_000, "la riproduzione non finisce");
            }
            assert_eq!(state.cursor, steps - 1);
        }
    }
}
