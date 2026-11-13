#![windows_subsystem = "windows"]

mod alerts;
mod api;
mod atlas;
mod atlas_draw;
mod atlas_ui;
mod charts;
mod model;
mod state;

use alerts::{Condition, History, Settings, Severity, Watch};
use api::KpKind;
use charts::{Axis, KpBar, Marker, Series};
use model::*;
use state::Data;
use windows_reactor::*;

pub(crate) const BG: Color = Color::rgb(7, 11, 16);
pub(crate) const CARD_BG: Color = Color::rgb(14, 21, 29);
pub(crate) const CARD_BORDER: Color = Color::rgb(30, 41, 53);
pub(crate) const TEXT: Color = Color::rgb(226, 232, 240);
pub(crate) const SOFT: Color = Color::rgb(203, 213, 225);
pub(crate) const MUTED: Color = Color::rgb(124, 139, 156);
pub(crate) const CYAN: Color = Color::rgb(34, 211, 238);
pub(crate) const AMBER: Color = Color::rgb(245, 158, 11);
pub(crate) const VIOLET: Color = Color::rgb(167, 139, 250);
pub(crate) const GREEN: Color = Color::rgb(52, 211, 153);
pub(crate) const ROSE: Color = Color::rgb(248, 113, 113);
pub(crate) const SLATE: Color = Color::rgb(100, 116, 139);

const APP_ICO: &[u8] = include_bytes!("../assets/app.ico");
pub(crate) const WORLD_MAP: &[u8] = include_bytes!("../assets/worldmap.png");
const LOGO_NMDB: &[u8] = include_bytes!("../assets/credits/nmdb.png");
const LOGO_OULU: &[u8] = include_bytes!("../assets/credits/oulu.png");
const LOGO_INAF: &[u8] = include_bytes!("../assets/credits/inaf.png");
const LOGO_KYOTO: &[u8] = include_bytes!("../assets/credits/kyoto.png");
const LOGO_NE: &[u8] = include_bytes!("../assets/credits/naturalearth.png");

const MAP_W: f64 = 1200.0;
const MAP_H: f64 = 600.0;
const TICK_S: u64 = 30;

const NMDB_ACK: &str = "We acknowledge the NMDB database (www.nmdb.eu), founded under the European Union's FP7 programme (contract no. 213007), and the PIs of individual neutron monitors at: Oulu (Sodankylä Geophysical Observatory of the University of Oulu, Finland) and Rome (SVIRCO Observatory, INAF/IFSI-UniRomaTre, Italy) for providing data.";

/// Percorso dell'icona della finestra: `app.ico` accanto all'exe, altrimenti la copia
/// incorporata viene scritta nella cartella dati (l'exe funziona anche da solo).
fn icon_path() -> &'static str {
    static PATH: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let leak = |p: std::path::PathBuf| -> &'static str {
            Box::leak(p.to_string_lossy().into_owned().into_boxed_str())
        };
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
            let candidate = dir.join("app.ico");
            if candidate.exists() {
                return leak(candidate);
            }
        }
        let dir = alerts::data_dir();
        let target = dir.join("app.ico");
        if std::fs::metadata(&target).map(|m| m.len() as usize) .ok() != Some(APP_ICO.len()) {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&target, APP_ICO);
        }
        leak(target)
    })
}

fn downsample<T: Copy>(items: &[T], max: usize) -> Vec<T> {
    if items.len() <= max || max < 2 {
        return items.to_vec();
    }
    let step = items.len() as f64 / max as f64;
    let mut out: Vec<T> = (0..max).map(|i| items[(i as f64 * step) as usize]).collect();
    if let Some(last) = items.last() {
        *out.last_mut().unwrap() = *last;
    }
    out
}

pub(crate) fn color3((r, g, b): (u8, u8, u8)) -> Color {
    Color::rgb(r, g, b)
}

/// Numero con la virgola decimale (stile italiano).
pub(crate) fn it(value: f64, decimals: usize) -> String {
    format!("{value:.decimals$}").replace('.', ",")
}

pub(crate) fn it_signed(value: f64, decimals: usize) -> String {
    format!("{value:+.decimals$}").replace('.', ",")
}

/// Pagina visibile nella finestra.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Atlas,
}

struct Dashboard {
    data: Data,
    now: i64,
    in_flight: bool,
    pending_force: bool,
    settings: Settings,
    help: std::collections::HashSet<String>,
    watch: Watch,
    active: Vec<Condition>,
    history: History,
    map: Option<(i64, EncodedImage)>,
    show_ack: bool,
    map_big: bool,
    page: Page,
    atlas: atlas_ui::AtlasState,
}

#[derive(Clone)]
enum Message {
    Tick,
    Refresh,
    Fetched(Box<Vec<api::FetchResult>>),
    SetAltitude(Option<usize>),
    SetPlace(Option<usize>),
    SetToasts(bool),
    ToggleHelp(String),
    ToggleAck,
    ToggleMap,
    OpenDataDir,
    TestToast,
    SetPage(Page),
    AtlasSource(Option<usize>),
    AtlasSelect(Option<usize>),
    AtlasScrub(f64),
    AtlasTogglePlay,
    AtlasSpeed(Option<usize>),
    AtlasTick,
    AtlasLive(Box<Result<atlas::Grid, String>>),
    AtlasRefresh,
}

impl Component for Dashboard {
    type Input = ();
    type Message = Message;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        let mut dashboard = Self {
            data: Data::default(),
            now: api::now_epoch(),
            in_flight: false,
            pending_force: false,
            settings: Settings::load(),
            help: Default::default(),
            watch: Watch::default(),
            active: Vec::new(),
            history: History::open(),
            map: None,
            show_ack: true,
            map_big: false,
            page: Page::Dashboard,
            atlas: atlas_ui::AtlasState::new(),
        };
        dashboard.refresh_map();
        dashboard.start_fetch(context, true);
        // un solo timer per tutta la vita dell'app: si rischedula solo da `Tick`
        schedule_tick(context);
        dashboard
    }

    fn update(&mut self, message: Message, context: &ComponentContext<Self>) {
        self.now = api::now_epoch();
        match message {
            Message::Tick => {
                schedule_tick(context);
                self.start_fetch(context, false);
                if self.page == Page::Atlas {
                    self.fetch_atlas_live(context, false);
                }
            }
            Message::Refresh => {
                if self.in_flight {
                    self.pending_force = true;
                } else {
                    self.start_fetch(context, true);
                }
            }
            Message::Fetched(results) => {
                self.in_flight = false;
                self.data.apply(*results, self.now);
                let (active, notify) = self.watch.update(&self.data, self.now);
                self.active = active;
                if self.settings.toasts {
                    for c in notify {
                        alerts::toast(&c.title, &c.text);
                    }
                }
                self.history.record(&self.data, self.now);
                self.refresh_map();
                if std::mem::take(&mut self.pending_force) {
                    self.start_fetch(context, true);
                }
            }
            Message::SetAltitude(Some(index)) if index < ALTITUDE_CHOICES.len() => {
                self.settings.altitude = index;
                self.settings.save();
            }
            Message::SetPlace(Some(index)) if index < PLACES.len() => {
                self.settings.place = index;
                self.settings.save();
            }
            Message::SetToasts(on) => {
                self.settings.toasts = on;
                self.settings.save();
            }
            Message::ToggleHelp(id) => {
                if !self.help.remove(&id) {
                    self.help.insert(id);
                }
            }
            Message::ToggleAck => self.show_ack = !self.show_ack,
            Message::ToggleMap => self.map_big = !self.map_big,
            Message::OpenDataDir => {
                let dir = alerts::data_dir();
                let _ = std::fs::create_dir_all(&dir);
                let _ = std::process::Command::new("explorer.exe").arg(dir).spawn();
            }
            Message::TestToast => alerts::toast(
                "Notifica di prova",
                "Le notifiche funzionano: riceverai un avviso per tempeste, brillamenti, protoni, cali di Forbush e GLE.",
            ),
            Message::SetPage(page) => {
                if self.page != page {
                    self.page = page;
                    self.atlas.set_selected(None);
                    if page == Page::Atlas {
                        self.fetch_atlas_live(context, false);
                    }
                }
            }
            Message::AtlasSource(index) => {
                if let Some(index) = index {
                    self.atlas.set_source(index);
                }
            }
            Message::AtlasSelect(index) => self.atlas.set_selected(index),
            Message::AtlasScrub(value) => self.atlas.scrub(value),
            Message::AtlasTogglePlay => {
                self.atlas.toggle_play();
                if self.atlas.playing {
                    schedule_atlas_tick(context);
                }
            }
            Message::AtlasSpeed(index) => {
                if let Some(index) = index {
                    self.atlas.speed = index.min(atlas_ui::SPEEDS.len() - 1);
                }
            }
            Message::AtlasTick => {
                if self.atlas.advance() {
                    schedule_atlas_tick(context);
                }
            }
            Message::AtlasLive(result) => {
                self.atlas.live_in_flight = false;
                match *result {
                    Ok(grid) => {
                        let dataset = atlas::Dataset::from_grid(grid, true, None);
                        let status = format!("download {} UTC", api::hhmm(self.now));
                        self.atlas.live_error = None;
                        self.atlas.set_live(dataset, status);
                    }
                    Err(error) => {
                        self.atlas.live_error = Some(error);
                    }
                }
            }
            Message::AtlasRefresh => self.fetch_atlas_live(context, true),
            Message::SetAltitude(None) | Message::SetPlace(None) => {}
            Message::SetAltitude(Some(_)) | Message::SetPlace(Some(_)) => {}
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("ECHO Cosmic Ray · radiazione cosmica e meteo spaziale");
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

        let body: View = match self.page {
            Page::Dashboard => self.dashboard_page(context),
            Page::Atlas => self.atlas_page(context),
        };
        let content = StackPanel::new()
            .spacing(16.0)
            .margin(Thickness::new(24.0, 18.0, 24.0, 26.0))
            .children((self.nav(context), body));

        Border::new().background(BG).content(
            ScrollViewer::new()
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .content(content),
        )
    }
}

impl Dashboard {
    /// Barra delle pagine: dashboard classica e atlante.
    fn nav(&self, context: &mut ViewContext<Self>) -> View {
        let tab = |label: &str, page: Page| -> View {
            let active = self.page == page;
            let mut button = if active {
                Button::new().style(ButtonStyle::Accent)
            } else if page == Page::Atlas {
                // l'atlante ha un colore suo, così si vede subito dove andare
                Button::new()
                    .style(ButtonStyle::Default)
                    .resource_overrides(blue_overrides())
            } else {
                Button::new().style(ButtonStyle::Subtle)
            };
            if !active {
                button = button.on_click(context.message(Message::SetPage(page)));
            }
            button.content(label.to_string())
        };
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                tab("Dashboard", Page::Dashboard),
                tab("Atlante dei raggi cosmici", Page::Atlas),
            ))
    }

    fn dashboard_page(&self, context: &mut ViewContext<Self>) -> View {
        StackPanel::new()
            .spacing(16.0)
            .children((
                self.header(context),
                self.events_strip(),
                self.kpi_row(context),
                self.row_main(context),
                self.driver_row(context),
                self.row_month(context),
                self.row_altitude_map(context),
                self.credits(context),
                self.footer(context),
            ))
    }

    fn atlas_page(&self, context: &mut ViewContext<Self>) -> View {
        let callbacks = atlas_ui::AtlasCallbacks {
            source: context.callback(Message::AtlasSource),
            select_station: context.callback(Message::AtlasSelect),
            scrub: context.callback(Message::AtlasScrub),
            toggle_play: context.message(Message::AtlasTogglePlay),
            speed: context.callback(Message::AtlasSpeed),
            refresh: context.message(Message::AtlasRefresh),
            help_map: context.message(Message::ToggleHelp("atl-map".into())),
            help_multiples: context.message(Message::ToggleHelp("atl-multiples".into())),
            help_solar: context.message(Message::ToggleHelp("atl-solar".into())),
            help_heatmap: context.message(Message::ToggleHelp("atl-heat".into())),
            help_spectrum: context.message(Message::ToggleHelp("atl-spectrum".into())),
            help_events: context.message(Message::ToggleHelp("atl-events".into())),
            open_dashboard: context.message(Message::SetPage(Page::Dashboard)),
        };
        self.atlas.page(&callbacks, &self.help)
    }

    /// Scarica la rete completa: all'apertura della pagina e poi ogni 30 minuti.
    fn fetch_atlas_live(&mut self, context: &ComponentContext<Self>, force: bool) {
        if self.atlas.live_in_flight {
            return;
        }
        let now = api::now_epoch();
        if !force && self.atlas.live_last != 0 && now - self.atlas.live_last < 1800 {
            return;
        }
        self.atlas.live_last = now;
        self.atlas.live_in_flight = true;
        context.spawn_background(|_cancel| {
            Message::AtlasLive(Box::new(api::fetch_atlas_live()))
        });
    }
}

fn schedule_tick(context: &ComponentContext<Dashboard>) {
    context.spawn_background(|_cancel| {
        std::thread::sleep(std::time::Duration::from_secs(TICK_S));
        Message::Tick
    });
}

/// Un fotogramma dell'animazione dell'atlante (≈8 al secondo). Il compito si
/// rischedula da `AtlasTick` finché la riproduzione è attiva.
fn schedule_atlas_tick(context: &ComponentContext<Dashboard>) {
    context.spawn_background(|cancel| {
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_millis(120) {
            if cancel.is_cancelled() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        Message::AtlasTick
    });
}

impl Dashboard {
    fn start_fetch(&mut self, context: &ComponentContext<Self>, force: bool) {
        if self.in_flight {
            return;
        }
        let requests = self.data.due(self.now, force);
        if requests.is_empty() {
            return;
        }
        self.in_flight = true;
        context.spawn_background(move |_cancel| Message::Fetched(Box::new(api::fetch(&requests))));
    }

    /// La mappa si ricalcola solo quando la modulazione cambia di almeno 0,2 punti.
    fn refresh_map(&mut self) {
        let modulation = self.data.modulation_pct(self.now);
        let key = (modulation * 5.0).round() as i64;
        if self.map.as_ref().map(|m| m.0) != Some(key) {
            let bmp = intensity_bmp(360, 180, key as f64 / 5.0, (14, 21, 29));
            self.map = Some((key, EncodedImage::new(bmp)));
        }
    }

    fn place(&self) -> &'static Place {
        &PLACES[self.settings.place.min(PLACES.len() - 1)]
    }

    fn header(&self, context: &mut ViewContext<Self>) -> View {
        let title = StackPanel::new().spacing(1.0).children((
            TextBlock::new()
                .text("ECHO COSMIC RAY")
                .font_size(10.5)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(CYAN),
            TextBlock::new()
                .text("RADIAZIONE COSMICA")
                .font_size(22.0)
                .font_weight(FontWeight::BOLD)
                .foreground(TEXT),
            TextBlock::new()
                .text("raggi cosmici al suolo e in quota · meteo spaziale · dati live NMDB e NOAA")
                .font_size(12.0)
                .foreground(MUTED),
        ));

        let errors = self.data.errors.len();
        let status_text = if self.in_flight && !self.data.has_any() {
            "primo scaricamento…".to_string()
        } else if let Some(t) = self.data.latest_update() {
            let local = t + api::italy_offset(t);
            format!(
                "{}aggiornato {} UTC · {} ora italiana{}",
                if self.in_flight { "↻ " } else { "" },
                api::hhmm(t),
                api::hhmm(local),
                if errors > 0 { format!(" · {errors} fonti in ritardo") } else { String::new() }
            )
        } else {
            "in attesa dei dati".to_string()
        };
        let status_color = if !self.data.has_any() {
            MUTED
        } else if errors > 0 {
            AMBER
        } else {
            GREEN
        };

        let status = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(10.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                Ellipse::new()
                    .width(8.0)
                    .height(8.0)
                    .fill(status_color)
                    .vertical_alignment(VerticalAlignment::Center),
                TextBlock::new()
                    .text(status_text)
                    .font_size(11.0)
                    .foreground(MUTED)
                    .vertical_alignment(VerticalAlignment::Center),
                TextBlock::new()
                    .text("Notifiche")
                    .font_size(11.0)
                    .foreground(MUTED)
                    .vertical_alignment(VerticalAlignment::Center),
                ToggleSwitch::new()
                    .is_on(self.settings.toasts)
                    .on_toggled(context.callback(Message::SetToasts))
                    .tooltip("Notifiche di Windows per tempeste geomagnetiche e di radiazione, brillamenti, cali di Forbush e GLE"),
                Button::new()
                    .on_click(context.message(Message::Refresh))
                    .content("Aggiorna")
                    .tooltip("Riscarica subito tutte le fonti"),
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

    /// Eventi in corso (o una riga tranquilla) con le scale NOAA attuali.
    fn events_strip(&self) -> View {
        let scales_line = match &self.data.scales {
            Some(s) => format!("Scale NOAA adesso: R{} · S{} · G{}", s.r, s.s, s.g),
            None => "Scale NOAA: in attesa".to_string(),
        };
        if self.active.is_empty() {
            return Border::new()
                .background(Color::rgb(12, 26, 24))
                .border_brush(Color::rgb(22, 60, 50))
                .border_thickness(Thickness::uniform(1.0))
                .corner_radius(10.0)
                .padding(Thickness::new(14.0, 8.0, 14.0, 8.0))
                .content(
                    TextBlock::new()
                        .text(if self.data.loaded_once() {
                            format!("● Nessun evento significativo in corso · {scales_line}")
                        } else {
                            format!("● Raccolta dati in corso · {scales_line}")
                        })
                        .font_size(11.5)
                        .foreground(GREEN),
                );
        }
        let bars: Vec<KeyedView> = self
            .active
            .iter()
            .map(|c| {
                KeyedView::new(
                    c.id,
                    InfoBar::new()
                        .title(c.title.clone())
                        .message(c.text.clone())
                        .severity(match c.severity {
                            Severity::Info => InfoBarSeverity::Informational,
                            Severity::Warning => InfoBarSeverity::Warning,
                            Severity::Error => InfoBarSeverity::Error,
                        })
                        .is_open(true)
                        .is_closable(false),
                )
            })
            .collect();
        StackPanel::new().spacing(6.0).children((
            StackPanel::new().spacing(6.0).keyed_children(bars),
            TextBlock::new().text(scales_line).font_size(10.5).foreground(MUTED),
        ))
    }

    fn kpi_row(&self, context: &mut ViewContext<Self>) -> View {
        let now = self.now;
        let d = &self.data;
        let (alt, _) = ALTITUDE_CHOICES[self.settings.altitude.min(ALTITUDE_CHOICES.len() - 1)];
        let place = self.place();
        let modulation = d.modulation_pct(now);

        let vs_min = d.oulu_vs_min(now);
        let vs_month = d.oulu_vs_month(now);
        let kp = d.kp_now(now);
        let xray = d.xray_now(now);
        let wind = d.wind_now(now);
        let bz = d.bz_now(now);

        let (dose_value, dose_sub) = if d.has_any() {
            (
                it(modeled_dose(alt, place.rc, modulation), 3),
                format!(
                    "{} · {alt:.0} m · al suolo {} µSv/h",
                    place.name,
                    it(modeled_dose(place.altitude_m, place.rc, modulation), 3)
                ),
            )
        } else {
            ("—".to_string(), "in attesa".to_string())
        };

        let cards = (
            kpi_card(
                "RAGGI COSMICI · OULU",
                vs_min.map(|v| it_signed(v, 1)).unwrap_or("—".into()),
                "% vs minimo solare",
                match vs_month {
                    Some(m) => format!("rispetto ai 30 giorni {} % · media 1 h", it_signed(m, 2)),
                    None => "neutron monitor · media 1 h".into(),
                },
                CYAN,
                "Intensità dei raggi cosmici al suolo misurata dal neutron monitor di Oulu (Finlandia, 65°N), confrontata con il record di dicembre 2009 (minimo solare: 114,07 conteggi/s). Il valore è sempre negativo e dice quanto il Sole sta 'schermando' la Terra: vicino allo 0 % al minimo del ciclo, −15/−20 % al massimo. Sotto, la variazione di breve periodo rispetto alla media degli ultimi 30 giorni: lì si vedono i cali di Forbush. Fonte: NMDB.",
                self.help.contains("kpi-gcr"),
                context.message(Message::ToggleHelp("kpi-gcr".into())),
            ),
            kpi_card(
                "INDICE KP",
                kp.map(|v| it(v, 2)).unwrap_or("—".into()),
                "Kp",
                kp.map(|v| kp_storm(v).to_string()).unwrap_or("—".into()),
                kp.map(|v| color3(kp_color(v))).unwrap_or(MUTED),
                "Indice Kp (0-9) del disturbo geomagnetico planetario, in terzi di unità: Kp 0-3 quiete, 4 attivo, ≥5 tempesta (scala NOAA G1-G5). Il valore è la stima al minuto di NOAA SWPC; nel pannello Attività solare ci sono i blocchi ufficiali di 3 ore e la previsione dei prossimi 3 giorni.",
                self.help.contains("kpi-kp"),
                context.message(Message::ToggleHelp("kpi-kp".into())),
            ),
            kpi_card(
                "VENTO SOLARE",
                wind.map(|v| format!("{v:.0}")).unwrap_or("—".into()),
                "km/s",
                match (bz, d.density_now(now)) {
                    (Some(bz), Some(n)) => format!("Bz {} nT · densità {} p/cm³", it_signed(bz, 1), it(n, 1)),
                    (Some(bz), None) => format!("Bz {} nT · IMF", it_signed(bz, 1)),
                    _ => "IMF non ancora scaricata".into(),
                },
                VIOLET,
                "Velocità del plasma solare a monte della Terra (punto L1) misurata da SOLAR-1/ACE, con la componente Bz del campo magnetico interplanetario e la densità. Vento oltre 600 km/s indica un flusso veloce o una CME; Bz negativo (campo verso sud) permette al vento di accoppiarsi alla magnetosfera e innescare tempeste.",
                self.help.contains("kpi-wind"),
                context.message(Message::ToggleHelp("kpi-wind".into())),
            ),
            kpi_card(
                "FLUSSO X-RAY GOES",
                xray.map(flare_class).unwrap_or("—".into()),
                "classe",
                xray.map(|v| format!("{v:.2e} W/m² · R{}", r_scale(v))).unwrap_or("—".into()),
                AMBER,
                "Flusso di raggi X nella banda 0,1-0,8 nm misurato dai satelliti GOES. Le classi vanno da A (1e-8 W/m²) a X (1e-4 W/m²), ognuna dieci volte la precedente: C e M indicano attività crescente, X un brillamento forte (scala NOAA R3 e oltre). La radiazione arriva in 8 minuti e può precedere un aumento dei protoni energetici.",
                self.help.contains("kpi-xray"),
                context.message(Message::ToggleHelp("kpi-xray".into())),
            ),
            kpi_card(
                "DOSE · QUOTA SCELTA",
                dose_value,
                "µSv/h",
                dose_sub,
                ROSE,
                "Dose equivalente da raggi cosmici alla quota scelta sopra la località scelta (pannello Dose per quota), stimata con un modello: ancore da misure pubblicate (UNSCEAR/ICRU), effetto della latitudine geomagnetica e modulazione solare osservata a Oulu. Non è una misura.",
                self.help.contains("kpi-dose"),
                context.message(Message::ToggleHelp("kpi-dose".into())),
            ),
        );

        Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR, GridLength::STAR, GridLength::STAR])
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
        let gcr_card = card(
            "RAGGI COSMICI · ULTIME 24 ORE",
            CYAN,
            "Variazione percentuale dei conteggi di due neutron monitor rispetto alla loro media degli ultimi 30 giorni: Oulu (Finlandia, 65°N, rigidità di taglio 0,8 GV) e Roma (41°N, 6,2 GV). La linea sottile è il dato al minuto, quella spessa la media mobile a 30 minuti: la statistica dei neutroni è rumorosa (±1,5 % al minuto) e la media rende visibili le variazioni vere, come i cali di Forbush o un raro aumento al suolo (GLE). A Roma arrivano solo raggi cosmici più energetici, quindi le variazioni sono più piccole. Fonte: NMDB, aggiornamento ogni 2 minuti.",
            self.gcr_chart(),
            self.help.contains("gcr"),
            context.message(Message::ToggleHelp("gcr".into())),
        );
        let solar_card = card(
            "ATTIVITÀ SOLARE",
            AMBER,
            "In alto l'indice Kp ufficiale a blocchi di 3 ore: ultimi 2 giorni osservati (barre piene) e previsione NOAA dei prossimi 3 giorni (barre tenui). In mezzo il flusso X-ray GOES delle ultime 6 ore in scala logaritmica, con le classi A/B/C/M/X. In basso i valori più recenti di vento solare, densità, campo magnetico, Dst, protoni ≥10 MeV e flusso radio F10.7 (indicatore dell'attività solare: ~70 al minimo, 150-250 al massimo).",
            self.solar_panel(context),
            self.help.contains("solar"),
            context.message(Message::ToggleHelp("solar".into())),
        );

        Grid::new()
            .columns([GridLength::Pixel(900.0), GridLength::STAR])
            .column_spacing(16.0)
            .children((cell(0, gcr_card), cell(1, solar_card)))
    }

    fn gcr_chart(&self) -> View {
        let oulu = self.data.short_variation(false);
        let rome = self.data.short_variation(true);
        let end = oulu.last().map(|p| p.0).unwrap_or(self.now);
        let t0 = end - 24 * 3600;
        let to_h = |s: &[(i64, f64)]| -> Vec<(f64, f64)> {
            s.iter()
                .filter(|p| p.0 >= t0 - 1800)
                .map(|(t, v)| ((*t - t0) as f64 / 3600.0, *v))
                .collect()
        };
        let raw_h = to_h(&oulu);
        let rome_h = to_h(&rome);
        // la media mobile è affidabile solo dopo 15 minuti dall'inizio della serie
        let settled = |s: &[(f64, f64)]| s.first().map_or(0.0, |p| p.0.max(0.0) + 0.25).max(0.0);
        let (oulu_from, rome_from) = (settled(&raw_h), settled(&rome_h));
        let mut smooth_h = smooth_mean(&raw_h, 0.5);
        let mut rome_s = smooth_mean(&rome_h, 0.5);
        smooth_h.retain(|p| p.0 >= 0.0 && (p.0 >= oulu_from || raw_h[0].0 < -0.25));
        rome_s.retain(|p| p.0 >= 0.0 && (p.0 >= rome_from || rome_h[0].0 < -0.25));
        let raw: Vec<(f64, f64)> = raw_h.into_iter().filter(|p| p.0 >= 0.0).collect();

        let raw = downsample(&raw, 240);
        let smooth = downsample(&smooth_h, 180);
        let rome = downsample(&rome_s, 180);

        let (x, y, series, markers) = if smooth.len() < 2 {
            (
                Axis::linear(0.0, 24.0, vec![(0.0, "—".into()), (24.0, "—".into())]),
                Axis::linear(-1.0, 1.0, vec![(0.0, "0".into())]),
                vec![],
                vec![],
            )
        } else {
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for (_, v) in smooth.iter().chain(rome.iter()) {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
            let margin = ((hi - lo) * 0.25).max(0.6);
            lo -= margin;
            hi += margin;
            let ticks = nice_ticks(lo, hi, 5)
                .into_iter()
                .map(|v| (v, if v == 0.0 { "0".to_string() } else { it_signed(v, 1) }))
                .collect();
            let xticks = (0..=4)
                .map(|i| {
                    let h = i as f64 * 6.0;
                    (h, api::hhmm(t0 + (h * 3600.0) as i64))
                })
                .collect();
            let last = smooth.last().copied().unwrap_or((0.0, 0.0));
            let mut plot_series = Vec::new();
            if rome.len() > 1 {
                plot_series.push(Series { points: rome, color: AMBER, thickness: 1.7, max_gap: 0.3 });
            }
            plot_series.push(Series {
                points: raw,
                color: Color::argb(35, 34, 211, 238),
                thickness: 0.7,
                max_gap: 0.25,
            });
            plot_series.push(Series { points: smooth, color: CYAN, thickness: 2.2, max_gap: 0.3 });
            (
                Axis::linear(0.0, 24.0, xticks),
                Axis::linear(lo, hi, ticks),
                plot_series,
                vec![Marker { x: last.0, y: last.1, color: CYAN, label: format!("Oulu {} %", it_signed(last.1, 2)) }],
            )
        };

        let legend = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(16.0)
            .children((
                legend_item(CYAN, "Oulu · 0,8 GV · al minuto + media 30 min"),
                legend_item(AMBER, "Roma · 6,2 GV · media 30 min"),
                TextBlock::new().text("ore UTC").font_size(10.0).foreground(SLATE),
            ));

        StackPanel::new().spacing(6.0).children((
            legend,
            scaled(charts::plot(872.0, 232.0, x, y, series, markers, true)),
            TextBlock::new()
                .text("Variazione rispetto alla media degli ultimi 30 giorni. I raggi cosmici calano quando il Sole è più attivo, con brusche cadute (effetto Forbush) al passaggio delle nubi di plasma; un picco improvviso a Oulu può indicare un GLE, particelle solari che arrivano fino al suolo.")
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn solar_panel(&self, context: &mut ViewContext<Self>) -> View {
        let now = self.now;
        let d = &self.data;

        // Kp: blocchi di 3 ore (osservati 2 giorni + previsione), altrimenti medie orarie
        let bars: Vec<KpBar> = if !d.kp_blocks.is_empty() {
            d.kp_blocks
                .iter()
                .filter(|b| b.t >= now - 48 * 3600)
                .map(|b| KpBar {
                    label: if b.t.rem_euclid(86_400) == 0 { api::ddmm(b.t) } else { String::new() },
                    kp: b.kp,
                    predicted: b.kind == KpKind::Predicted,
                })
                .collect()
        } else {
            d.kp_hourly()
                .iter()
                .enumerate()
                .map(|(i, (t, v))| KpBar {
                    label: if i % 2 == 0 { api::hhmm(*t) } else { String::new() },
                    kp: *v,
                    predicted: false,
                })
                .collect()
        };
        let kp_title = if !d.kp_blocks.is_empty() {
            "Indice Kp · blocchi di 3 ore · 2 giorni osservati + previsione 3 giorni".to_string()
        } else {
            "Indice Kp · medie orarie".to_string()
        };

        // X-ray, ultime 6 ore
        let xray_points = downsample(&d.xray, 120);
        let xray_hours = match (xray_points.first(), xray_points.last()) {
            (Some(a), Some(b)) => ((b.0 - a.0) as f64 / 3600.0).max(0.5),
            _ => 6.0,
        };
        let xray_series = if xray_points.len() > 1 {
            let t0 = xray_points[0].0;
            vec![Series {
                points: xray_points.iter().map(|(t, v)| ((*t - t0) as f64 / 3600.0, *v)).collect(),
                color: AMBER,
                thickness: 1.6,
                max_gap: 0.1,
            }]
        } else {
            vec![]
        };
        let h_label = |h: f64| format!("-{} h", it(h, if h.fract().abs() < 0.05 { 0 } else { 1 }));

        let row1 = Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR])
            .column_spacing(8.0)
            .children((
                cell(0, mini("VENTO", d.wind_now(now).map(|v| format!("{v:.0} km/s")), VIOLET)),
                cell(1, mini("DENSITÀ", d.density_now(now).map(|v| format!("{} p/cm³", it(v, 1))), VIOLET)),
                cell(
                    2,
                    mini(
                        "Bz / Bt",
                        d.bz_now(now).map(|bz| match d.bt_now(now) {
                            Some(bt) => format!("{} / {} nT", it_signed(bz, 1), it(bt, 1)),
                            None => format!("{} nT", it_signed(bz, 1)),
                        }),
                        VIOLET,
                    ),
                ),
            ));
        let row2 = Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR])
            .column_spacing(8.0)
            .children((
                cell(0, mini("Dst", d.dst_now(now).map(|v| format!("{v:+.0} nT")), CYAN)),
                cell(1, mini("PROTONI ≥10 MeV", d.protons_now(now).map(|v| format!("{} pfu", it(v, 2))), AMBER)),
                cell(
                    2,
                    mini(
                        "F10.7",
                        d.f107_now().map(|v| match d.f107_mean() {
                            Some(m) => format!("{v:.0} sfu · 30 g {m:.0}"),
                            None => format!("{v:.0} sfu"),
                        }),
                        AMBER,
                    ),
                ),
            ));

        StackPanel::new().spacing(10.0).children((
            chart_title(
                &kp_title,
                self.help.contains("solar-kp"),
                context.message(Message::ToggleHelp("solar-kp".into())),
            ),
            if self.help.contains("solar-kp") {
                help_panel("Ogni barra è la media dell'indice Kp su 3 ore, la stessa grandezza del pannello qui sopra. Barre piene: valori osservati, ultimi due giorni. Barre tenui a destra della linea verticale: previsione NOAA per i prossimi tre giorni. Scala 0-9 in terzi di unità: sotto 4 il campo è quieto, 4 è attivo, da 5 in su è tempesta geomagnetica (G1 a Kp 5, G3 a 7, G5 a 9). Le etichette sull'asse sono ore UTC.")
            } else {
                Border::new().height(0.0).into()
            },
            scaled(charts::kp_bars(608.0, 118.0, &bars)),
            chart_title(
                "Flusso X-ray GOES · banda 0,1-0,8 nm · ultime ore",
                self.help.contains("solar-xray"),
                context.message(Message::ToggleHelp("solar-xray".into())),
            ),
            if self.help.contains("solar-xray") {
                help_panel("Flusso di raggi X molli misura­to dai satelliti GOES, in watt per metro quadrato, su scala logaritmica: ogni gradino dell'asse è un fattore 10, non una differenza fissa. Le lettere sull'asse verticale sono le classi NOAA: A sotto 1e-7, poi B, C, M e X, che è la più intensa. Le linee orizzontali sono le soglie tra una classe e l'altra, quindi un picco che attraversa una linea è un brillamento di classe superiore. Sull'asse orizzontale le ore prima dell'istante attuale.")
            } else {
                Border::new().height(0.0).into()
            },
            scaled(charts::plot(
                608.0,
                92.0,
                Axis::linear(
                    0.0,
                    xray_hours,
                    vec![(0.0, h_label(xray_hours)), (xray_hours / 2.0, h_label(xray_hours / 2.0)), (xray_hours, "ora".into())],
                ),
                Axis::log(
                    1e-8,
                    3e-4,
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
            row1,
            row2,
        ))
    }

    fn driver_row(&self, context: &mut ViewContext<Self>) -> View {
        let d = &self.data;
        let now = self.now;
        let within = |points: &[(i64, f64)], hours: i64| -> Vec<(i64, f64)> {
            points.iter().filter(|p| p.0 >= now - hours * 3600).cloned().collect()
        };

        let dst_series = within(&d.dst, 72);
        let bz_series = within(&d.bz, 24);
        let wind_series = within(&d.wind, 24);
        let proton_series = within(&d.protons, 6);

        let dst_chart = mini_chart(
            &dst_series,
            CYAN,
            true,
            None,
            dst_series.last().map(|p| format!("{:+.0} nT", p.1)).unwrap_or("—".into()),
        );
        let bz_chart = mini_chart(
            &bz_series,
            VIOLET,
            true,
            None,
            bz_series.last().map(|p| format!("{} nT", it_signed(p.1, 1))).unwrap_or("—".into()),
        );
        let wind_chart = mini_chart(
            &wind_series,
            GREEN,
            false,
            None,
            wind_series.last().map(|p| format!("{:.0} km/s", p.1)).unwrap_or("—".into()),
        );
        let proton_chart = mini_chart(
            &proton_series,
            AMBER,
            false,
            Some((
                1e-2,
                1e3,
                vec![(1e-1, "0,1".into()), (1.0, "1".into()), (10.0, "10 · S1".into()), (100.0, "100".into())],
            )),
            proton_series.last().map(|p| format!("{} pfu", it(p.1, 2))).unwrap_or("—".into()),
        );

        let cards = (
            card(
                "DST · 72 ORE",
                CYAN,
                "Indice Dst (nT): media degli scostamenti del campo geomagnetico misurati a basse latitudini, aggiornato ogni ora. Valori negativi indicano la corrente ad anello che si intensifica durante le tempeste: circa −30 nT quiete, −50 tempesta debole, −100 tempesta intensa, −300 evento estremo. Fonte: WDC per il Geomagnetismo di Kyoto, tramite NOAA SWPC.",
                dst_chart,
                self.help.contains("dst"),
                context.message(Message::ToggleHelp("dst".into())),
            ),
            card(
                "BZ IMF · 24 ORE",
                VIOLET,
                "Componente nord-sud del campo magnetico interplanetario (nT), ultime 24 ore. Bz negativo significa campo diretto a sud: si riconnette con la magnetosfera terrestre e alimenta tempeste e aurore. È il precursore più diretto di un peggioramento geomagnetico; la linea dello zero è il punto di inversione.",
                bz_chart,
                self.help.contains("bz"),
                context.message(Message::ToggleHelp("bz".into())),
            ),
            card(
                "VENTO SOLARE · 24 ORE",
                GREEN,
                "Velocità del vento solare (km/s) nelle ultime 24 ore. Valori tipici 350-450 km/s in quiete; oltre 600 km/s indicano un flusso veloce da buco coronale o il passaggio di un'espulsione coronale (ICME). Fonte: SOLAR-1/ACE via NOAA SWPC.",
                wind_chart,
                self.help.contains("wind"),
                context.message(Message::ToggleHelp("wind".into())),
            ),
            card(
                "PROTONI ≥10 MeV · 6 ORE",
                AMBER,
                "Flusso integrale di protoni ≥10 MeV (particelle per cm²·sr·s, detti pfu) misurato dai satelliti GOES. Fondo tipico 0,1-1 pfu; sopra 10 pfu scatta la tempesta di radiazione S1 della scala NOAA, sopra 100 pfu S2. Sono la componente più pericolosa per astronauti, satelliti e voli polari.",
                proton_chart,
                self.help.contains("protons"),
                context.message(Message::ToggleHelp("protons".into())),
            ),
        );

        Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR, GridLength::STAR])
            .column_spacing(12.0)
            .children((cell(0, cards.0), cell(1, cards.1), cell(2, cards.2), cell(3, cards.3)))
    }

    fn row_month(&self, context: &mut ViewContext<Self>) -> View {
        let month_card = card(
            "RAGGI COSMICI · 30 GIORNI",
            CYAN,
            "Medie orarie degli ultimi 30 giorni, in percentuale rispetto al minimo solare di ciascuna stazione (Oulu: dicembre 2009; Roma: dicembre 2020). La distanza dallo zero è la modulazione solare: quanto il vento solare sta tenendo lontani i raggi cosmici dalla Terra. I cali bruschi di qualche punto, con recupero in diversi giorni, sono decrementi di Forbush; le oscillazioni lente di ~27 giorni seguono la rotazione del Sole.",
            self.month_chart(),
            self.help.contains("month"),
            context.message(Message::ToggleHelp("month".into())),
        );
        let noaa_card = card(
            "SCALE E BOLLETTINI NOAA",
            VIOLET,
            "Scale ufficiali NOAA del meteo spaziale: R (blackout radio da brillamenti), S (tempeste di radiazione da protoni solari) e G (tempeste geomagnetiche), da 0 a 5. Sotto, le probabilità previste per i prossimi giorni e gli ultimi bollettini emessi dallo Space Weather Prediction Center (testo originale in inglese).",
            self.noaa_panel(),
            self.help.contains("noaa"),
            context.message(Message::ToggleHelp("noaa".into())),
        );
        Grid::new()
            .columns([GridLength::Pixel(900.0), GridLength::STAR])
            .column_spacing(16.0)
            .children((cell(0, month_card), cell(1, noaa_card)))
    }

    fn month_chart(&self) -> View {
        let d = &self.data;
        let end = d.oulu_month.last().map(|p| p.0).unwrap_or(self.now);
        let t0 = end - 30 * 86_400;
        let to_days = |s: &[(i64, f64)], reference: f64| -> Vec<(f64, f64)> {
            s.iter()
                .filter(|p| p.0 >= t0)
                .map(|(t, v)| ((*t - t0) as f64 / 86_400.0, (v / reference - 1.0) * 100.0))
                .collect()
        };
        let oulu = downsample(&smooth_mean(&to_days(&d.oulu_month, OULU_SOLAR_MIN), 0.25), 300);
        let rome = downsample(&smooth_mean(&to_days(&d.rome_month, ROME_SOLAR_MIN), 0.25), 300);

        let body: View = if oulu.len() < 2 {
            TextBlock::new()
                .text("Serie dei 30 giorni non ancora disponibile")
                .font_size(10.5)
                .foreground(MUTED)
                .into()
        } else {
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            for (_, v) in oulu.iter().chain(rome.iter()) {
                lo = lo.min(*v);
                hi = hi.max(*v);
            }
            let margin = ((hi - lo) * 0.15).max(0.8);
            let (lo, hi) = (lo - margin, (hi + margin).min(2.0).max(lo + 1.0));
            let ticks = nice_ticks(lo, hi, 5).into_iter().map(|v| (v, it_signed(v, 0))).collect();
            let xticks = (0..=6)
                .map(|i| {
                    let day = i as f64 * 5.0;
                    (day, api::ddmm(t0 + (day * 86_400.0) as i64))
                })
                .collect();
            let mut series = Vec::new();
            if rome.len() > 1 {
                series.push(Series { points: rome, color: AMBER, thickness: 1.6, max_gap: 0.3 });
            }
            let last = oulu.last().copied().unwrap_or((0.0, 0.0));
            series.push(Series { points: oulu, color: CYAN, thickness: 2.0, max_gap: 0.3 });
            scaled(charts::plot(
                872.0,
                220.0,
                Axis::linear(0.0, 30.0, xticks),
                Axis::linear(lo, hi, ticks),
                series,
                vec![Marker { x: last.0, y: last.1, color: CYAN, label: format!("Oulu {} %", it_signed(last.1, 1)) }],
                true,
            ))
        };

        let rome_now = d.rome_vs_min(self.now);
        StackPanel::new().spacing(6.0).children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(16.0)
                .children((
                    legend_item(CYAN, "Oulu · % rispetto al minimo solare 2009"),
                    legend_item(AMBER, "Roma · % rispetto al minimo solare 2020"),
                )),
            body,
            TextBlock::new()
                .text(format!(
                    "Medie orarie NMDB, lisciate su 6 ore. Roma oggi {} % dal suo minimo: a taglio alto il Sole modula circa la metà rispetto a Oulu.",
                    rome_now.map(|v| it_signed(v, 1)).unwrap_or("—".into())
                ))
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn noaa_panel(&self) -> View {
        let pill = |letter: &str, level: u8, name: &str| -> View {
            let c = color3(scale_color(level));
            Border::new()
                .background(Color::argb(28, c.r, c.g, c.b))
                .border_brush(Color::argb(120, c.r, c.g, c.b))
                .border_thickness(Thickness::uniform(1.0))
                .corner_radius(10.0)
                .padding(Thickness::new(12.0, 6.0, 12.0, 8.0))
                .content(StackPanel::new().spacing(0.0).children((
                    TextBlock::new()
                        .text(format!("{letter}{level}"))
                        .font_size(24.0)
                        .font_weight(FontWeight::BOLD)
                        .foreground(c),
                    TextBlock::new().text(name).font_size(9.5).foreground(MUTED),
                )))
        };
        let s = self.data.scales.clone().unwrap_or_default();
        let pills = Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR])
            .column_spacing(8.0)
            .children((
                cell(0, pill("R", s.r, "blackout radio")),
                cell(1, pill("S", s.s, "radiazione")),
                cell(2, pill("G", s.g, "geomagnetica")),
            ));

        let pct = |v: Option<u8>| v.map(|v| format!("{v} %")).unwrap_or("—".into());
        let mut outlook: Vec<KeyedView> = vec![KeyedView::new(
            "h",
            outlook_row("giorno", "R1-R2", "R3+", "S1+", "G prev.", MUTED),
        )];
        for (i, day) in s.days.iter().enumerate() {
            outlook.push(KeyedView::new(
                i,
                outlook_row(
                    &day.date,
                    &pct(day.r_minor),
                    &pct(day.r_major),
                    &pct(day.s_prob),
                    &day.g_scale.map(|g| format!("G{g}")).unwrap_or("—".into()),
                    SOFT,
                ),
            ));
        }

        let alerts: Vec<KeyedView> = self
            .data
            .alerts
            .iter()
            .filter(|a| a.t >= self.now - 3 * 86_400)
            .take(5)
            .enumerate()
            .map(|(i, a)| {
                KeyedView::new(
                    i,
                    StackPanel::new().spacing(0.0).children((
                        TextBlock::new()
                            .text(format!("{} {} UTC · {}", api::ddmm(a.t), api::hhmm(a.t), a.kind))
                            .font_size(9.5)
                            .foreground(MUTED),
                        TextBlock::new()
                            .text(a.headline.clone())
                            .font_size(11.0)
                            .foreground(SOFT)
                            .text_wrapping(TextWrapping::Wrap),
                    )),
                )
            })
            .collect();
        let alerts_view: View = if alerts.is_empty() {
            TextBlock::new()
                .text("Nessun bollettino negli ultimi 3 giorni.")
                .font_size(10.5)
                .foreground(MUTED)
                .into()
        } else {
            StackPanel::new().spacing(7.0).keyed_children(alerts).into()
        };

        StackPanel::new().spacing(10.0).children((
            pills,
            TextBlock::new().text("Probabilità previste").font_size(10.5).foreground(MUTED),
            StackPanel::new().spacing(3.0).keyed_children(outlook),
            TextBlock::new().text("Ultimi bollettini SWPC").font_size(10.5).foreground(MUTED),
            alerts_view,
        ))
    }

    fn row_altitude_map(&self, context: &mut ViewContext<Self>) -> View {
        let altitude_card = card(
            "DOSE PER QUOTA · PROFILO VERTICALE",
            ROSE,
            "Dose equivalente da raggi cosmici in funzione della quota, da 0 a 40 km, in scala logaritmica, per la località scelta (rosso) confrontata con una zona polare e una equatoriale (grigio). Il massimo attorno ai 15-18 km è il massimo di Pfotzer: lo sciame secondario è più intenso lì che al suolo o nello spazio. Ai poli il campo magnetico scherma meno, quindi in quota la dose è circa 3 volte quella all'equatore. È un modello basato su misure pubblicate (UNSCEAR/ICRU), corretto per latitudine geomagnetica e modulazione solare del momento: non una misura in tempo reale.",
            self.altitude_panel(context),
            self.help.contains("altitude"),
            context.message(Message::ToggleHelp("altitude".into())),
        );
        let zoom = Button::new()
            .on_click(context.message(Message::ToggleMap))
            .content(if self.map_big { "Riduci" } else { "Ingrandisci" })
            .tooltip(if self.map_big {
                "Riporta la mappa alla dimensione normale"
            } else {
                "Apri la mappa a tutta larghezza, con i nomi delle stazioni"
            });
        let map_card = card_with_action(
            "INTENSITÀ AL SUOLO · RETE MONDIALE",
            GREEN,
            "Intensità relativa dei raggi cosmici al suolo: massima alle alte latitudini magnetiche e minima all'equatore magnetico, perché il campo geomagnetico scherma le particelle meno energetiche. Il colore usa un dipolo inclinato (polo geomagnetico IGRF 2025, 80,9°N 72,7°W), da cui le bande curve, più la modulazione solare osservata a Oulu. Non include le anomalie locali del campo (Atlantico meridionale). I puntini gialli sono neutron monitor della rete NMDB (passa il mouse per i dettagli), il cerchio azzurro è la località scelta.",
            self.map_panel(),
            self.help.contains("map"),
            context.message(Message::ToggleHelp("map".into())),
            Some(zoom.into()),
        );

        if self.map_big {
            return StackPanel::new().spacing(16.0).children((
                map_card,
                Grid::new()
                    .columns([GridLength::Pixel(900.0), GridLength::STAR])
                    .children((cell(0, altitude_card),)),
            ));
        }
        Grid::new()
            .columns([GridLength::Pixel(900.0), GridLength::STAR])
            .column_spacing(16.0)
            .children((cell(0, altitude_card), cell(1, map_card)))
    }

    fn altitude_panel(&self, context: &mut ViewContext<Self>) -> View {
        let altitude_index = self.settings.altitude.min(ALTITUDE_CHOICES.len() - 1);
        let (alt, label) = ALTITUDE_CHOICES[altitude_index];
        let place = self.place();
        let modulation = self.data.modulation_pct(self.now);
        let dose = modeled_dose(alt, place.rc, modulation);
        let ground = modeled_dose(place.altitude_m, place.rc, modulation);
        let pressure = pressure_hpa(alt);

        let curve = |rc: f64| -> Vec<(f64, f64)> {
            (0..=160)
                .map(|i| {
                    let km = i as f64 * 0.25;
                    (km, modeled_dose(km * 1000.0, rc, modulation))
                })
                .collect()
        };

        let place_selector = ComboBox::new()
            .items_source(PLACES.iter().map(|p| format!("{} · Rc ≈ {} GV", p.name, it(p.rc, 1))))
            .selected_index(Some(self.settings.place.min(PLACES.len() - 1)))
            .on_selection_changed(context.callback(Message::SetPlace))
            .width(300.0);
        let altitude_selector = ComboBox::new()
            .items_source(ALTITUDE_CHOICES.iter().map(|(_, label)| label.to_string()))
            .selected_index(Some(altitude_index))
            .on_selection_changed(context.callback(Message::SetAltitude))
            .width(260.0);

        let readouts = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(22.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                stat("DOSE", format!("{} µSv/h", it(dose, 3)), ROSE),
                stat("RISPETTO A CASA", format!("×{}", it(dose / ground, 1)), CYAN),
                stat("PRESSIONE ISA", format!("{pressure:.0} hPa"), MUTED),
                stat("1 mSv IN", format_duration_h(1000.0 / dose.max(1e-4)), VIOLET),
            ));

        let controls = StackPanel::new().spacing(10.0).children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(10.0)
                .children((place_selector, altitude_selector)),
            readouts,
        ));

        let marker_label = format!("{} µSv/h · {label}", it(dose, 2));
        let ticks = vec![
            (0.02, "0,02".into()),
            (0.05, "0,05".into()),
            (0.2, "0,2".into()),
            (0.5, "0,5".into()),
            (1.0, "1".into()),
            (2.0, "2".into()),
            (5.0, "5".into()),
            (10.0, "10".into()),
        ];
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
            Axis::log(0.018, 12.0, ticks),
            vec![
                Series { points: curve(RC_POLAR), color: Color::rgb(96, 115, 140), thickness: 1.2, max_gap: 1.0 },
                Series { points: curve(RC_EQUATOR), color: Color::rgb(71, 85, 105), thickness: 1.2, max_gap: 1.0 },
                Series {
                    points: vec![(0.0, ground), (40.0, ground)],
                    color: Color::rgb(51, 65, 85),
                    thickness: 1.0,
                    max_gap: 100.0,
                },
                Series { points: curve(place.rc), color: ROSE, thickness: 2.2, max_gap: 1.0 },
            ],
            vec![Marker { x: alt / 1000.0, y: dose, color: AMBER, label: marker_label }],
            false,
        ));

        StackPanel::new().spacing(10.0).children((
            controls,
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(16.0)
                .children((
                    legend_item(ROSE, place.name),
                    legend_item(Color::rgb(96, 115, 140), "polare (0,8 GV, in alto)"),
                    legend_item(Color::rgb(71, 85, 105), "equatoriale (14 GV, in basso)"),
                    legend_item(Color::rgb(51, 65, 85), "dose a casa, al suolo"),
                )),
            chart,
            TextBlock::new()
                .text(format!(
                    "Modulazione solare usata dal modello: {} % rispetto alla media del ciclo (Oulu, media 30 min). Il massimo di Pfotzer (≈15-18 km) è il picco dello sciame secondario.",
                    it_signed(modulation, 1)
                ))
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn map_panel(&self) -> View {
        let modulation = self.data.modulation_pct(self.now);
        let place = self.place();
        let mut items: Vec<KeyedView> = Vec::new();
        let mut key = 0usize;

        if let Some((_, image)) = &self.map {
            items.push(KeyedView::new(
                key,
                Image::new()
                    .source_data(image.clone())
                    .stretch(Stretch::Fill)
                    .width(MAP_W)
                    .height(MAP_H)
                    .canvas_left(0.0)
                    .canvas_top(0.0),
            ));
        }
        key += 1;
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

        let project = |lat: f64, lon: f64| ((lon + 180.0) / 360.0 * MAP_W, (90.0 - lat) / 180.0 * MAP_H);
        for station in STATIONS {
            let (x, y) = project(station.lat, station.lon);
            items.push(KeyedView::new(
                key,
                Ellipse::new()
                    .width(12.0)
                    .height(12.0)
                    .fill(Color::rgb(253, 230, 138))
                    .stroke(Color::rgb(8, 12, 18))
                    .stroke_thickness(1.5)
                    .canvas_left(x - 6.0)
                    .canvas_top(y - 6.0)
                    .tooltip(format!(
                        "{} ({}) · Rc {} GV · {:.0} m s.l.m.",
                        station.name,
                        station.country,
                        it(station.rc, 2),
                        station.altitude_m
                    )),
            ));
            key += 1;
            if self.map_big {
                items.push(KeyedView::new(
                    key,
                    TextBlock::new()
                        .text(format!("{} · {} GV", station.name, it(station.rc, 1)))
                        .font_size(9.0)
                        .foreground(Color::rgb(253, 230, 138))
                        // vicino al bordo destro l'etichetta va a sinistra del punto (larghezza stimata)
                        .canvas_left(if x > MAP_W - 150.0 {
                            x - 9.0 - 5.2 * (station.name.chars().count() + 7) as f64
                        } else {
                            x + 7.0
                        })
                        .canvas_top((y - 6.0).min(MAP_H - 14.0))
                        .width(150.0),
                ));
                key += 1;
            }
        }

        let (hx, hy) = project(place.lat, place.lon);
        items.push(KeyedView::new(
            key,
            Ellipse::new()
                .width(26.0)
                .height(26.0)
                .fill(Color::transparent())
                .stroke(CYAN)
                .stroke_thickness(3.0)
                .canvas_left(hx - 13.0)
                .canvas_top(hy - 13.0),
        ));
        key += 1;
        let label_left = if hx > MAP_W - 240.0 { hx - 236.0 } else { hx + 16.0 };
        items.push(KeyedView::new(
            key,
            TextBlock::new()
                .text(place.name)
                .font_size(15.0)
                .font_weight(FontWeight::SEMI_BOLD)
                .foreground(CYAN)
                .canvas_left(label_left)
                .canvas_top(hy - 28.0)
                .width(220.0),
        ));

        let canvas = scaled(Canvas::new().width(MAP_W).height(MAP_H).keyed_children(items));

        let swatches: Vec<KeyedView> = [0.20_f64, 0.42, 0.62, 0.80, 0.95, 1.10]
            .iter()
            .enumerate()
            .map(|(index, value)| {
                KeyedView::new(index, Rectangle::new().width(30.0).height(7.0).fill(color3(intensity_color(*value))))
            })
            .collect();
        let legend = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                TextBlock::new().text("intensità relativa, bassa").font_size(9.5).foreground(MUTED),
                StackPanel::new().orientation(Orientation::Horizontal).keyed_children(swatches),
                TextBlock::new().text("alta").font_size(9.5).foreground(MUTED),
            ));

        let rc_dipole = cutoff_dipole(place.lat, place.lon);
        StackPanel::new().spacing(8.0).children((
            canvas,
            legend,
            TextBlock::new()
                .text(format!(
                    "Taglio geomagnetico dipolare + modulazione solare {} %. {}: latitudine geomagnetica {}°, Rc dipolare {} GV (stima reale ≈ {} GV).",
                    it_signed(modulation, 1),
                    place.name,
                    it(geomagnetic_lat(place.lat, place.lon), 1),
                    it(rc_dipole, 1),
                    it(place.rc, 1)
                ))
                .font_size(10.5)
                .foreground(MUTED)
                .text_wrapping(TextWrapping::Wrap),
        ))
    }

    fn credits(&self, context: &mut ViewContext<Self>) -> View {
        let entry = |key: usize,
                     logo: Option<&'static [u8]>,
                     name: &str,
                     role: &str,
                     url: &str,
                     link: &str|
         -> KeyedView {
            let logo_view = match logo {
                Some(bytes) => Border::new()
                    .background(Color::rgb(241, 245, 249))
                    .corner_radius(8.0)
                    .padding(Thickness::uniform(6.0))
                    .width(132.0)
                    .height(64.0)
                    .content(Image::new().source_data(EncodedImage::from_static(bytes)).stretch(Stretch::Uniform)),
                None => Border::new()
                    .background(Color::rgb(19, 30, 42))
                    .border_brush(Color::rgb(44, 60, 78))
                    .border_thickness(Thickness::uniform(1.0))
                    .corner_radius(8.0)
                    .width(132.0)
                    .height(64.0)
                    .content(
                        TextBlock::new()
                            .text(name.split(" · ").next().unwrap_or(name).to_string())
                            .font_size(18.0)
                            .font_weight(FontWeight::BOLD)
                            .foreground(SOFT)
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .vertical_alignment(VerticalAlignment::Center),
                    ),
            };
            KeyedView::new(
                key,
                Grid::new()
                    .columns([GridLength::Auto, GridLength::STAR])
                    .column_spacing(12.0)
                    .children((
                        Border::new().vertical_alignment(VerticalAlignment::Top).content(logo_view),
                        cell(
                            1,
                            StackPanel::new().spacing(2.0).children((
                                TextBlock::new()
                                    .text(name)
                                    .font_size(12.0)
                                    .font_weight(FontWeight::SEMI_BOLD)
                                    .foreground(TEXT)
                                    .text_wrapping(TextWrapping::Wrap),
                                TextBlock::new()
                                    .text(role)
                                    .font_size(10.5)
                                    .foreground(MUTED)
                                    .text_wrapping(TextWrapping::Wrap),
                                link_button(url, link),
                            )),
                        ),
                    )),
            )
        };

        let column = |entries: Vec<KeyedView>| StackPanel::new().spacing(14.0).keyed_children(entries);
        let grid = Grid::new()
            .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR])
            .column_spacing(20.0)
            .children((
                cell_top(
                    0,
                    column(vec![
                        entry(0, Some(LOGO_NMDB), "NMDB · Neutron Monitor Database", "La banca dati europea dei neutron monitor: i conteggi in tempo reale e gli archivi storici di Oulu e Roma.", "https://www.nmdb.eu", "www.nmdb.eu"),
                        entry(1, Some(LOGO_OULU), "Università di Oulu · Osservatorio Geofisico di Sodankylä", "Il neutron monitor di Oulu (Finlandia), riferimento mondiale per i raggi cosmici dal 1964.", "https://cosmicrays.oulu.fi", "cosmicrays.oulu.fi"),
                    ]),
                ),
                cell_top(
                    1,
                    column(vec![
                        entry(2, Some(LOGO_INAF), "Osservatorio SVIRCO · INAF-IAPS e Università Roma Tre", "Il neutron monitor di Roma, in funzione dal 1957.", "https://www.nmdb.eu/station/rome/", "SVIRCO su nmdb.eu"),
                        entry(3, None, "NOAA · Space Weather Prediction Center", "Indice Kp e previsione, raggi X e protoni GOES, vento solare e campo magnetico, F10.7, scale e bollettini del meteo spaziale.", "https://www.swpc.noaa.gov", "www.swpc.noaa.gov"),
                    ]),
                ),
                cell_top(
                    2,
                    column(vec![
                        entry(4, Some(LOGO_KYOTO), "World Data Center for Geomagnetism, Kyoto", "L'indice Dst della corrente ad anello, distribuito tramite NOAA SWPC.", "https://wdc.kugi.kyoto-u.ac.jp", "wdc.kugi.kyoto-u.ac.jp"),
                        entry(5, Some(LOGO_NE), "Natural Earth", "Le linee di costa della mappa mondiale, di pubblico dominio.", "https://www.naturalearthdata.com", "naturalearthdata.com"),
                    ]),
                ),
            ));

        let thanks = TextBlock::new()
            .text("Un grazie sincero a tutti gli enti, gli osservatori e le persone che misurano, conservano e condividono liberamente questi dati. Dietro ogni curva di questa dashboard ci sono decenni di lavoro: strumenti tenuti in funzione giorno e notte, calibrazioni, archivi curati con pazienza e servizi aperti a tutti. Senza la loro generosità ECHO Cosmic Ray non esisterebbe.")
            .font_size(12.0)
            .foreground(SOFT)
            .text_wrapping(TextWrapping::Wrap);

        let official: View = if self.show_ack {
            StackPanel::new().spacing(4.0).children((
                TextBlock::new()
                    .text("Riconoscimento richiesto da NMDB (testo originale):")
                    .font_size(10.0)
                    .foreground(MUTED),
                TextBlock::new()
                    .text(NMDB_ACK)
                    .font_size(10.5)
                    .foreground(SOFT)
                    .text_wrapping(TextWrapping::Wrap),
                TextBlock::new()
                    .text("Riferimenti del modello: UNSCEAR 2008 (Annex B), ICRU Report 84 per il profilo di dose; IGRF-14 (IAGA) per il polo geomagnetico. ECHO Cosmic Ray è un progetto personale e non è affiliato né approvato dagli enti citati: i loghi servono solo a riconoscere la provenienza dei dati.")
                    .font_size(10.0)
                    .foreground(MUTED)
                    .text_wrapping(TextWrapping::Wrap),
            ))
        } else {
            StackPanel::new().into()
        };

        let head = Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .children((
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children((
                        Rectangle::new().width(4.0).height(14.0).fill(GREEN).radius_x(2.0).radius_y(2.0),
                        TextBlock::new()
                            .text("RINGRAZIAMENTI · FONTI DEI DATI")
                            .font_size(11.0)
                            .font_weight(FontWeight::SEMI_BOLD)
                            .foreground(Color::rgb(148, 163, 184))
                            .vertical_alignment(VerticalAlignment::Center),
                    )),
                cell_v(
                    1,
                    Button::new()
                        .on_click(context.message(Message::ToggleAck))
                        .content(if self.show_ack { "Nascondi dettagli" } else { "Mostra dettagli" }),
                ),
            ));

        Border::new()
            .background(CARD_BG)
            .border_brush(CARD_BORDER)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(12.0)
            .padding(Thickness::uniform(16.0))
            .content(StackPanel::new().spacing(14.0).children((head, grid, thanks, official)))
    }

    fn footer(&self, context: &mut ViewContext<Self>) -> View {
        let mut errors: Vec<String> = self
            .data
            .errors
            .iter()
            .map(|(feed, e)| format!("{} ({e})", feed.label()))
            .collect();
        errors.sort();
        let warn = if errors.is_empty() {
            String::new()
        } else {
            format!("Fonti momentaneamente non raggiungibili, riprovo tra poco: {}", errors.join(" · "))
        };
        let dir = alerts::data_dir();

        let buttons = StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::OpenDataDir))
                    .content("Cartella dati")
                    .tooltip(format!("Storico orario CSV e impostazioni: {}", dir.display())),
                Button::new()
                    .on_click(context.message(Message::TestToast))
                    .content("Notifica di prova"),
            ));

        Grid::new()
            .columns([GridLength::STAR, GridLength::Auto])
            .column_spacing(16.0)
            .children((
                StackPanel::new().spacing(3.0).children((
                    TextBlock::new()
                        .text(format!(
                            "ECHO Cosmic Ray {} · storico orario in {} · dati NMDB e NOAA SWPC, aggiornati con cadenze diverse per non sovraccaricare le fonti.",
                            env!("CARGO_PKG_VERSION"),
                            History::path().display()
                        ))
                        .font_size(10.5)
                        .foreground(MUTED)
                        .text_wrapping(TextWrapping::Wrap),
                    TextBlock::new()
                        .text(warn)
                        .font_size(10.5)
                        .foreground(AMBER)
                        .text_wrapping(TextWrapping::Wrap),
                    TextBlock::new()
                        .text("Informazioni a scopo divulgativo: per valutazioni operative o di radioprotezione usare fonti e strumenti certificati. Copyright (c) 2026 PIGNOLO GIMMY.")
                        .font_size(10.0)
                        .foreground(SLATE)
                        .text_wrapping(TextWrapping::Wrap),
                )),
                cell_v(1, buttons),
            ))
    }
}

fn format_duration_h(hours: f64) -> String {
    if hours >= 48.0 * 365.0 {
        format!("{} anni", it(hours / 8760.0, 0))
    } else if hours >= 72.0 {
        format!("{:.0} giorni", hours / 24.0)
    } else {
        format!("{hours:.0} h")
    }
}

fn outlook_row(day: &str, a: &str, b: &str, c: &str, g: &str, color: Color) -> View {
    let t = |s: &str| -> View { TextBlock::new().text(s.to_string()).font_size(10.5).foreground(color).into() };
    Grid::new()
        .columns([GridLength::STAR, GridLength::STAR, GridLength::STAR, GridLength::STAR, GridLength::STAR])
        .children((
            cell(0, t(day)),
            cell(1, t(a)),
            cell(2, t(b)),
            cell(3, t(c)),
            cell(4, t(g)),
        ))
}

/// Link che apre il browser; se l'indirizzo non è valido resta come testo.
fn link_button(url: &str, label: &str) -> View {
    match HyperlinkButton::new().navigate_uri(url) {
        Ok(button) => button.content(label.to_string()).into(),
        Err(_) => TextBlock::new().text(label.to_string()).font_size(10.5).foreground(CYAN).into(),
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
pub(crate) fn scaled(view: impl Into<View>) -> View {
    Viewbox::new()
        .stretch(Stretch::Uniform)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Top)
        .slot(ViewboxSlot::Child, view.into())
}

pub(crate) fn help_panel(desc: &str) -> View {
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
                .foreground(SOFT)
                .text_wrapping(TextWrapping::Wrap),
        )
}

fn help_button(on_toggle: Callback<()>) -> View {
    Button::new()
        .on_click(on_toggle)
        .height(30.0)
        .content("?  Come si legge")
        .tooltip("Spiega il grafico: cosa sono gli assi, i colori e le sigle")
}

pub(crate) fn card(title: &str, accent: Color, desc: &str, body: View, open: bool, on_toggle: Callback<()>) -> View {
    card_with_action(title, accent, desc, body, open, on_toggle, None)
}

/// Scheda con un pulsante aggiuntivo accanto a "?".
/// Titolo di un grafico con il suo pulsantino: apre la spiegazione di quel
/// singolo grafico, non dell'intero pannello.
fn chart_title(text: &str, open: bool, on_toggle: Callback<()>) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .children((
            TextBlock::new()
                .text(text.to_string())
                .font_size(10.5)
                .foreground(MUTED),
            Button::new()
                .on_click(on_toggle)
                .height(22.0)
                .content(if open { "−" } else { "?" })
                .tooltip("Come si legge questo grafico"),
        ))
}

fn card_with_action(
    title: &str,
    accent: Color,
    desc: &str,
    body: View,
    open: bool,
    on_toggle: Callback<()>,
    action: Option<View>,
) -> View {
    let buttons: View = match action {
        Some(action) => StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((action, help_button(on_toggle)))
            .into(),
        None => help_button(on_toggle),
    };
    let head = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .children((
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Rectangle::new().width(4.0).height(14.0).fill(accent).radius_x(2.0).radius_y(2.0),
                    TextBlock::new()
                        .text(title)
                        .font_size(11.0)
                        .font_weight(FontWeight::SEMI_BOLD)
                        .foreground(Color::rgb(148, 163, 184))
                        .vertical_alignment(VerticalAlignment::Center),
                )),
            cell_v(1, buttons),
        ));

    let stack = StackPanel::new().spacing(10.0);
    let inner = if open {
        stack.children((head, help_panel(desc), body))
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

#[allow(clippy::too_many_arguments)]
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

    let sub = TextBlock::new()
        .text(sub)
        .font_size(10.5)
        .foreground(MUTED)
        .text_wrapping(TextWrapping::Wrap);
    let stack = StackPanel::new().spacing(4.0);
    let inner = if open {
        stack.children((head, help_panel(desc), value_row, sub))
    } else {
        stack.children((head, value_row, sub))
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

/// Risorse del pulsante blu pieno: il tema WinUI non espone il colore, si
/// sovrascrivono sfondo, testo, hover e stato premuto.
pub(crate) fn blue_overrides() -> ResourceOverrides {
    ResourceOverrides::new()
        .set("ButtonBackground", Color::rgb(37, 99, 235))
        .set("ButtonForeground", Color::rgb(255, 255, 255))
        .set("ButtonBorderBrush", Color::rgb(37, 99, 235))
        .set("ButtonBackgroundPointerOver", Color::rgb(29, 78, 216))
        .set("ButtonBorderBrushPointerOver", Color::rgb(29, 78, 216))
        .set("ButtonBackgroundPressed", Color::rgb(30, 64, 175))
        .set("ButtonBorderBrushPressed", Color::rgb(30, 64, 175))
}

pub(crate) fn legend_item(color: Color, label: &str) -> View {
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .children((
            Rectangle::new().width(14.0).height(3.0).fill(color).radius_x(1.5).radius_y(1.5),
            TextBlock::new().text(label.to_string()).font_size(10.0).foreground(MUTED),
        ))
}

/// Media mobile causale sull'asse x (finestra nelle stesse unità).
fn smooth_mean(points: &[(f64, f64)], window: f64) -> Vec<(f64, f64)> {
    let mut out = Vec::with_capacity(points.len());
    let mut start = 0usize;
    let mut sum = 0.0;
    for i in 0..points.len() {
        sum += points[i].1;
        while points[i].0 - points[start].0 > window {
            sum -= points[start].1;
            start += 1;
        }
        out.push((points[i].0, sum / (i - start + 1) as f64));
    }
    out
}

/// Cifre "tonde" tra `lo` e `hi`.
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
    (0..50)
        .map(|i| start + step * i as f64)
        .take_while(|v| *v <= hi + 1e-9)
        .map(|v| if v.abs() < step * 1e-6 { 0.0 } else { v })
        .collect()
}

fn axis_label(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else if v.abs() >= 10.0 {
        format!("{v:.0}")
    } else if v.abs() >= 1.0 {
        it(v, 1)
    } else {
        it(v, 2)
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

    let pts = downsample(points, 160);
    if pts.len() < 2 {
        return StackPanel::new().spacing(2.0).children((
            head,
            TextBlock::new().text("dati non disponibili").font_size(10.0).foreground(MUTED),
        ));
    }

    let t0 = pts[0].0;
    let mapped: Vec<(f64, f64)> = pts.iter().map(|(t, v)| ((*t - t0) as f64 / 3600.0, *v)).collect();
    let span = mapped.last().map(|p| p.0).unwrap_or(1.0).max(0.1);
    let gap = (span / mapped.len() as f64 * 3.0).max(0.05);
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
        let ticks = nice_ticks(lo, hi, 4).into_iter().map(|v| (v, axis_label(v))).collect();
        Axis::linear(lo, hi, ticks)
    };

    StackPanel::new().spacing(2.0).children((
        head,
        scaled(charts::plot(
            W,
            H,
            Axis::linear(0.0, span, xticks),
            y,
            vec![Series { points: mapped, color, thickness: 1.7, max_gap: gap }],
            vec![],
            zero_line,
        )),
    ))
}

fn main() {
    App::run_component::<Dashboard>(()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_mobile() {
        let pts = vec![(0.0, 1.0), (1.0, 3.0), (2.0, 5.0)];
        assert_eq!(smooth_mean(&pts, 1.0), vec![(0.0, 1.0), (1.0, 2.0), (2.0, 4.0)]);
    }

    #[test]
    fn numeri_italiani() {
        assert_eq!(it(1.26, 1), "1,3");
        assert_eq!(it_signed(-13.64, 1), "-13,6");
        assert_eq!(format_duration_h(370.0), "15 giorni");
    }
}
