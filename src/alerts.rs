//! Eventi di meteo spaziale (con isteresi), notifiche di Windows, impostazioni
//! persistenti e storico orario in CSV.

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

use crate::api;
use crate::model::*;
use crate::state::{max_between, mean_between, min_between, Data};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Condition {
    pub id: &'static str,
    #[allow(dead_code)] // letto nei test e utile per ordinare gli eventi
    pub level: u8,
    pub title: String,
    pub text: String,
    pub severity: Severity,
}

/// Regola: valore osservato, soglia di attivazione e di rientro (isteresi).
struct Rule {
    id: &'static str,
    value: Option<f64>,
    on: f64,
    off: f64,
    level: fn(f64) -> u8,
    describe: fn(f64, u8) -> (String, String, Severity),
}

fn rules(data: &Data, now: i64) -> Vec<Rule> {
    vec![
        Rule {
            id: "G",
            value: data.kp_now(now),
            on: 5.0,
            off: 4.33,
            level: |v| g_scale(v).max(1),
            describe: |v, l| {
                (
                    format!("Tempesta geomagnetica G{l}"),
                    format!("Kp stimato {v:.2}: {}. Possibili aurore a latitudini più basse del solito.", kp_storm(v)),
                    if l >= 3 { Severity::Error } else { Severity::Warning },
                )
            },
        },
        Rule {
            id: "S",
            value: data.protons_now(now),
            on: 10.0,
            off: 8.0,
            level: |v| s_scale(v).max(1),
            describe: |v, l| {
                (
                    format!("Tempesta di radiazione S{l}"),
                    format!("Protoni ≥10 MeV a {v:.0} pfu (GOES). Dose aumentata sulle rotte polari e in orbita."),
                    if l >= 2 { Severity::Error } else { Severity::Warning },
                )
            },
        },
        Rule {
            id: "R",
            value: data.xray_now(now),
            on: 1e-5,
            off: 7e-6,
            level: |v| r_scale(v).max(1),
            describe: |v, l| {
                (
                    format!("Brillamento solare classe {} (R{l})", flare_class(v)),
                    "Flusso X-ray GOES elevato: possibili blackout radio HF sul lato illuminato della Terra.".into(),
                    if l >= 3 { Severity::Error } else { Severity::Warning },
                )
            },
        },
        Rule {
            id: "GLE",
            value: data.gle_pct(now),
            on: 4.0,
            off: 2.0,
            level: |_| 1,
            describe: |v, _| {
                (
                    "Possibile GLE · aumento al suolo".into(),
                    format!("Oulu +{v:.1}% negli ultimi 10 minuti: particelle solari abbastanza energetiche da arrivare al suolo (evento raro). Da confermare con altre stazioni."),
                    Severity::Error,
                )
            },
        },
        Rule {
            id: "FD",
            value: data.forbush_pct(now).map(|v| -v),
            on: 3.0,
            off: 2.0,
            level: |v| if v >= 6.0 { 2 } else { 1 },
            describe: |v, _| {
                (
                    "Calo di Forbush in corso".into(),
                    format!("Oulu −{v:.1}% rispetto alla media dei 30 giorni (ultime 3 ore): una nube di plasma solare sta schermando i raggi cosmici."),
                    Severity::Warning,
                )
            },
        },
        Rule {
            id: "BZ",
            value: data.bz_mean30(now).map(|v| -v),
            on: 10.0,
            off: 6.0,
            level: |v| if v >= 20.0 { 2 } else { 1 },
            describe: |v, _| {
                (
                    "Bz fortemente verso sud".into(),
                    format!("Media 30 min −{v:.1} nT: il vento solare si accoppia alla magnetosfera, tempesta probabile."),
                    Severity::Warning,
                )
            },
        },
        Rule {
            id: "VS",
            value: data.wind_mean30(now),
            on: 700.0,
            off: 620.0,
            level: |v| if v >= 900.0 { 2 } else { 1 },
            describe: |v, _| {
                (
                    "Vento solare molto veloce".into(),
                    format!("Media 30 min {v:.0} km/s: flusso da buco coronale o passaggio di una CME."),
                    Severity::Info,
                )
            },
        },
    ]
}

/// Stato degli eventi tra un aggiornamento e l'altro.
#[derive(Default)]
pub struct Watch {
    levels: HashMap<&'static str, u8>,
    primed: bool,
}

impl Watch {
    /// Ricalcola gli eventi attivi. Ritorna (attivi, da notificare): si notifica solo
    /// un evento nuovo o in peggioramento, e non quelli già in corso all'avvio.
    pub fn update(&mut self, data: &Data, now: i64) -> (Vec<Condition>, Vec<Condition>) {
        let mut active = Vec::new();
        let mut notify = Vec::new();
        for rule in rules(data, now) {
            let prev = self.levels.get(rule.id).copied().unwrap_or(0);
            let level = match rule.value {
                Some(v) if v >= rule.on || (prev > 0 && v >= rule.off) => {
                    (rule.level)(v).max(1).max(if v < rule.on { prev } else { 0 })
                }
                Some(_) => 0,
                // dato non disponibile: si mantiene lo stato
                None => prev,
            };
            if level > 0 {
                if let Some(v) = rule.value {
                    let (title, text, severity) = (rule.describe)(v, level);
                    let c = Condition { id: rule.id, level, title, text, severity };
                    if self.primed && level > prev {
                        notify.push(c.clone());
                    }
                    active.push(c);
                }
            }
            self.levels.insert(rule.id, level);
        }
        if data.loaded_once() {
            self.primed = true;
        }
        (active, notify)
    }
}

// ---------------------------------------------------------------------------
// Notifiche di Windows (toast) tramite PowerShell, senza dipendenze aggiuntive.
// ---------------------------------------------------------------------------

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Mostra una notifica di Windows. Titolo e testo passano per variabili d'ambiente,
/// così non possono alterare lo script.
pub fn toast(title: &str, text: &str) {
    use std::os::windows::process::CommandExt;
    const SCRIPT: &str = r#"
[void][Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime]
[void][Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]
$e = [Security.SecurityElement]
$x = New-Object Windows.Data.Xml.Dom.XmlDocument
$x.LoadXml("<toast><visual><binding template='ToastGeneric'><text>" + $e::Escape($env:ECHO_TITLE) + "</text><text>" + $e::Escape($env:ECHO_TEXT) + "</text><text placement='attribution'>ECHO Cosmic Ray</text></binding></visual></toast>")
$id = '{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe'
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier($id).Show([Windows.UI.Notifications.ToastNotification]::new($x))
"#;
    let utf16: Vec<u8> = SCRIPT.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let _ = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand"])
        .arg(base64(&utf16))
        .env("ECHO_TITLE", title)
        .env("ECHO_TEXT", text)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn();
}

// ---------------------------------------------------------------------------
// Impostazioni e cartella dati: %APPDATA%\ECHO Cosmic Ray
// ---------------------------------------------------------------------------

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("ECHO Cosmic Ray")
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub place: usize,
    pub altitude: usize,
    pub toasts: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { place: 0, altitude: 1, toasts: true }
    }
}

impl Settings {
    fn path() -> PathBuf {
        data_dir().join("impostazioni.txt")
    }

    pub fn load() -> Self {
        let mut s = Self::default();
        let Ok(text) = std::fs::read_to_string(Self::path()) else { return s };
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match (k.trim(), v.trim()) {
                ("localita", v) => s.place = v.parse().unwrap_or(0).min(PLACES.len() - 1),
                ("quota", v) => s.altitude = v.parse().unwrap_or(0).min(ALTITUDE_CHOICES.len() - 1),
                ("notifiche", v) => s.toasts = v != "0",
                _ => {}
            }
        }
        s
    }

    pub fn save(&self) {
        let _ = std::fs::create_dir_all(data_dir());
        let _ = std::fs::write(
            Self::path(),
            format!(
                "# ECHO Cosmic Ray · impostazioni\nlocalita={}\nquota={}\nnotifiche={}\n",
                self.place,
                self.altitude,
                u8::from(self.toasts)
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Storico orario (CSV con separatore ';' e virgola decimale, apribile in Excel).
// ---------------------------------------------------------------------------

pub struct History {
    last_hour: Option<i64>,
}

const HEADER: &str = "ora_utc;oulu_conteggi_s;oulu_vs_minimo_pct;roma_conteggi_s;roma_vs_minimo_pct;kp_max;xray_max_W_m2;classe_xray;protoni_max_pfu;vento_km_s;bz_min_nT;dst_nT";

impl History {
    pub fn path() -> PathBuf {
        data_dir().join("storico_orario.csv")
    }

    pub fn open() -> Self {
        let last_hour = std::fs::read_to_string(Self::path()).ok().and_then(|text| {
            text.lines()
                .rev()
                .find_map(|l| api::parse_epoch(l.split(';').next()?))
        });
        Self { last_hour }
    }

    /// Scrive le ore complete non ancora salvate (al massimo le ultime 24).
    pub fn record(&mut self, data: &Data, now: i64) {
        let Some(&first) = data.oulu.first().map(|p| &p.0) else { return };
        let current = now.div_euclid(3_600) * 3_600;
        let mut hour = self
            .last_hour
            .map(|h| h + 3_600)
            .unwrap_or(current - 3_600)
            .max(current - 24 * 3_600)
            .max(first.div_euclid(3_600) * 3_600 + 3_600);
        let mut lines = Vec::new();
        while hour < current {
            let end = hour + 3_600;
            let oulu = mean_between(&data.oulu, hour, end);
            if oulu.is_some() {
                let rome = mean_between(&data.rome, hour, end);
                let xmax = max_between(&data.xray, hour, end);
                let f = |v: Option<f64>, d: usize| {
                    v.map(|v| format!("{v:.d$}").replace('.', ",")).unwrap_or_default()
                };
                let e = |v: Option<f64>| {
                    v.map(|v| format!("{v:.2e}").replace('.', ",")).unwrap_or_default()
                };
                lines.push(format!(
                    "{};{};{};{};{};{};{};{};{};{};{};{}",
                    api::iso_minute(hour),
                    f(oulu, 3),
                    f(oulu.map(|c| (c / OULU_SOLAR_MIN - 1.0) * 100.0), 2),
                    f(rome, 3),
                    f(rome.map(|c| (c / ROME_SOLAR_MIN - 1.0) * 100.0), 2),
                    f(max_between(&data.kp, hour, end), 2),
                    e(xmax),
                    xmax.map(flare_class).unwrap_or_default(),
                    f(max_between(&data.protons, hour, end), 2),
                    f(mean_between(&data.wind, hour, end), 0),
                    f(min_between(&data.bz, hour, end), 1),
                    f(mean_between(&data.dst, hour, end + 1), 0),
                ));
                self.last_hour = Some(hour);
            }
            hour = end;
        }
        if lines.is_empty() {
            return;
        }
        let _ = std::fs::create_dir_all(data_dir());
        let path = Self::path();
        let new_file = !path.exists();
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            if new_file {
                let _ = writeln!(file, "{HEADER}");
            }
            for line in lines {
                let _ = writeln!(file, "{line}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_standard() {
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn isteresi_e_niente_notifica_all_avvio() {
        let mut d = Data::default();
        let now = 1_000_000;
        d.kp = vec![(now - 60, 5.33)];
        let mut w = Watch::default();
        let (active, notify) = w.update(&d, now);
        assert_eq!(active.len(), 1);
        assert!(notify.is_empty(), "nessun toast per eventi già in corso all'avvio");
        w.primed = true;
        d.kp = vec![(now - 60, 4.67)]; // sotto 5 ma sopra la soglia di rientro
        let (active, _) = w.update(&d, now);
        assert_eq!(active.len(), 1);
        d.kp = vec![(now - 60, 3.0)];
        let (active, _) = w.update(&d, now);
        assert!(active.is_empty());
        d.kp = vec![(now - 60, 6.0)];
        let (_, notify) = w.update(&d, now);
        assert_eq!(notify.len(), 1);
        assert_eq!(notify[0].level, 2);
    }
}
