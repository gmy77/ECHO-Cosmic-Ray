//! Acquisizione e parsing dei dati live.
//!
//! Ogni sorgente è un `Feed` con la propria cadenza: solo quelli scaduti vengono
//! richiesti, in parallelo, e i neutron monitor in modo incrementale (ultime 3 ore).
//!
//! Sorgenti:
//!  * NMDB (neutron monitor Oulu e Roma) -> conteggi corretti per efficienza
//!  * NOAA SWPC -> Kp, previsione Kp, X-ray e protoni GOES, vento solare e IMF,
//!    Dst (WDC Kyoto), F10.7, scale NOAA e bollettini di allerta

use std::sync::OnceLock;
use std::time::Duration;

const SWPC: &str = "https://services.swpc.noaa.gov";
const NMDB: &str = "https://www.nmdb.eu/nest/draw_graph.php?formchk=1&output=ascii&tabchoice=revori&dtype=corr_for_efficiency&yunits=0&date_choice=last";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Feed {
    Oulu,
    Rome,
    Month,
    Kp,
    KpForecast,
    Xray,
    Protons,
    Dst,
    F107,
    Wind,
    Mag,
    WindNow,
    MagNow,
    Scales,
    Alerts,
}

impl Feed {
    pub const ALL: [Feed; 15] = [
        Feed::Oulu,
        Feed::Rome,
        Feed::Month,
        Feed::Kp,
        Feed::KpForecast,
        Feed::Xray,
        Feed::Protons,
        Feed::Dst,
        Feed::F107,
        Feed::Wind,
        Feed::Mag,
        Feed::WindNow,
        Feed::MagNow,
        Feed::Scales,
        Feed::Alerts,
    ];

    /// Cadenza di aggiornamento (secondi), scelta sul ritmo reale del dato.
    pub fn every_s(self) -> i64 {
        match self {
            Feed::Oulu | Feed::Rome => 120,
            Feed::Kp | Feed::WindNow | Feed::MagNow => 120,
            Feed::Xray => 180,
            Feed::Protons => 300,
            Feed::Scales | Feed::Alerts => 600,
            Feed::Wind | Feed::Mag => 900,
            Feed::Dst | Feed::KpForecast => 1800,
            Feed::Month => 3600,
            Feed::F107 => 6 * 3600,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Feed::Oulu => "NMDB Oulu",
            Feed::Rome => "NMDB Roma",
            Feed::Month => "NMDB 30 giorni",
            Feed::Kp => "Kp",
            Feed::KpForecast => "previsione Kp",
            Feed::Xray => "X-ray GOES",
            Feed::Protons => "protoni GOES",
            Feed::Dst => "Dst",
            Feed::F107 => "F10.7",
            Feed::Wind => "vento solare",
            Feed::Mag => "campo IMF",
            Feed::WindNow => "vento solare (ultimo)",
            Feed::MagNow => "IMF (ultimo)",
            Feed::Scales => "scale NOAA",
            Feed::Alerts => "allerte NOAA",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Request {
    pub feed: Feed,
    /// Per i neutron monitor: `true` scarica 24 h, `false` solo le ultime 3 ore.
    pub full: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KpKind {
    Observed,
    Estimated,
    Predicted,
}

#[derive(Clone, Debug)]
pub struct KpBlock {
    pub t: i64,
    pub kp: f64,
    pub kind: KpKind,
}

#[derive(Clone, Debug, Default)]
pub struct DayOutlook {
    pub date: String,
    pub r_minor: Option<u8>,
    pub r_major: Option<u8>,
    pub s_prob: Option<u8>,
    pub g_scale: Option<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Scales {
    pub r: u8,
    pub s: u8,
    pub g: u8,
    pub days: Vec<DayOutlook>,
}

#[derive(Clone, Debug)]
pub struct SpaceAlert {
    pub t: i64,
    pub kind: String,
    pub headline: String,
}

#[derive(Clone, Debug)]
pub enum Payload {
    Nmdb(Vec<(i64, f64)>),
    Month { oulu: Vec<(i64, f64)>, rome: Vec<(i64, f64)> },
    Series(Vec<(i64, f64)>),
    Pair(Vec<(i64, f64)>, Vec<(i64, f64)>),
    KpForecast(Vec<KpBlock>),
    Scales(Scales),
    Alerts(Vec<SpaceAlert>),
}

pub type FetchResult = (Feed, Result<Payload, String>);

fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent(concat!("ECHO-Cosmic-Ray/", env!("CARGO_PKG_VERSION"), " (desktop dashboard)"))
            .build()
    })
}

fn http_get(url: &str) -> Result<String, String> {
    let response = agent().get(url).call().map_err(short_error)?;
    response.into_string().map_err(|e| e.to_string())
}

fn http_json(url: &str) -> Result<serde_json::Value, String> {
    let body = http_get(url)?;
    serde_json::from_str(&body).map_err(|e| format!("JSON non valido ({e})"))
}

/// Proxy Cloudflare per NMDB: l'host diretto non è raggiungibile da tutte le reti,
/// e il reader pubblico è lento. Il Worker restituisce la sola tabella ASCII.
const NMDB_PROXY: &str = "https://nmdb-proxy.gimmy077.workers.dev";

/// Riscrive un URL NMDB sul proxy, se l'host è quello atteso.
fn proxied_url(url: &str) -> Option<String> {
    url.strip_prefix("https://www.nmdb.eu")
        .map(|rest| format!("{NMDB_PROXY}{rest}"))
}

/// Scarica un URL NMDB: prima il proxy, poi la connessione diretta.
fn nmdb_get(url: &str) -> Result<String, String> {
    let mut errors: Vec<String> = Vec::new();
    if let Some(proxied) = proxied_url(url) {
        match http_get(&proxied) {
            Ok(body) if !body.trim().is_empty() => return Ok(body),
            Ok(_) => errors.push("proxy vuoto".to_string()),
            Err(error) => errors.push(format!("proxy: {error}")),
        }
    }
    match http_get(url) {
        Ok(body) if !body.trim().is_empty() => Ok(body),
        Ok(_) => Err("risposta vuota".to_string()),
        Err(error) => {
            errors.push(format!("diretto: {error}"));
            Err(errors.join(" · "))
        }
    }
}

/// Messaggio d'errore compatto, senza l'URL completo.
fn short_error(e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, _) => format!("HTTP {code}"),
        ureq::Error::Transport(t) => {
            let text = t.to_string();
            match text.split_once(": ") {
                Some((_, rest)) if !rest.is_empty() => rest.to_string(),
                _ => text,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tempo
// ---------------------------------------------------------------------------

/// Giorni dall'epoca Unix per una data del calendario civile.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Data civile (anno, mese, giorno) da giorni dall'epoca.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Converte `YYYY-MM-DD[ T]HH:MM[:SS]` (UTC) in secondi dall'epoca.
pub fn parse_epoch(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.len() < 16 {
        return None;
    }
    let y: i64 = text.get(0..4)?.parse().ok()?;
    let mo: i64 = text.get(5..7)?.parse().ok()?;
    let d: i64 = text.get(8..10)?.parse().ok()?;
    let h: i64 = text.get(11..13)?.parse().ok()?;
    let mi: i64 = text.get(14..16)?.parse().ok()?;
    let s: i64 = text.get(17..19).and_then(|v| v.parse().ok()).unwrap_or(0);
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3_600 + mi * 60 + s)
}

/// Formatta un istante epoch come `HH:MM`.
pub fn hhmm(epoch: i64) -> String {
    let secs = epoch.rem_euclid(86_400);
    format!("{:02}:{:02}", secs / 3_600, (secs % 3_600) / 60)
}

/// Formatta un istante epoch come `GG/MM`.
pub fn ddmm(epoch: i64) -> String {
    let (_, m, d) = civil_from_days(epoch.div_euclid(86_400));
    format!("{d:02}/{m:02}")
}

/// Formatta un istante epoch come `AAAA-MM-GG HH:MM` (per il CSV).
pub fn iso_minute(epoch: i64) -> String {
    let (y, m, d) = civil_from_days(epoch.div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02} {}", hhmm(epoch))
}

/// Scarto dell'ora italiana (CET/CEST, regola UE) rispetto a UTC, in secondi.
pub fn italy_offset(epoch: i64) -> i64 {
    let (y, _, _) = civil_from_days(epoch.div_euclid(86_400));
    // ultima domenica di marzo e di ottobre, alle 01:00 UTC
    let last_sunday = |month: i64| {
        let last_day = days_from_civil(y, month, 31);
        let weekday = (last_day + 4).rem_euclid(7); // 0 = domenica
        (last_day - weekday) * 86_400 + 3_600
    };
    if epoch >= last_sunday(3) && epoch < last_sunday(10) {
        7_200
    } else {
        3_600
    }
}

pub fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

fn parse_nmdb(body: &str) -> Vec<(i64, f64)> {
    let start = body.find("<code>").map(|i| i + 6).unwrap_or(0);
    let end = body[start..]
        .find("</code>")
        .map(|i| start + i)
        .unwrap_or(body.len());
    let mut out = Vec::new();
    for line in body[start..end].lines() {
        let line = line.trim();
        if line.starts_with('#') || !line.contains(';') {
            continue;
        }
        let mut parts = line.splitn(2, ';');
        let (Some(stamp), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some(epoch) = parse_epoch(stamp) else {
            continue;
        };
        if let Ok(value) = value.trim().replace(',', ".").parse::<f64>() {
            // conteggi assoluti: valori nulli o negativi sono buchi del monitor
            if value.is_finite() && value > 1.0 {
                out.push((epoch, value));
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out.dedup_by_key(|p| p.0);
    drop_outliers(out)
}

/// Scarta i valori assurdi (es. un'ora di Roma a 14 109 conteggi/s il 28/10/2026).
/// La soglia è larga rispetto alla mediana: un GLE fino a +100 % resta visibile.
fn drop_outliers(mut series: Vec<(i64, f64)>) -> Vec<(i64, f64)> {
    if series.len() < 5 {
        return series;
    }
    let mut values: Vec<f64> = series.iter().map(|p| p.1).collect();
    values.sort_by(|a, b| a.total_cmp(b));
    let median = values[values.len() / 2];
    series.retain(|p| p.1 > median * 0.6 && p.1 < median * 2.0);
    series
}

fn rows(value: &serde_json::Value) -> &[serde_json::Value] {
    value.as_array().map(|v| v.as_slice()).unwrap_or(&[])
}

fn stamp(row: &serde_json::Value) -> Option<i64> {
    row.get("time_tag").and_then(|v| v.as_str()).and_then(parse_epoch)
}

fn num(row: &serde_json::Value, field: &str) -> Option<f64> {
    let v = row.get(field)?;
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
        .filter(|v| v.is_finite())
}

/// Serie da un array JSON con `time_tag` e un campo numerico; `filter` scarta righe.
fn series_where(
    value: &serde_json::Value,
    field: &str,
    filter: impl Fn(&serde_json::Value) -> bool,
) -> Vec<(i64, f64)> {
    let mut out: Vec<(i64, f64)> = rows(value)
        .iter()
        .filter(|row| filter(row))
        .filter_map(|row| Some((stamp(row)?, num(row, field)?)))
        .collect();
    out.sort_by_key(|p| p.0);
    out.dedup_by_key(|p| p.0);
    out
}

fn energy_is(row: &serde_json::Value, energy: &str) -> bool {
    row.get("energy").and_then(|v| v.as_str()) == Some(energy)
}

/// I feed RTSW mescolano più sonde (SOLAR-1, ACE, IMAP...): conta solo quella `active`.
fn rtsw_active(row: &serde_json::Value) -> bool {
    row.get("active").and_then(|v| v.as_bool()) != Some(false)
}

fn parse_kp_forecast(value: &serde_json::Value) -> Vec<KpBlock> {
    let mut out: Vec<KpBlock> = rows(value)
        .iter()
        .filter_map(|row| {
            let kind = match row.get("observed").and_then(|v| v.as_str())? {
                "observed" => KpKind::Observed,
                "estimated" => KpKind::Estimated,
                _ => KpKind::Predicted,
            };
            Some(KpBlock { t: stamp(row)?, kp: num(row, "kp")?, kind })
        })
        .collect();
    out.sort_by_key(|b| b.t);
    out
}

fn small(value: Option<&serde_json::Value>) -> Option<u8> {
    let v = value?;
    v.as_u64()
        .map(|n| n.min(100) as u8)
        .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn parse_scales(value: &serde_json::Value) -> Scales {
    let now = value.get("0");
    let level = |key: &str| {
        small(now.and_then(|n| n.get(key)).and_then(|s| s.get("Scale"))).unwrap_or(0)
    };
    let mut scales = Scales { r: level("R"), s: level("S"), g: level("G"), days: Vec::new() };
    for key in ["1", "2", "3"] {
        let Some(day) = value.get(key) else { continue };
        let r = day.get("R");
        scales.days.push(DayOutlook {
            date: day
                .get("DateStamp")
                .and_then(|v| v.as_str())
                .and_then(|s| parse_epoch(&format!("{s} 00:00")))
                .map(ddmm)
                .unwrap_or_default(),
            r_minor: small(r.and_then(|r| r.get("MinorProb"))),
            r_major: small(r.and_then(|r| r.get("MajorProb"))),
            s_prob: small(day.get("S").and_then(|s| s.get("Prob"))),
            g_scale: small(day.get("G").and_then(|g| g.get("Scale"))),
        });
    }
    scales
}

/// Riga principale di un bollettino SWPC ("ALERT: ...", "WARNING: ...").
fn parse_alerts(value: &serde_json::Value) -> Vec<SpaceAlert> {
    // L'ordine conta: "CANCEL WARNING" va riconosciuto prima di "WARNING".
    const KINDS: &[(&str, &str)] = &[
        ("CANCEL", "Annullato"),
        ("EXTENDED", "Avviso esteso"),
        ("CONTINUED", "Allerta in corso"),
        ("ALERT", "Allerta"),
        ("WARNING", "Avviso"),
        ("WATCH", "Sorveglianza"),
        ("SUMMARY", "Riepilogo"),
    ];
    let mut out = Vec::new();
    for row in rows(value) {
        let Some(t) = row
            .get("issue_datetime")
            .and_then(|v| v.as_str())
            .and_then(parse_epoch)
        else {
            continue;
        };
        let message = row.get("message").and_then(|v| v.as_str()).unwrap_or("");
        let mut found = None;
        for line in message.lines().map(str::trim) {
            let Some((prefix, text)) = line.split_once(':') else { continue };
            let prefix = prefix.to_ascii_uppercase();
            let is_head = ["ALERT", "WARNING", "WATCH", "SUMMARY"]
                .iter()
                .any(|k| prefix.ends_with(k));
            if !is_head || prefix.len() > 24 {
                continue;
            }
            if let Some((_, it)) = KINDS.iter().find(|(k, _)| prefix.contains(k)) {
                found = Some((it.to_string(), text.trim().to_string()));
                break;
            }
        }
        if let Some((kind, headline)) = found {
            if !headline.is_empty() {
                out.push(SpaceAlert { t, kind, headline });
            }
        }
    }
    out.sort_by_key(|a| -a.t);
    out.truncate(12);
    out
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

fn nmdb_url(station: &str, window: &str) -> String {
    format!("{NMDB}&stations[]={station}&{window}")
}

fn fetch_one(req: Request) -> Result<Payload, String> {
    let nmdb_window = if req.full {
        "last_days=1&last_label=days_label"
    } else {
        "last_hours=3&last_label=hours_label"
    };
    let json = |path: &str| http_json(&format!("{SWPC}{path}"));
    let nonempty = |s: Vec<(i64, f64)>| {
        if s.is_empty() {
            Err("risposta senza dati".to_string())
        } else {
            Ok(s)
        }
    };
    match req.feed {
        Feed::Oulu => Ok(Payload::Nmdb(nonempty(parse_nmdb(&nmdb_get(&nmdb_url("OULU", nmdb_window))?))?)),
        Feed::Rome => Ok(Payload::Nmdb(nonempty(parse_nmdb(&nmdb_get(&nmdb_url("ROME", nmdb_window))?))?)),
        Feed::Month => {
            let window = "last_days=30&last_label=days_label&tresolution=60";
            let oulu = parse_nmdb(&nmdb_get(&nmdb_url("OULU", window))?);
            let rome = parse_nmdb(&nmdb_get(&nmdb_url("ROME", window)).unwrap_or_default());
            Ok(Payload::Month { oulu: nonempty(oulu)?, rome })
        }
        Feed::Kp => {
            let v = json("/json/planetary_k_index_1m.json")?;
            let mut s = series_where(&v, "estimated_kp", |_| true);
            if s.is_empty() {
                s = series_where(&v, "kp_index", |_| true);
            }
            Ok(Payload::Series(nonempty(s)?))
        }
        Feed::KpForecast => Ok(Payload::KpForecast(parse_kp_forecast(&json(
            "/products/noaa-planetary-k-index-forecast.json",
        )?))),
        Feed::Xray => {
            let v = json("/json/goes/primary/xrays-6-hour.json")?;
            let s = series_where(&v, "flux", |r| energy_is(r, "0.1-0.8nm"));
            Ok(Payload::Series(nonempty(s.into_iter().filter(|p| p.1 > 0.0).collect())?))
        }
        Feed::Protons => {
            let v = json("/json/goes/primary/integral-protons-6-hour.json")?;
            let s = series_where(&v, "flux", |r| energy_is(r, ">=10 MeV"));
            Ok(Payload::Series(nonempty(s.into_iter().filter(|p| p.1 > 0.0).collect())?))
        }
        Feed::Dst => Ok(Payload::Series(nonempty(series_where(
            &json("/products/kyoto-dst.json")?,
            "dst",
            |_| true,
        ))?)),
        Feed::F107 => Ok(Payload::Series(nonempty(series_where(
            &json("/products/10cm-flux-30-day.json")?,
            "flux",
            |_| true,
        ))?)),
        Feed::Wind => {
            let v = json("/json/rtsw/rtsw_wind_1m.json")?;
            Ok(Payload::Pair(
                nonempty(series_where(&v, "proton_speed", rtsw_active))?,
                series_where(&v, "proton_density", rtsw_active),
            ))
        }
        Feed::Mag => {
            let v = json("/json/rtsw/rtsw_mag_1m.json")?;
            Ok(Payload::Pair(
                nonempty(series_where(&v, "bz_gsm", rtsw_active))?,
                series_where(&v, "bt", rtsw_active),
            ))
        }
        Feed::WindNow => Ok(Payload::Series(nonempty(series_where(
            &json("/products/summary/solar-wind-speed.json")?,
            "proton_speed",
            |_| true,
        ))?)),
        Feed::MagNow => {
            let v = json("/products/summary/solar-wind-mag-field.json")?;
            Ok(Payload::Pair(
                nonempty(series_where(&v, "bz_gsm", |_| true))?,
                series_where(&v, "bt", |_| true),
            ))
        }
        Feed::Scales => Ok(Payload::Scales(parse_scales(&json("/products/noaa-scales.json")?))),
        Feed::Alerts => Ok(Payload::Alerts(parse_alerts(&json("/products/alerts.json")?))),
    }
}

/// Scarica in parallelo le sorgenti richieste.
pub fn fetch(requests: &[Request]) -> Vec<FetchResult> {    std::thread::scope(|scope| {
        let handles: Vec<_> = requests
            .iter()
            .map(|req| {
                let req = *req;
                (
                    req.feed,
                    scope.spawn(move || {
                        std::panic::catch_unwind(|| fetch_one(req))
                            .unwrap_or_else(|_| Err("errore interno".to_string()))
                    }),
                )
            })
            .collect();
        handles
            .into_iter()
            .map(|(feed, handle)| {
                (feed, handle.join().unwrap_or_else(|_| Err("errore interno".to_string())))
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------
// Atlante: rete completa NMDB (ultimi 7 giorni, passo orario)
// ---------------------------------------------------------------------------

/// URL NMDB per un blocco di stazioni: un'ora di risoluzione, 7 giorni,
/// conteggi corretti per efficienza, tabella ASCII.
fn atlas_url(codes: &[&str]) -> String {
    let mut url = String::from("https://www.nmdb.eu/nest/draw_graph.php?formchk=1&output=ascii");
    for code in codes {
        url.push_str("&stations[]=");
        url.push_str(code);
    }
    url.push_str("&tabchoice=ori&dtype=corr_for_efficiency&tresolution=60&yunits=0&wget=1");
    url.push_str("&date_choice=last&last_days=7&last_label=days_label");
    url
}

/// Un codice stazione NMDB: quattro caratteri maiuscoli o cifre.
fn is_station_code(token: &str) -> bool {
    token.len() == 4
        && token
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Intestazione delle colonne: riga di soli codici; in mancanza, la riga
/// `# STATION: A, B, ...`. Il reader appiattisce le righe con ` # `.
fn atlas_columns(text: &str) -> Vec<String> {
    let flat = text.replace(" # ", "\n");
    let cut = flat
        .find(|c: char| c.is_ascii_digit())
        .unwrap_or(flat.len());
    let prefix = &flat[..cut];
    let mut head: Vec<String> = Vec::new();
    for line in prefix.lines() {
        let tokens: Vec<&str> = line
            .trim()
            .trim_start_matches('#')
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|t| !t.is_empty())
            .collect();
        if tokens.len() >= 2 && tokens.iter().all(|t| is_station_code(t)) {
            head = tokens.iter().map(|t| t.to_string()).collect();
        }
        if let Some(pos) = line.find("STATION:") {
            let rest = &line[pos + 8..];
            let rest = rest.split('#').next().unwrap_or(rest);
            let codes: Vec<String> = rest
                .split(',')
                .map(|c| c.trim().to_string())
                .filter(|c| is_station_code(c))
                .collect();
            if head.is_empty() && !codes.is_empty() {
                head = codes;
            }
        }
    }
    head
}

/// Posizioni di tutti i timestamp `AAAA-MM-GG HH:MM:SS` nel testo.
fn atlas_stamps(text: &str) -> Vec<(usize, i64)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    if bytes.len() < 19 {
        return out;
    }
    let digit = |b: u8| b.is_ascii_digit();
    let mut i = 0;
    while i + 19 <= bytes.len() {
        let matched = digit(bytes[i])
            && digit(bytes[i + 1])
            && digit(bytes[i + 2])
            && digit(bytes[i + 3])
            && bytes[i + 4] == b'-'
            && digit(bytes[i + 5])
            && digit(bytes[i + 6])
            && bytes[i + 7] == b'-'
            && digit(bytes[i + 8])
            && digit(bytes[i + 9])
            && (bytes[i + 10] == b' ' || bytes[i + 10] == b'T')
            && digit(bytes[i + 11])
            && digit(bytes[i + 12])
            && bytes[i + 13] == b':'
            && digit(bytes[i + 14])
            && digit(bytes[i + 15])
            && bytes[i + 16] == b':'
            && digit(bytes[i + 17])
            && digit(bytes[i + 18]);
        if matched {
            if let Some(epoch) = parse_epoch(&text[i..i + 19]) {
                out.push((i, epoch));
                i += 19;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Righe `timestamp -> valori` di una risposta multilinea o appiattita.
fn parse_atlas_rows(text: &str, columns: usize) -> Vec<(i64, Vec<Option<f64>>)> {
    let stamps = atlas_stamps(text);
    let mut out = Vec::with_capacity(stamps.len());
    for (index, (offset, epoch)) in stamps.iter().enumerate() {
        let end = stamps
            .get(index + 1)
            .map(|(next, _)| *next)
            .unwrap_or(text.len());
        let row = &text[offset + 19..end];
        let values: Vec<Option<f64>> = row
            .split(';')
            .skip(1)
            .take(columns)
            .map(|token| {
                let token = token.split('#').next().unwrap_or("").trim();
                if token.is_empty() || token.eq_ignore_ascii_case("null") {
                    None
                } else {
                    token
                        .replace(',', ".")
                        .parse::<f64>()
                        .ok()
                        .filter(|v| v.is_finite() && *v > 0.0)
                }
            })
            .collect();
        if values.len() == columns {
            out.push((*epoch, values));
        }
    }
    out
}

fn atlas_text(url: &str, proxy: bool) -> Result<String, String> {
    let request = if proxy {
        agent()
            .get(&format!("https://r.jina.ai/{url}"))
            .set("X-Return-Format", "text")
    } else {
        agent().get(url)
    };
    let response = request.call().map_err(short_error)?;
    response.into_string().map_err(|e| e.to_string())
}

/// Scarica un blocco di stazioni: prima proxy/diretto NMDB, poi il reader.
fn fetch_atlas_block(codes: &[&str]) -> Result<(Vec<String>, Vec<(i64, Vec<Option<f64>>)>), String> {
    let url = atlas_url(codes);
    let parsed = |text: &str| -> Option<(Vec<String>, Vec<(i64, Vec<Option<f64>>)>)> {
        let mut columns = atlas_columns(text);
        if columns.is_empty() {
            columns = codes.iter().map(|c| c.to_string()).collect();
        }
        let rows = parse_atlas_rows(text, columns.len());
        if rows.is_empty() {
            None
        } else {
            Some((columns, rows))
        }
    };
    let mut last_error = match nmdb_get(&url) {
        Ok(text) => match parsed(&text) {
            Some(block) => return Ok(block),
            None => "risposta NMDB senza tabella".to_string(),
        },
        Err(error) => error,
    };
    for attempt in 0..3 {
        match atlas_text(&url, true) {
            Ok(text) => match parsed(&text) {
                Some(block) => return Ok(block),
                None => last_error = "risposta del reader senza tabella".to_string(),
            },
            Err(error) => last_error = error,
        }
        std::thread::sleep(Duration::from_secs(8 * (attempt + 1) as u64));
    }
    Err(last_error)
}

/// Scarica la rete completa (7 giorni, passo orario) e la impacchetta nella
/// stessa griglia binaria degli asset, pronta per `atlas::Dataset`.
pub fn fetch_atlas_live() -> Result<crate::atlas::Grid, String> {
    use std::collections::BTreeMap;

    let all: Vec<&str> = crate::atlas::STATIONS.iter().map(|s| s.code).collect();
    let mut per_station: BTreeMap<&str, BTreeMap<i64, f64>> = BTreeMap::new();
    let mut last_error = String::from("nessun blocco scaricato");
    let mut blocks = 0;
    for chunk in all.chunks(14) {
        match fetch_atlas_block(chunk) {
            Ok((columns, rows)) => {
                blocks += 1;
                for (epoch, values) in rows {
                    for (code, value) in columns.iter().zip(values) {
                        let Some(&known) = all.iter().find(|c| **c == code.as_str()) else {
                            continue;
                        };
                        if let Some(value) = value {
                            per_station.entry(known).or_default().insert(epoch, value);
                        }
                    }
                }
            }
            Err(error) => last_error = error,
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    if blocks == 0 {
        return Err(last_error);
    }
    let mut times: Vec<i64> = Vec::new();
    for series in per_station.values() {
        for &t in series.keys() {
            times.push(t);
        }
    }
    times.sort_unstable();
    times.dedup();
    if times.len() < 24 {
        return Err(format!("serie troppo corta ({} ore)", times.len()));
    }
    let t0 = times[0];
    let step = 3600;
    let steps = ((times[times.len() - 1] - t0) / step + 1) as usize;
    let mut columns = Vec::new();
    let mut stations = Vec::new();
    for (index, station) in crate::atlas::STATIONS.iter().enumerate() {
        if per_station.contains_key(station.code) {
            columns.push(index);
            stations.push(station.code);
        }
    }
    if columns.len() < 20 {
        return Err(format!("solo {} stazioni", columns.len()));
    }
    let mut values = vec![f32::NAN; steps * columns.len()];
    for (col, code) in stations.iter().enumerate() {
        let Some(series) = per_station.get(code) else {
            continue;
        };
        for (&time, &value) in series {
            let step_index = ((time - t0) / step) as usize;
            if step_index < steps {
                values[step_index * columns.len() + col] = value as f32;
            }
        }
    }
    Ok(crate::atlas::Grid { t0, step, steps, columns, values })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_andata_e_ritorno() {
        let t = parse_epoch("2026-11-10 09:37:00").unwrap();
        assert_eq!(iso_minute(t), "2026-11-10 09:37");
        assert_eq!(ddmm(t), "10/11");
        assert_eq!(parse_epoch("2026-11-10T09:36:00Z"), Some(t - 60));
    }

    #[test]
    fn ora_legale_italiana() {
        assert_eq!(italy_offset(parse_epoch("2026-11-10 12:00").unwrap()), 3_600);
        assert_eq!(italy_offset(parse_epoch("2026-07-01 12:00").unwrap()), 7_200);
        // 2026: cambio il 29 marzo e il 25 ottobre alle 01:00 UTC
        assert_eq!(italy_offset(parse_epoch("2026-03-29 00:59").unwrap()), 3_600);
        assert_eq!(italy_offset(parse_epoch("2026-03-29 01:00").unwrap()), 7_200);
        assert_eq!(italy_offset(parse_epoch("2026-10-25 00:59").unwrap()), 7_200);
        assert_eq!(italy_offset(parse_epoch("2026-10-25 01:00").unwrap()), 3_600);
    }

    #[test]
    fn nmdb_ascii() {
        let body = "<pre><code>#x\n  start_date_time   RCORR_E\n2026-11-10 06:40:00;100.617\n2026-11-10 06:41:00; 97.716\n2026-11-10 06:42:00;   0\n</code></pre>";
        let s = parse_nmdb(body);
        assert_eq!(s.len(), 2);
        assert!((s[1].1 - 97.716).abs() < 1e-9);
    }

    #[test]
    fn scarta_valori_assurdi() {
        let mut s: Vec<(i64, f64)> = (0..10).map(|i| (i, 122.0)).collect();
        s[4].1 = 14_109.0;
        s[6].1 = 200.0; // +64 %: possibile GLE, resta
        let out = drop_outliers(s);
        assert_eq!(out.len(), 9);
        assert!(out.iter().all(|p| p.1 < 1_000.0));
    }

    #[test]
    fn tabella_atlante() {
        let text = "# STATION: OULU, ROME\n# start_date_time 1HCOR_E 1HCOR_E\n\
                    2026-11-10 06:00:00;100.6;50.2\n2026-11-10 07:00:00;101.0; null \n";
        assert_eq!(atlas_columns(text), vec!["OULU", "ROME"]);
        let rows = parse_atlas_rows(text, 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].1[0], Some(100.6));
        assert_eq!(rows[1].1[1], None);
        // il reader appiattisce tutto su una riga con i separatori " # "
        let flat = "# STATION: OULU # 2026-11-10 06:00:00;100.6 # 2026-11-10 07:00:00;101.0";
        let rows = parse_atlas_rows(flat, 1);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1].1[0], Some(101.0));
    }

    #[test]
    fn url_atlante() {
        let url = atlas_url(&["OULU", "ROME"]);
        assert!(url.contains("stations[]=OULU&stations[]=ROME"));
        assert!(url.contains("last_days=7"));
        assert!(url.contains("tresolution=60"));
    }

    #[test]
    fn proxy_solo_per_nmdb() {
        let nmdb = nmdb_url("OULU", "last_days=1&last_label=days_label");
        let proxied = proxied_url(&nmdb).expect("URL NMDB riconosciuto");
        assert!(proxied.starts_with(NMDB_PROXY));
        assert!(proxied.ends_with("stations[]=OULU&last_days=1&last_label=days_label"));
        assert!(proxied_url("https://services.swpc.noaa.gov/json/x.json").is_none());
        assert!(proxied_url(&atlas_url(&["OULU"])).is_some());
    }

    #[test]
    fn bollettini() {
        let v: serde_json::Value = serde_json::from_str(r#"[{"issue_datetime":"2026-11-10 05:01:21.150","message":"Space Weather Message Code: ALTEF3\r\nSerial Number: 3771\r\n\r\nCONTINUED ALERT: Electron 2MeV Integral Flux exceeded 1,000pfu\nBegin"}]"#).unwrap();
        let a = parse_alerts(&v);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].kind, "Allerta in corso");
        assert!(a[0].headline.starts_with("Electron 2MeV"));
    }
}
