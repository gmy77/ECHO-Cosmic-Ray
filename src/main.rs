#![windows_subsystem = "windows"]

mod api;
mod charts;
mod model;

use charts::{Axis, Marker, Series};
use model::*;
use windows_reactor::*;

const BG: Color = Color::rgb(7, 11, 16);
const CARD_BG: Color = Color::rgb(14, 21, 29);
const CARD_BORDER: Color = Color::rgb(30, 41, 53);
const TEXT: Color = Color::rgb(226, 232, 240);
const MUTED: Color = Color::rgb(124, 139, 156);
const CYAN: Color = Color::rgb(34, 211, 238);
const AMBER: Color = Color::rgb(245, 158, 11);
const VIOLET: Color = Color::rgb(167, 139, 250);
const GREEN: Color = Color::rgb(52, 211, 153);
const ROSE: Color = Color::rgb(248, 113, 113);

const WORLD_MAP: &[u8] = include_bytes!("../assets/worldmap.png");

const MAP_W: f64 = 1200.0;
const MAP_H: f64 = 600.0;

fn icon_path() -> &'static str {
    static PATH: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let mut candidates: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("app.ico"));
            }
        }
        candidates.push(std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "\\assets\\app.ico"
        )));
        for candidate in candidates {
            if candidate.exists() {
                return Box::leak(
                    candidate.to_string_lossy().into_owned().into_boxed_str(),
                );
            }
        }
        Box::leak(String::new().into_boxed_str())
    })
}

fn downsample<T: Copy>(items: &[T], max: usize) -> Vec<T> {
    if items.len() <= max {
        return items.to_vec();
    }
    let step = items.len() as f64 / max as f64;
    let mut out: Vec<T> = (0..max).map(|i| items[(i as f64 * step) as usize]).collect();
    if let Some(last) = items.last() {
        *out.last_mut().unwrap() = *last;
    }
    out
}

struct Dashboard {
    snapshot: Option<api::Snapshot>,
    error: Option<String>,
    loading: bool,
    altitude_index: usize,
    fetches: u64,
    help: std::collections::HashSet<String>,
}

#[derive(Clone)]
enum Message {
    Loaded(Box<api::Snapshot>),
    Failed(String),
    Tick,
    Refresh,
    SetAltitude(Option<usize>),
    ToggleHelp(String),
}

impl Component for Dashboard {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        context.spawn_background(|_cancel| match std::panic::catch_unwind(|| api::fetch(true)) {
            Ok(snapshot) => Message::Loaded(Box::new(snapshot)),
            Err(_) => Message::Failed("errore interno durante il download".to_string()),
        });
        Self { snapshot: None, error: None, loading: true, altitude_index: 0, fetches: 0, help: std::collections::HashSet::new() }
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        match message {
            Message::Loaded(snapshot) => {
                let mut next = *snapshot;
                if let Some(old) = &self.snapshot {
                    if next.wind_speed.is_none() {
                        next.wind_speed = old.wind_speed;
                    }
                    if next.bz_nt.is_none() {
                        next.bz_nt = old.bz_nt;
                    }
                    if next.dst_nt.is_none() {
                        next.dst_nt = old.dst_nt;
                    }
                    if next.proton_pfu.is_none() {
                        next.proton_pfu = old.proton_pfu;
                    }
                    if next.wind_series.is_empty() {
                        next.wind_series = old.wind_series.clone();
                    }
                    if next.bz_series.is_empty() {
                        next.bz_series = old.bz_series.clone();
                    }
                    if next.gcr_rome.is_empty() {
                        next.gcr_rome = old.gcr_rome.clone();
                    }
                    if next.proton_series.is_empty() {
                        next.proton_series = old.proton_series.clone();
                    }
                    if next.dst_series.is_empty() {
                        next.dst_series = old.dst_series.clone();
                    }
                }
                self.error = None;
                self.snapshot = Some(next);
                self.loading = false;
                self.fetches += 1;
                schedule_tick(context);
            }
            Message::Failed(error) => {
                self.error = Some(error);
                self.loading = false;
                schedule_tick(context);
            }
            Message::Tick => {
                let deep = self.fetches % 5 == 0;
                self.loading = true;
                context.spawn_background(move |_cancel| {
                    match std::panic::catch_unwind(|| api::fetch(deep)) {
                        Ok(snapshot) => Message::Loaded(Box::new(snapshot)),
                        Err(_) => Message::Failed("errore interno durante il download".to_string()),
                    }
                });
            }
            Message::Refresh => {
                self.loading = true;
                context.spawn_background(|_cancel| {
                    match std::panic::catch_unwind(|| api::fetch(true)) {
                        Ok(snapshot) => Message::Loaded(Box::new(snapshot)),
                        Err(_) => Message::Failed("errore interno durante il download".to_string()),
                    }
                });
            }
            Message::SetAltitude(index) => {
                if let Some(index) = index {
                    if index < ALTITUDE_CHOICES.len() {
                        self.altitude_index = index;
                    }
                }
            }
            Message::ToggleHelp(id) => {
                if !self.help.remove(&id) {
                    self.help.insert(id);
                }
            }
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("Cosmic Ray Dashboard · radiazione cosmica e condizioni solari");
        context.window_visuals(
            WindowVisuals::new()
                .client_size(1600.0, 900.0)
                .backdrop(WindowBackdrop::Mica)
                .theme(WindowTheme::Dark)
                .icon(icon_path())
                .constraints(WindowConstraints {
                    min_width: Some(1280.0),
                    min_height: Some(760.0),
                    max_width: None,
                    max_height: None,
                }),
        );

        let header = self.header(context);
        let kpis = self.kpi_row(context);
        let row1 = self.row_main(context);
        let drivers = self.driver_row(context);
        let row2 = self.row_altitude_map(context);
        let footer = self.footer();

        let content = StackPanel::new()
            .spacing(16.0)
            .margin(Thickness::new(24.0, 18.0, 24.0, 26.0))
            .children((header, kpis, row1, drivers, row2, footer));

        Border::new()
            .background(BG)
            .content(
                ScrollViewer::new()
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                    .content(content),
            )
    }
}

fn schedule_tick(context: &ComponentContext<Dashboard>) {
    context.spawn_background(|_cancel| {
        std::thread::sleep(std::time::Duration::from_secs(60));
        Message::Tick
    });
}

impl Dashboard {
    fn header(&self, context: &mut ViewContext<Self>) -> View {
        let title = StackPanel::new().spacing(2.0).children((
            TextBlock::new()
                .text("RADIAZIONE COSMICA")
                .font_size(22.0)
                .font_weight(FontWeight::BOLD)
                .foreground(TEXT),
            TextBlock::new()
                .text("raggi cosmici al suolo e in quota · confronto con l'attività solare · dati live")
                .font_size(12.0)
                .foreground(MUTED),
        ));

        let status_text = if self.loading {
            "aggiornamento…".to_string()
        } else if let Some(snapshot) = &self.snapshot {
            format!("live · {} UTC", api::hhmm_utc(snapshot.fetched_at))
        } else {
            "in attesa dei dati".to_string()
        };
        let status_color = if self.error.is_some() { AMBER } else { GREEN };

        let status = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                Ellipse::new().width(8.0).height(8.0).fill(status_color),
                TextBlock::new().text(status_text).font_size(11.0).foreground(MUTED),
                Button::new()
                    .on_click(context.message(Message::Refresh))
                    .content("Aggiorna"),
            ));

        Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .column_spacing(16.0)
            .children((
                Border::new()
                    .vertical_alignment(VerticalAlignment::Center)
                    .content(title),
                cell(1, status),
            ))
    }

    fn kpi_row(&self, context: &mut ViewContext<Self>) -> View {
        let snapshot = self.snapshot.clone().unwrap_or_default();
        let (alt, _) = ALTITUDE_CHOICES[self.altitude_index];

        let gcr = snapshot.gcr_now_avg(10);
        let gcr_range = snapshot.gcr_range();
        let modulation = gcr.unwrap_or(0.0);

        let (alt_value, alt_sub) = if self.snapshot.is_some() {
            (
                format!("{:.3}", modeled_dose(alt, modulation)),
                format!("quota {alt:.0} m · suolo {:.3} µSv/h", dose_ush(0.0)),
            )
        } else {
            ("—".to_string(), "in attesa".to_string())
        };

        let kp = snapshot.kp_now();
        let xray = snapshot.xray_now();
        let wind = snapshot.wind_speed;
        let bz = snapshot.bz_nt;

        let cards = (
            kpi_card(
                "RAGGI COSMICI · OULU",
                gcr.map(|v| format!("{v:+.2}")).unwrap_or("—".into()),
                "%",
                match gcr_range {
                    Some((lo, hi)) => format!("24 h · min {lo:+.1}% / max {hi:+.1}%"),
                    None => "neutron monitor · 24 h".into(),
                },
                CYAN,
                "Conteggio del neutron monitor di Oulu (Finlandia, 65°N), espresso come variazione percentuale rispetto alla media delle ultime 24 ore. I neutroni misurati sono prodotti secondari dei raggi cosmici galattici che colpiscono l'atmosfera. Valori negativi = meno raggi cosmici al suolo: accade quando l'attività solare cresce o durante una tempesta geomagnetica (effetto Forbush). Fonte: NMDB, cadenza ~1 minuto.",
                self.help.contains("kpi-gcr"),
                context.message(Message::ToggleHelp("kpi-gcr".into())),
            ),
            kpi_card(
                "INDICE KP",
                kp.map(|v| format!("{v:.2}")).unwrap_or("—".into()),
                "Kp",
                kp.map(|v| format!("{}", kp_storm(v))).unwrap_or("—".into()),
                kp.map(|v| {
                    let (r, g, b) = model::kp_color(v);
                    Color::rgb(r, g, b)
                })
                .unwrap_or(MUTED),
                "Indice Kp (0-9) del disturbo geomagnetico planetario, in terzi di unità: Kp 0-3 quiete, 4 attivo, ≥5 tempesta (scala NOAA G1-G5). Il valore mostrato è la stima al minuto di SWPC; per la climatologia si usa la media a 3 ore. Più alto è Kp, più il campo geomagnetico è disturbato dal vento solare.",
                self.help.contains("kpi-kp"),
                context.message(Message::ToggleHelp("kpi-kp".into())),
            ),
            kpi_card(
                "VENTO SOLARE",
                wind.map(|v| format!("{v:.0}")).unwrap_or("—".into()),
                "km/s",
                match bz {
                    Some(bz) => format!("Bz {bz:+.1} nT · IMF"),
                    None => "IMF non ancora scaricata".into(),
                },
                VIOLET,
                "Velocità del plasma solare all'altezza della Terra (km/s) misurata dai satelliti DSCOVR/ACE, con la componente Bz del campo magnetico interplanetario. Vento oltre 600 km/s indica un flusso veloce o un'espulsione coronale; Bz negativo (campo diretto a sud) permette al vento di accoppiarsi alla magnetosfera e innescare tempeste.",
                self.help.contains("kpi-wind"),
                context.message(Message::ToggleHelp("kpi-wind".into())),
            ),
            kpi_card(
                "FLUSSO X-RAY GOES",
                xray.map(flare_class).unwrap_or("—".into()),
                "classe",
                xray
                    .map(|v| format!("{:.2e} W/m² · 0.1-0.8 nm", v))
                    .unwrap_or("—".into()),
                AMBER,
                "Flusso di raggi X nella banda 0,1-0,8 nm misurato dai satelliti GOES. Le classi vanno da A (1e-8 W/m²) a X (1e-4 W/m²), ognuna dieci volte la precedente: C e M indicano attività crescente, X un brillamento forte. La radiazione arriva in 8 minuti e può precedere un aumento dei protoni energetici.",
                self.help.contains("kpi-xray"),
                context.message(Message::ToggleHelp("kpi-xray".into())),
            ),
            kpi_card(
                "DOSE · QUOTA SCELTA",
                alt_value,
                "µSv/h",
                alt_sub,
                ROSE,
                "Dose equivalente da raggi cosmici alla quota scelta, stimata con un modello basato su misure pubblicate (UNSCEAR/ICRU) e scalata con la modulazione solare osservata da Oulu. Non è una misura: al suolo la componente cosmica vale ~0,03 µSv/h, a 10,7 km qualche µSv/h, con il massimo attorno ai 15 km (massimo di Pfotzer).",
                self.help.contains("kpi-dose"),
                context.message(Message::ToggleHelp("kpi-dose".into())),
            ),
        );

        Grid::new()
            .columns([
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
            ])
            .column_spacing(12.0)
            .children((
                cell_top(0, cards.0),
                cell_top(1, cards.1),
                cell_top(2, cards.2),
                cell_top(3, cards.3),
                cell_top(4, cards.4),
            ))
    }

    fn row_main(&self, context: &mut ViewContext<Self>) -> View {
        let snapshot = self.snapshot.clone().unwrap_or_default();
        let gcr_card = card(
            "INTENSITÀ DEI RAGGI COSMICI · ULTIME 24 ORE",
            CYAN,
            "Serie delle ultime 24 ore di due neutron monitor: Oulu (Finlandia, 65°N, rigidità di taglio 0,8 GV) e Roma (41°N, 6,2 GV). La linea sottile è il dato al minuto, quella spessa la media mobile a 30 minuti: la statistica dei neutroni è rumorosa e la media rende visibili la modulazione solare e le cadute di Forbush. Il confronto tra le due stazioni mostra l'effetto del taglio geomagnetico: a Roma arrivano solo raggi cosmici più energetici, quindi l'ampiezza delle variazioni è molto minore. Fonte: NMDB, aggiornamento ~1 minuto.",
            self.gcr_chart(&snapshot),
            self.help.contains("gcr"),
            context.message(Message::ToggleHelp("gcr".into())),
        );
        let solar_card = card(
            "ATTIVITÀ SOLARE",
            AMBER,
            "Indice Kp medio orario delle ultime ore (barre colorate: azzurro = quiete, verde/giallo = attivo, rosso = tempesta) e flusso X-ray GOES 0,1-0,8 nm su 6 ore in scala logaritmica, con le classi A/B/C/M/X. In basso i valori istantanei di vento solare, Bz, Dst e protoni ≥10 MeV, gli stessi parametri dei quattro pannelli successivi.",
            self.solar_panel(&snapshot),
            self.help.contains("solar"),
            context.message(Message::ToggleHelp("solar".into())),
        );

        Grid::new()
            .columns([GridLength::Pixel(900.0), GridLength::STAR])
            .column_spacing(16.0)
            .children((cell(0, gcr_card), cell(1, solar_card)))
    }

    fn row_altitude_map(&self, context: &mut ViewContext<Self>) -> View {
        let snapshot = self.snapshot.clone().unwrap_or_default();
        let altitude_card = card(
            "DOSE PER QUOTA · PROFILO VERTICALE",
            ROSE,
            "Dose equivalente da raggi cosmici in funzione della quota, da 0 a 40 km, in scala logaritmica. Il massimo attorno ai 15 km è il massimo di Pfotzer: lo sciame secondario (neutroni, muoni, elettroni) è più intenso lì che al suolo o nello spazio. Il selettore sceglie la quota; il pallino giallo mostra la dose a quella quota e la linea grigia il livello al suolo. Attenzione: è un modello basato su misure pubblicate (UNSCEAR/ICRU, campagne di volo), scalato con la modulazione solare del momento, non una misura in tempo reale.",
            self.altitude_panel(&snapshot, context),
            self.help.contains("altitude"),
            context.message(Message::ToggleHelp("altitude".into())),
        );
        let map_card = card(
            "INTENSITÀ AL SUOLO · RETE MONDIALE",
            GREEN,
            "Mappa dell'intensità relativa dei raggi cosmici al suolo: massima alle alte latitudini e minima all'equatore magnetico, perché il campo geomagnetico scherma le particelle meno energetiche. Il colore combina il taglio geomagnetico dipolare con la modulazione solare osservata in quel momento. I puntini gialli sono i neutron monitor della rete NMDB, il cerchio azzurro è Roma. Le bande orizzontali riflettono la sola dipendenza dalla latitudine: il modello non include l'anomalia magnetica dell'Atlantico meridionale.",
            self.map_panel(&snapshot),
            self.help.contains("map"),
            context.message(Message::ToggleHelp("map".into())),
        );

        Grid::new()
            .columns([GridLength::Pixel(900.0), GridLength::STAR])
            .column_spacing(16.0)
            .children((cell(0, altitude_card), cell(1, map_card)))
    }

    fn footer(&self) -> View {
        let notes = self
            .snapshot
            .as_ref()
            .map(|s| s.notes.clone())
            .unwrap_or_default();
        let mut lines = vec![
            "Fonti live: NMDB (neutron monitor Oulu) · NOAA SWPC (Kp, GOES X-ray, protoni, vento solare, Dst, F10.7).".to_string(),
            "Il profilo di dose in quota è un modello: ancore da misure pubblicate (UNSCEAR/ICRU, campagne di volo) scalate con la modulazione solare osservata.".to_string(),
        ];
        if let Some(error) = &self.error {
            lines.push(format!("Ultimo errore: {error}"));
        }
        if !notes.is_empty() {
            let short: Vec<String> = notes.iter().take(3).cloned().collect();
            lines.push(format!("Avvisi: {}", short.join(" · ")));
        }

        StackPanel::new()
            .spacing(3.0)
            .children((
                TextBlock::new()
                    .text(lines[0].clone())
                    .font_size(10.5)
                    .foreground(MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                TextBlock::new()
                    .text(lines[1].clone())
                    .font_size(10.5)
                    .foreground(MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                TextBlock::new()
                    .text(lines.get(2).cloned().unwrap_or_default())
                    .font_size(10.5)
                    .foreground(AMBER)
                    .text_wrapping(TextWrapping::Wrap),
            ))
    }

    fn gcr_chart(&self, snapshot: &api::Snapshot) -> View {
        let t0 = snapshot.gcr_pct.first().map(|p| p.0).unwrap_or(0);
        let hours_of = |t: i64| (t - t0) as f64 / 3600.0;

        let raw_h: Vec<(f64, f64)> = snapshot
            .gcr_pct
            .iter()
            .map(|(t, v)| (hours_of(*t), *v))
            .collect();
        let mut smooth_h = smooth_mean(&raw_h, 0.5);
        let rome_h: Vec<(f64, f64)> = snapshot
            .gcr_rome
            .iter()
            .filter_map(|(t, v)| {
                let h = hours_of(*t);
                (h >= -0.05).then_some((h.max(0.0), *v))
            })
            .collect();
        let mut rome_s = smooth_mean(&rome_h, 0.5);
        if let Some(first) = raw_h.first().map(|p| p.0) {
            smooth_h.retain(|p| p.0 >= first + 0.25);
            rome_s.retain(|p| p.0 >= first + 0.25);
        }

        let raw = downsample(&raw_h, 220);
        let smooth = downsample(&smooth_h, 220);
        let rome = downsample(&rome_s, 220);

        let (x, y, series, markers) = if smooth.len() < 2 {
            (
                Axis::linear(0.0, 24.0, vec![(0.0, "—".into()), (24.0, "—".into())]),
                Axis::linear(-1.0, 1.0, vec![(0.0, "0".into())]),
                vec![],
                vec![],
            )
        } else {
            let hours = raw_h.last().map(|p| p.0).unwrap_or(24.0);
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for (_, v) in smooth.iter().chain(rome.iter()) {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
            let margin = ((hi - lo) * 0.25).max(0.6);
            lo -= margin;
            hi += margin;
            let ticks: Vec<(f64, String)> = nice_ticks(lo, hi, 5)
                .into_iter()
                .map(|v| (v, format!("{v:+.1}")))
                .collect();
            let mut xticks = Vec::new();
            let mut hh = 0.0;
            while hh <= hours {
                let stamp = t0 + (hh * 3600.0) as i64;
                xticks.push((hh, api::hhmm_utc(stamp)));
                hh += 6.0;
            }
            let last = smooth.last().copied().unwrap_or((0.0, 0.0));
            let mut plot_series = Vec::new();
            if rome.len() > 1 {
                plot_series.push(Series {
                    points: rome,
                    color: AMBER,
                    thickness: 1.7,
                    max_gap: 0.3,
                });
            }
            plot_series.push(Series {
                points: raw,
                color: Color::argb(35, 34, 211, 238),
                thickness: 0.7,
                max_gap: 0.25,
            });
            plot_series.push(Series {
                points: smooth,
                color: CYAN,
                thickness: 2.2,
                max_gap: 0.25,
            });
            (
                Axis::linear(0.0, hours.max(1.0), xticks),
                Axis::linear(lo, hi, ticks),
                plot_series,
                vec![Marker {
                    x: last.0,
                    y: last.1,
                    color: AMBER,
                    label: format!("{:.2}%", last.1),
                }],
            )
        };

        let legend = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(16.0)
            .children((
                legend_item(CYAN, "Oulu · 0,8 GV · al minuto + media 30 min"),
                legend_item(AMBER, "Roma · 6,2 GV"),
            ));

        StackPanel::new().spacing(6.0).children((
            legend,
            scaled(charts::plot(872.0, 232.0, x, y, series, markers, true)),
            TextBlock::new()
                .text("Variazione percentuale del conteggio dei neutron monitor: i raggi cosmici galattici calano quando il Sole è più attivo (modulazione solare), con brusche cadute (effetto Forbush) durante le tempeste. La linea spessa è la media mobile a 30 minuti; Roma, con rigidità di taglio 6,2 GV, oscilla molto meno di Oulu (0,8 GV).")
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn driver_row(&self, context: &mut ViewContext<Self>) -> View {
        let snapshot = self.snapshot.clone().unwrap_or_default();
        let now = snapshot.fetched_at;
        let within = |points: &[(i64, f64)], hours: i64| -> Vec<(i64, f64)> {
            points
                .iter()
                .filter(|p| p.0 >= now - hours * 3600)
                .cloned()
                .collect()
        };

        let dst_series = within(&snapshot.dst_series, 72);
        let bz_series = within(&snapshot.bz_series, 24);
        let wind_series = within(&snapshot.wind_series, 24);
        let proton_series = within(&snapshot.proton_series, 6);

        let dst = dst_series.last().map(|p| p.1);
        let bz = bz_series.last().map(|p| p.1);
        let wind = wind_series.last().map(|p| p.1);
        let proton = proton_series.last().map(|p| p.1);

        let dst_chart = mini_chart(
            &dst_series,
            CYAN,
            true,
            None,
            dst.map(|v| format!("{v:+.0} nT")).unwrap_or("—".into()),
        );
        let bz_chart = mini_chart(
            &bz_series,
            VIOLET,
            true,
            None,
            bz.map(|v| format!("{v:+.1} nT")).unwrap_or("—".into()),
        );
        let wind_chart = mini_chart(
            &wind_series,
            GREEN,
            false,
            None,
            wind.map(|v| format!("{v:.0} km/s")).unwrap_or("—".into()),
        );
        let proton_chart = mini_chart(
            &proton_series,
            AMBER,
            false,
            Some((1e-2, 1e3, vec![
                (1e-1, "0,1".into()),
                (1.0, "1".into()),
                (10.0, "10".into()),
                (100.0, "100".into()),
            ])),
            proton.map(|v| format!("{v:.2} pfu")).unwrap_or("—".into()),
        );

        let dst_card = card(
            "DST · 72 ORE",
            CYAN,
            "Indice Dst (nT): media degli scostamenti del campo geomagnetico misurati a basse latitudini, aggiornato ogni ora. Valori negativi indicano la corrente ad anello che si intensifica durante le tempeste: circa −30 nT quiete, −50 tempesta debole, −100 tempesta intensa, −300 evento estremo. È il parametro che riassume meglio l'energia immagazzinata nella magnetosfera. Fonte: WDC Kyoto / NOAA SWPC, ultimi 3 giorni.",
            dst_chart,
            self.help.contains("dst"),
            context.message(Message::ToggleHelp("dst".into())),
        );
        let bz_card = card(
            "BZ IMF · 24 ORE",
            VIOLET,
            "Componente nord-sud del campo magnetico interplanetario (nT), ultime 24 ore. Bz negativo significa campo diretto a sud: si riconnette con la magnetosfera terrestre, le apre la porta e alimenta tempeste e aurore. È il precursore più diretto di un peggioramento geomagnetico; la linea dello zero è il punto di inversione.",
            bz_chart,
            self.help.contains("bz"),
            context.message(Message::ToggleHelp("bz".into())),
        );
        let wind_card = card(
            "VENTO SOLARE · 24 ORE",
            GREEN,
            "Velocità del vento solare (km/s) nelle ultime 24 ore. Valori tipici 350-450 km/s in quiete; oltre 600 km/s indicano un flusso veloce da buco coronale o il passaggio di un'espulsione coronale (ICME). I gradini corrispondono al passaggio da una regione di vento lento a una di vento veloce. Fonte: DSCOVR/ACE via NOAA SWPC.",
            wind_chart,
            self.help.contains("wind"),
            context.message(Message::ToggleHelp("wind".into())),
        );
        let proton_card = card(
            "PROTONI ≥10 MeV · 6 ORE",
            AMBER,
            "Flusso integrale di protoni ≥10 MeV (particelle per cm²·sr·s, detti pfu) misurato dai satelliti GOES. Fondo tipico 0,1-1 pfu; sopra 10 pfu scatta l'allerta S1 della scala NOAA, sopra 100 pfu S2-S3. Sono la componente più pericolosa per astronauti, satelliti ed elettronica in orbita. Il canale ≥1 MeV è escluso perché spesso contaminato.",
            proton_chart,
            self.help.contains("protons"),
            context.message(Message::ToggleHelp("protons".into())),
        );

        Grid::new()
            .columns([
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
            ])
            .column_spacing(12.0)
            .children((
                cell(0, dst_card),
                cell(1, bz_card),
                cell(2, wind_card),
                cell(3, proton_card),
            ))
    }

    fn solar_panel(&self, snapshot: &api::Snapshot) -> View {
        // Kp orario, ultime 24 ore
        let mut hourly: Vec<(i64, f64)> = Vec::new();
        let mut last_hour = i64::MIN;
        for (t, v) in snapshot.kp.iter().rev() {
            let hour = t.div_euclid(3600);
            if hour != last_hour {
                hourly.push((*t, *v));
                last_hour = hour;
            }
            if hourly.len() >= 24 {
                break;
            }
        }
        hourly.reverse();
        let bars: Vec<(String, f64)> = hourly
            .iter()
            .map(|(t, v)| (api::hhmm_utc(*t), *v))
            .collect();
        let kp_span_h = match (hourly.first(), hourly.last()) {
            (Some(first), Some(last)) => ((last.0 - first.0) as f64 / 3600.0).round().max(1.0),
            _ => 0.0,
        };
        let kp_title = if kp_span_h > 0.0 {
            format!("Indice Kp · medie orarie · ultime {kp_span_h:.0} h")
        } else {
            "Indice Kp · medie orarie".to_string()
        };

        // X-ray, ultime 6 ore
        let xray_points = downsample(&snapshot.xray, 90);
        let xray_series = if xray_points.len() > 1 {
            let t0 = xray_points[0].0;
            vec![Series {
                points: xray_points
                    .iter()
                    .map(|(t, v)| ((*t - t0) as f64 / 3600.0, *v))
                    .collect(),
                color: AMBER,
                thickness: 1.6,
                max_gap: 0.08,
            }]
        } else {
            vec![]
        };
        let xray_hours = xray_points
            .last()
            .map(|p| (p.0 - xray_points[0].0) as f64 / 3600.0)
            .unwrap_or(6.0);

        let metrics = Grid::new()
            .columns([
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
                GridLength::STAR,
            ])
            .column_spacing(8.0)
            .children((
                cell(0, mini("VENTO", snapshot.wind_speed.map(|v| format!("{v:.0} km/s")), VIOLET)),
                cell(1, mini("Bz", snapshot.bz_nt.map(|v| format!("{v:+.1} nT")), VIOLET)),
                cell(2, mini("Dst", snapshot.dst_nt.map(|v| format!("{v:+.0} nT")), CYAN)),
                cell(3, mini("PROTONI ≥10 MeV", snapshot.proton_pfu.map(|v| format!("{v:.2} pfu")), AMBER)),
            ));

        StackPanel::new().spacing(10.0).children((
            TextBlock::new()
                .text(kp_title)
                .font_size(10.5)
                .foreground(MUTED),
            scaled(charts::kp_bars(608.0, 108.0, &bars)),
            TextBlock::new()
                .text("Flusso X-ray GOES · banda 0.1-0.8 nm")
                .font_size(10.5)
                .foreground(MUTED),
            scaled(charts::plot(
                608.0,
                92.0,
                Axis::linear(
                    0.0,
                    xray_hours.max(1.0),
                    vec![(0.0, "-6h".into()), (xray_hours / 2.0, "-3h".into()), (xray_hours, "ora".into())],
                ),
                Axis::log(
                    1e-8,
                    1e-4,
                    vec![
                        (1e-8, "A".into()),
                        (1e-7, "B".into()),
                        (1e-6, "C".into()),
                        (1e-5, "M".into()),
                        (1e-4, "X".into()),
                    ],
                ),
                xray_series,
                vec![],
                false,
            )),
            metrics,
            TextBlock::new()
                .text("Kp misura il disturbo geomagnetico: ≥5 indica una tempesta. Le classi X-ray vanno da A (quiete) a X (brillamento forte).")
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn altitude_panel(&self, snapshot: &api::Snapshot, context: &mut ViewContext<Self>) -> View {
        let (alt, label) = ALTITUDE_CHOICES[self.altitude_index];
        let modulation = snapshot.gcr_now().unwrap_or(0.0);
        let dose = modeled_dose(alt, modulation);
        let ground = dose_ush(0.0);
        let pressure = pressure_hpa(alt);

        let curve: Vec<(f64, f64)> = (0..=160)
            .map(|i| {
                let km = i as f64 * 0.25;
                (km, dose_ush(km * 1000.0))
            })
            .collect();

        let ticks = vec![
            (0.05, "0.05".into()),
            (0.2, "0.2".into()),
            (0.5, "0.5".into()),
            (1.0, "1".into()),
            (2.0, "2".into()),
            (5.0, "5".into()),
        ];

        let selector = ComboBox::new()
            .items_source(ALTITUDE_CHOICES.iter().map(|(_, label)| label.to_string()))
            .selected_index(Some(self.altitude_index))
            .on_selection_changed(context.callback(Message::SetAltitude))
            .width(320.0);

        let readouts = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(26.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                stat("DOSE", format!("{dose:.3} µSv/h"), ROSE),
                stat("RISPETTO AL SUOLO", format!("×{:.1}", dose / ground), CYAN),
                stat("PRESSIONE ISA", format!("{pressure:.0} hPa"), MUTED),
                stat("1 mSv in", format!("{:.0} h", 1000.0 / dose.max(0.0001)), VIOLET),
            ));

        let controls = Grid::new()
            .columns([GridLength::Auto, GridLength::STAR])
            .column_spacing(18.0)
            .children((
                selector.vertical_alignment(VerticalAlignment::Center),
                cell(1, readouts),
            ));

        let marker_label = format!("{dose:.2} µSv/h · {label}");
        let chart = scaled(charts::plot(
            872.0,
            286.0,
            Axis::linear(
                0.0,
                40.0,
                vec![
                    (0.0, "0".into()),
                    (5.0, "5 km".into()),
                    (10.0, "10".into()),
                    (15.0, "15".into()),
                    (20.0, "20".into()),
                    (30.0, "30".into()),
                    (40.0, "40".into()),
                ],
            ),
            Axis::log(0.025, 6.0, ticks),
            vec![
                Series { points: curve, color: ROSE, thickness: 2.0, max_gap: 1.0 },
                Series {
                    points: vec![(0.0, ground), (40.0, ground)],
                    color: Color::rgb(71, 85, 105),
                    thickness: 1.0,
                    max_gap: 100.0,
                },
            ],
            vec![Marker {
                x: alt / 1000.0,
                y: dose,
                color: AMBER,
                label: marker_label,
            }],
            false,
        ));

        StackPanel::new().spacing(10.0).children((
            controls,
            chart,
            TextBlock::new()
                .text("Il massimo di Pfotzer (≈15 km) è il picco di ionizzazione prodotto dallo sciame secondario. Scala verticale logaritmica.")
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn map_panel(&self, snapshot: &api::Snapshot) -> View {
        let modulation = snapshot.gcr_now().unwrap_or(0.0);
        let cols = 60;
        let rows = 30;
        let cell_w = MAP_W / cols as f64;
        let cell_h = MAP_H / rows as f64;

        let mut items: Vec<KeyedView> = Vec::new();
        let mut key = 0usize;

        for row in 0..rows {
            for col in 0..cols {
                let lat = 90.0 - (row as f64 + 0.5) * (180.0 / rows as f64);
                let rc = cutoff_from_lat(lat);
                let value = modeled_intensity(rc, 0.0, modulation);
                let (r, g, b, a) = intensity_color(value);
                items.push(KeyedView::new(
                    key,
                    Rectangle::new()
                        .width(cell_w + 0.5)
                        .height(cell_h + 0.5)
                        .fill(Color::argb(a, r, g, b))
                        .canvas_left(col as f64 * cell_w)
                        .canvas_top(row as f64 * cell_h),
                ));
                key += 1;
            }
        }

        items.push(KeyedView::new(
            key,
            Image::new()
                .source_data(EncodedImage::from_static(WORLD_MAP))
                .stretch(Stretch::Fill)
                .width(MAP_W)
                .height(MAP_H)
                .canvas_left(0.0)
                .canvas_top(0.0),
        ));
        key += 1;

        for station in STATIONS {
            let x = (station.lon + 180.0) / 360.0 * MAP_W;
            let y = (90.0 - station.lat) / 180.0 * MAP_H;
            items.push(KeyedView::new(
                key,
                Ellipse::new()
                    .width(10.0)
                    .height(10.0)
                    .fill(Color::rgb(253, 230, 138))
                    .stroke(Color::rgb(8, 12, 18))
                    .stroke_thickness(1.5)
                    .canvas_left(x - 5.0)
                    .canvas_top(y - 5.0),
            ));
            key += 1;
        }

        let hx = (HOME_LON + 180.0) / 360.0 * MAP_W;
        let hy = (90.0 - HOME_LAT) / 180.0 * MAP_H;
        items.push(KeyedView::new(
            key,
            Ellipse::new()
                .width(24.0)
                .height(24.0)
                .fill(Color::transparent())
                .stroke(CYAN)
                .stroke_thickness(3.0)
                .canvas_left(hx - 12.0)
                .canvas_top(hy - 12.0),
        ));
        key += 1;
        items.push(KeyedView::new(
            key,
            TextBlock::new()
                .text("Roma")
                .font_size(14.0)
                .foreground(CYAN)
                .canvas_left(hx + 14.0)
                .canvas_top(hy - 24.0)
                .width(90.0),
        ));

        let canvas = scaled(
            Canvas::new()
                .width(MAP_W)
                .height(MAP_H)
                .keyed_children(items),
        );

        let mut swatches: Vec<KeyedView> = Vec::new();
        for (index, value) in [0.20_f64, 0.42, 0.62, 0.80, 1.10].iter().enumerate() {
            let (r, g, b, _) = intensity_color(*value);
            swatches.push(KeyedView::new(
                index,
                Rectangle::new().width(34.0).height(7.0).fill(Color::rgb(r, g, b)),
            ));
        }
        let legend = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                TextBlock::new()
                    .text("intensità relativa, bassa")
                    .font_size(9.5)
                    .foreground(MUTED),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .keyed_children(swatches),
                TextBlock::new().text("alta").font_size(9.5).foreground(MUTED),
            ));

        StackPanel::new().spacing(8.0).children((
            canvas,
            legend,
            TextBlock::new()
                .text(format!(
                    "Intensità modellata dai raggi cosmici al suolo (taglio geomagnetico dipolare + modulazione solare {:+.2}%). Punti = neutron monitor della rete NMDB. Massima alle alte latitudini, minima all'equatore magnetico.",
                    modulation
                ))
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }
}

fn cell(column: i32, view: View) -> View {
    Border::new().grid_column(column).content(view)
}

/// Come `cell`, ma allineato in alto (la scheda non si allunga a tutta la riga).
fn cell_top(column: i32, view: View) -> View {
    Border::new()
        .grid_column(column)
        .vertical_alignment(VerticalAlignment::Top)
        .content(view)
}

/// Come `cell`, ma centrato verticalmente nella riga.
fn cell_v(column: i32, view: View) -> View {
    Border::new()
        .grid_column(column)
        .vertical_alignment(VerticalAlignment::Center)
        .content(view)
}

/// Scala un disegno a dimensione fissa sulla larghezza disponibile, mantenendo le proporzioni.
fn scaled(view: View) -> View {
    Viewbox::new()
        .stretch(Stretch::Uniform)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Top)
        .slot(ViewboxSlot::Child, view)
}

fn help_panel(desc: &str) -> View {
    Border::new()
        .background(Color::rgb(19, 30, 42))
        .border_brush(Color::rgb(44, 60, 78))
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(8.0)
        .padding(Thickness::uniform(10.0))
        .content(
            TextBlock::new()
                .text(desc)
                .font_size(11.0)
                .foreground(Color::rgb(203, 213, 225))
                .text_wrapping(TextWrapping::Wrap),
        )
}

fn help_button(on_toggle: Callback<()>) -> View {
    Button::new()
        .on_click(on_toggle)
        .width(34.0)
        .height(30.0)
        .content("?")
}

fn card(
    title: &str,
    accent: Color,
    desc: &str,
    body: View,
    open: bool,
    on_toggle: Callback<()>,
) -> View {
    let head = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Rectangle::new()
                        .width(4.0)
                        .height(14.0)
                        .fill(accent)
                        .radius_x(2.0)
                        .radius_y(2.0),
                    TextBlock::new()
                        .text(title)
                        .font_size(11.0)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .foreground(Color::rgb(148, 163, 184))
                        .vertical_alignment(VerticalAlignment::Center),
                )),
            cell_v(1, help_button(on_toggle)),
        ));

    let stack = StackPanel::new().spacing(10.0);
    let inner = if open {
        let help = help_panel(desc);
        stack.children((head, help, body))
    } else {
        stack.children((head, body))
    };

    Border::new()
        .background(CARD_BG)
        .border_brush(CARD_BORDER)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(12.0)
        .padding(Thickness::uniform(14.0))
        .content(inner)
}

fn kpi_card(
    label: &str,
    value: String,
    unit: &str,
    sub: String,
    accent: Color,
    desc: &str,
    open: bool,
    on_toggle: Callback<()>,
) -> View {
    let value_row = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .children((
            TextBlock::new()
                .text(value)
                .font_size(26.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(accent),
            TextBlock::new()
                .text(unit)
                .font_size(11.0)
                .foreground(MUTED)
                .vertical_alignment(VerticalAlignment::Bottom),
        ));

    let head = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            TextBlock::new()
                .text(label)
                .font_size(10.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(MUTED)
                .vertical_alignment(VerticalAlignment::Center),
            cell_v(1, help_button(on_toggle)),
        ));

    let stack = StackPanel::new().spacing(4.0);
    let inner = if open {
        let help = help_panel(desc);
        stack.children((head, help, value_row, TextBlock::new()
            .text(sub)
            .font_size(10.5)
            .foreground(MUTED)
            .text_wrapping(TextWrapping::Wrap)))
    } else {
        stack.children((head, value_row, TextBlock::new()
            .text(sub)
            .font_size(10.5)
            .foreground(MUTED)
            .text_wrapping(TextWrapping::Wrap)))
    };

    Border::new()
        .background(CARD_BG)
        .border_brush(CARD_BORDER)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(12.0)
        .padding(Thickness::uniform(14.0))
        .content(inner)
}

fn mini(label: &str, value: Option<String>, accent: Color) -> View {
    StackPanel::new().spacing(1.0).children((
        TextBlock::new().text(label).font_size(9.5).foreground(MUTED),
        TextBlock::new()
            .text(value.unwrap_or_else(|| "—".into()))
            .font_size(13.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .foreground(accent),
    ))
}

fn stat(label: &str, value: String, accent: Color) -> View {
    StackPanel::new().spacing(1.0).children((
        TextBlock::new().text(label).font_size(9.5).foreground(MUTED),
        TextBlock::new()
            .text(value)
            .font_size(15.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .foreground(accent),
    ))
}

fn legend_item(color: Color, label: &str) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .children((
            Rectangle::new()
                .width(14.0)
                .height(3.0)
                .fill(color)
                .radius_x(1.5)
                .radius_y(1.5),
            TextBlock::new().text(label).font_size(10.0).foreground(MUTED),
        ))
}

/// Media mobile causale sull'asse del tempo (in ore).
fn smooth_mean(points: &[(f64, f64)], window: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(points.len());
    let mut start = 0usize;
    for i in 0..points.len() {
        while points[i].0 - points[start].0 > window {
            start += 1;
        }
        let sum: f64 = points[start..=i].iter().map(|p| p.1).sum();
        out.push((points[i].0, sum / (i - start + 1) as f64));
    }
    out
}

/// Cifre significative "tonde" tra `lo` e `hi`.
fn nice_ticks(lo: f64, hi: f64, target: usize) -> Vec<f64> {
    let span = (hi - lo).max(1e-9);
    let raw = span / target.max(1) as f64;
    let mag = 10f64.powf(raw.log10().floor());
    let norm = raw / mag;
    let step = mag
        * if norm <= 1.0 {
            1.0
        } else if norm <= 2.0 {
            2.0
        } else if norm <= 5.0 {
            5.0
        } else {
            10.0
        };
    let start = (lo / step).ceil() * step;
    let mut out = Vec::new();
    for i in 0..50 {
        let v = start + step * i as f64;
        if v > hi + 1e-9 {
            break;
        }
        out.push(v);
    }
    out
}

fn axis_label(v: f64) -> String {
    if v.abs() >= 10.0 {
        format!("{v:.0}")
    } else if v.abs() >= 1.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// Grafico compatto con valore corrente in testa, per i pannelli "driver solari".
fn mini_chart(
    points: &[(i64, f64)],
    color: Color,
    zero_line: bool,
    log: Option<(f64, f64, Vec<(f64, String)>)>,
    value: String,
) -> View {
    const W: f64 = 348.0;
    const H: f64 = 130.0;
    let head = TextBlock::new()
        .text(value)
        .font_size(15.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .foreground(color);

    let pts = downsample(points, 200);
    if pts.len() < 2 {
        return StackPanel::new().spacing(2.0).children((
            head,
            TextBlock::new()
                .text("dati non disponibili")
                .font_size(10.0)
                .foreground(MUTED),
        ));
    }

    let t0 = pts[0].0;
    let mapped: Vec<(f64, f64)> = pts
        .iter()
        .map(|(t, v)| ((*t - t0) as f64 / 3600.0, *v))
        .collect();
    let span = mapped.last().map(|p| p.0).unwrap_or(1.0).max(0.1);
    let gap = (span / mapped.len() as f64 * 3.0).max(0.02);
    let xticks = vec![
        (0.0, format!("-{:.0} h", span)),
        (span / 2.0, format!("-{:.0} h", span / 2.0)),
        (span, "ora".to_string()),
    ];

    let y = if let Some((lo, hi, ticks)) = log {
        Axis::log(lo, hi, ticks)
    } else {
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for (_, v) in &mapped {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
        let pad = ((hi - lo) * 0.18).max(1.0);
        let (lo, hi) = (lo - pad, hi + pad);
        let ticks = nice_ticks(lo, hi, 4)
            .into_iter()
            .map(|v| (v, axis_label(v)))
            .collect();
        Axis::linear(lo, hi, ticks)
    };

    let series = vec![Series {
        points: mapped,
        color,
        thickness: 1.7,
        max_gap: gap,
    }];

    StackPanel::new().spacing(2.0).children((
        head,
        scaled(charts::plot(
            W,
            H,
            Axis::linear(0.0, span, xticks),
            y,
            series,
            vec![],
            zero_line,
        )),
    ))
}

fn main() {
    App::run_component::<Dashboard>(()).unwrap();
}