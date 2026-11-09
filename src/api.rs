//! Acquisizione e parsing dei dati live.
//!
//! Sorgenti:
//!  * NMDB (neutron monitor Oulu)  -> variazione percentuale dei raggi cosmici al suolo
//!  * NOAA SWPC (Kp, X-ray GOES, protoni, vento solare, Dst, F10.7) -> condizioni solari

use std::time::Duration;

pub const NMDB_URL: &str = "https://www.nmdb.eu/nest/draw_graph.php?formchk=1&stations[]=OULU&tabchoice=rev1h&dtype=corr_for_efficiency&date_choice=last&last=24&output=ascii";
pub const NMDB_ROME_URL: &str = "https://www.nmdb.eu/nest/draw_graph.php?formchk=1&stations[]=ROME&tabchoice=rev1h&dtype=corr_for_efficiency&date_choice=last&last=24&output=ascii";
const KP_URL: &str = "https://services.swpc.noaa.gov/json/planetary_k_index_1m.json";
const XRAY_URL: &str = "https://services.swpc.noaa.gov/json/goes/primary/xrays-6-hour.json";
const PROTON_URL: &str = "https://services.swpc.noaa.gov/json/goes/primary/integral-protons-6-hour.json";
const DST_URL: &str = "https://services.swpc.noaa.gov/products/kyoto-dst.json";
const F107_URL: &str = "https://services.swpc.noaa.gov/products/10cm-flux-30-day.json";
const WIND_URL: &str = "https://services.swpc.noaa.gov/json/rtsw/rtsw_wind_1m.json";
const MAG_URL: &str = "https://services.swpc.noaa.gov/json/rtsw/rtsw_mag_1m.json";

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// Variazione percentuale del conteggio del neutron monitor, in ordine cronologico.
    pub gcr_pct: Vec<(i64, f64)>,
    /// Stessa grandezza per il monitor di Roma (taglio geomagnetico più alto).
    pub gcr_rome: Vec<(i64, f64)>,
    /// Kp stimato (1 minuto), ordine cronologico.
    pub kp: Vec<(i64, f64)>,
    /// Flusso X-ray 0.1-0.8 nm (W/m²), ordine cronologico.
    pub xray: Vec<(i64, f64)>,
    /// Dst orario (nT), ordine cronologico.
    pub dst_series: Vec<(i64, f64)>,
    /// Bz IMF (nT), ordine cronologico.
    pub bz_series: Vec<(i64, f64)>,
    /// Velocità del vento solare (km/s), ordine cronologico.
    pub wind_series: Vec<(i64, f64)>,
    /// Protoni ≥10 MeV (pfu), ordine cronologico. Il canale ≥1 MeV è spesso contaminato.
    pub proton_series: Vec<(i64, f64)>,
    pub proton_pfu: Option<f64>,
    pub wind_speed: Option<f64>,
    pub bz_nt: Option<f64>,
    pub dst_nt: Option<f64>,
    pub f107: Option<f64>,
    pub fetched_at: i64,
    pub notes: Vec<String>,
}

impl Snapshot {
    pub fn gcr_now(&self) -> Option<f64> {
        self.gcr_pct.last().map(|p| p.1)
    }

    /// Media degli ultimi `n` campioni al minuto: la serie NMDB è rumorosa.
    pub fn gcr_now_avg(&self, n: usize) -> Option<f64> {
        let n = n.min(self.gcr_pct.len());
        if n == 0 {
            return None;
        }
        let sum: f64 = self.gcr_pct[self.gcr_pct.len() - n..]
            .iter()
            .map(|p| p.1)
            .sum();
        Some(sum / n as f64)
    }

    pub fn gcr_range(&self) -> Option<(f64, f64)> {
        let mut iter = self.gcr_pct.iter().map(|p| p.1);
        let first = iter.next()?;
        Some(iter.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v))))
    }

    pub fn kp_now(&self) -> Option<f64> {
        self.kp.last().map(|p| p.1)
    }

    pub fn xray_now(&self) -> Option<f64> {
        self.xray.last().map(|p| p.1)
    }
}

fn http_get(url: &str) -> Result<String, String> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(25))
        .call()
        .map_err(|e| format!("{url}: {e}"))?;
    response.into_string().map_err(|e| format!("{url}: {e}"))
}

fn http_json(url: &str) -> Result<serde_json::Value, String> {
    let body = http_get(url)?;
    serde_json::from_str(&body).map_err(|e| format!("{url}: JSON non valido ({e})"))
}

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

/// Converte `YYYY-MM-DD[ T]HH:MM:SS` (UTC) in secondi dall'epoca.
pub fn parse_epoch(text: &str) -> Option<i64> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let y: i64 = text.get(0..4)?.parse().ok()?;
    let mo: i64 = text.get(5..7)?.parse().ok()?;
    let d: i64 = text.get(8..10)?.parse().ok()?;
    let h: i64 = text.get(11..13)?.parse().ok()?;
    let mi: i64 = text.get(14..16)?.parse().ok()?;
    let s: i64 = text.get(17..19)?.parse().ok()?;
    Some(days_from_civil(y, mo, d) * 86_400 + h * 3_600 + mi * 60 + s)
}

/// Formatta un istante epoch come `HH:MM` UTC.
pub fn hhmm_utc(epoch: i64) -> String {
    let secs = epoch.rem_euclid(86_400);
    format!("{:02}:{:02}", secs / 3_600, (secs % 3_600) / 60)
}

pub fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Tiene solo le ultime 24 ore di una serie (NMDB ne restituisce 72).
fn trim_last_day(points: &mut Vec<(i64, f64)>) {
    if let Some(last) = points.last().map(|p| p.0) {
        points.retain(|p| p.0 >= last - 86_400);
    }
}

fn parse_nmdb(body: &str) -> Vec<(i64, f64)> {    let start = body.find("<code>").map(|i| i + 6).unwrap_or(0);
    let end = body[start..]
        .find("</code>")
        .map(|i| start + i)
        .unwrap_or(body.len());
    let mut out = Vec::new();
    for line in body[start..end].lines() {
        let line = line.trim();
        if line.is_empty() || !line.contains(';') {
            continue;
        }
        let mut parts = line.splitn(2, ';');
        let (Some(stamp), Some(value)) = (parts.next(), parts.next()) else {
            continue;
        };
        let Some(epoch) = parse_epoch(stamp) else {
            continue;
        };
        let value = value.trim().replace(',', ".");
        if let Ok(value) = value.parse::<f64>() {
            if value.is_finite() {
                out.push((epoch, value));
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out
}

fn parse_kp(value: &serde_json::Value) -> Vec<(i64, f64)> {
    let mut out = Vec::new();
    if let Some(rows) = value.as_array() {
        for row in rows {
            let Some(stamp) = row.get("time_tag").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(epoch) = parse_epoch(stamp) else {
                continue;
            };
            let kp = row
                .get("estimated_kp")
                .and_then(|v| v.as_f64())
                .or_else(|| row.get("kp_index").and_then(|v| v.as_f64()));
            if let Some(kp) = kp {
                out.push((epoch, kp));
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out
}

fn parse_xray(value: &serde_json::Value) -> Vec<(i64, f64)> {
    let mut out = Vec::new();
    if let Some(rows) = value.as_array() {
        for row in rows {
            if row.get("energy").and_then(|v| v.as_str()) != Some("0.1-0.8nm") {
                continue;
            }
            let Some(stamp) = row.get("time_tag").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(epoch) = parse_epoch(stamp) else {
                continue;
            };
            if let Some(flux) = row.get("flux").and_then(|v| v.as_f64()) {
                if flux > 0.0 {
                    out.push((epoch, flux));
                }
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out
}

/// Serie generica da un array JSON con `time_tag` e un campo numerico.
fn parse_series(value: &serde_json::Value, field: &str) -> Vec<(i64, f64)> {
    let mut out = Vec::new();
    if let Some(rows) = value.as_array() {
        for row in rows {
            let Some(stamp) = row.get("time_tag").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(epoch) = parse_epoch(stamp) else {
                continue;
            };
            if let Some(v) = row.get(field).and_then(|v| v.as_f64()) {
                if v.is_finite() {
                    out.push((epoch, v));
                }
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out.dedup_by_key(|p| p.0);
    out
}

/// Serie dai feed RTSW, che mescolano più sonde: conta solo le righe `active`.
fn parse_rtsw(value: &serde_json::Value, field: &str) -> Vec<(i64, f64)> {
    let mut out = Vec::new();
    if let Some(rows) = value.as_array() {
        for row in rows {
            if row.get("active").and_then(|v| v.as_bool()) == Some(false) {
                continue;
            }
            let Some(stamp) = row.get("time_tag").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(epoch) = parse_epoch(stamp) else {
                continue;
            };
            if let Some(v) = row.get(field).and_then(|v| v.as_f64()) {
                if v.is_finite() {
                    out.push((epoch, v));
                }
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out.dedup_by_key(|p| p.0);
    out
}

fn parse_protons(value: &serde_json::Value) -> Vec<(i64, f64)> {
    let mut out = Vec::new();
    if let Some(rows) = value.as_array() {
        for row in rows {
            if row.get("energy").and_then(|v| v.as_str()) != Some(">=10 MeV") {
                continue;
            }
            let Some(stamp) = row.get("time_tag").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(epoch) = parse_epoch(stamp) else {
                continue;
            };
            if let Some(flux) = row.get("flux").and_then(|v| v.as_f64()) {
                if flux > 0.0 {
                    out.push((epoch, flux));
                }
            }
        }
    }
    out.sort_by_key(|p| p.0);
    out
}

/// Scarica tutto. `deep` abilita i feed pesanti (vento solare, IMF).
pub fn fetch(deep: bool) -> Snapshot {
    let mut snap = Snapshot {
        fetched_at: now_epoch(),
        ..Default::default()
    };

    match http_get(NMDB_URL) {
        Ok(body) => {
            snap.gcr_pct = parse_nmdb(&body);
            trim_last_day(&mut snap.gcr_pct);
            if snap.gcr_pct.is_empty() {
                snap.notes.push("NMDB: risposta senza serie dati".into());
            }
        }
        Err(e) => snap.notes.push(format!("NMDB non raggiungibile ({e})")),
    }

    match http_json(KP_URL) {
        Ok(value) => snap.kp = parse_kp(&value),
        Err(e) => snap.notes.push(format!("Kp non disponibile ({e})")),
    }

    match http_json(XRAY_URL) {
        Ok(value) => {
            snap.xray = parse_xray(&value);
        }
        Err(e) => snap.notes.push(format!("X-ray non disponibile ({e})")),
    }

    match http_json(PROTON_URL) {
        Ok(value) => {
            snap.proton_series = parse_protons(&value);
            snap.proton_pfu = snap.proton_series.last().map(|p| p.1);        }
        Err(_) => snap.notes.push("Protoni GOES non disponibili".into()),
    }

    match http_json(DST_URL) {
        Ok(value) => {
            snap.dst_series = parse_series(&value, "dst");
            snap.dst_nt = snap.dst_series.last().map(|p| p.1);
        }
        Err(_) => snap.notes.push("Dst non disponibile".into()),
    }

    match http_json(F107_URL) {
        Ok(value) => {
            snap.f107 = value
                .as_array()
                .and_then(|rows| rows.last())
                .and_then(|row| row.get("flux"))
                .and_then(|v| v.as_f64());
        }
        Err(_) => snap.notes.push("F10.7 non disponibile".into()),
    }

    if deep {
        match http_json(WIND_URL) {
            Ok(value) => {
                snap.wind_series = parse_rtsw(&value, "proton_speed");
                snap.wind_speed = snap.wind_series.last().map(|p| p.1);
            }
            Err(_) => snap.notes.push("Vento solare non disponibile".into()),
        }
        match http_json(MAG_URL) {
            Ok(value) => {
                snap.bz_series = parse_rtsw(&value, "bz_gsm");
                snap.bz_nt = snap.bz_series.last().map(|p| p.1);
            }
            Err(_) => snap.notes.push("Campo magnetico non disponibile".into()),
        }
        match http_get(NMDB_ROME_URL) {
            Ok(body) => {
                snap.gcr_rome = parse_nmdb(&body);
                trim_last_day(&mut snap.gcr_rome);
            }
            Err(_) => snap.notes.push("NMDB Roma non disponibile".into()),
        }
    }

    snap
}
