//! Stato dei dati: serie unite in modo incrementale, cadenze per sorgente e
//! grandezze derivate (modulazione solare, Forbush, GLE).

use std::collections::HashMap;

use crate::api::{FetchResult, Feed, KpBlock, Payload, Request, Scales, SpaceAlert};
use crate::model::*;

const HOUR: i64 = 3_600;
const DAY: i64 = 86_400;

#[derive(Clone, Default)]
pub struct Data {
    /// Conteggi al minuto (conteggi/s), ultime ~26 ore.
    pub oulu: Vec<(i64, f64)>,
    pub rome: Vec<(i64, f64)>,
    /// Medie orarie degli ultimi 30 giorni.
    pub oulu_month: Vec<(i64, f64)>,
    pub rome_month: Vec<(i64, f64)>,
    pub kp: Vec<(i64, f64)>,
    pub kp_blocks: Vec<KpBlock>,
    pub xray: Vec<(i64, f64)>,
    pub protons: Vec<(i64, f64)>,
    pub dst: Vec<(i64, f64)>,
    pub f107: Vec<(i64, f64)>,
    pub wind: Vec<(i64, f64)>,
    pub density: Vec<(i64, f64)>,
    pub bz: Vec<(i64, f64)>,
    pub bt: Vec<(i64, f64)>,
    pub scales: Option<Scales>,
    pub alerts: Vec<SpaceAlert>,
    pub last_try: HashMap<Feed, i64>,
    pub last_ok: HashMap<Feed, i64>,
    pub errors: HashMap<Feed, String>,
}

/// Unisce `new` in `series` (ordinata), sostituendo i duplicati, e tiene solo `keep` secondi.
fn merge(series: &mut Vec<(i64, f64)>, new: Vec<(i64, f64)>, keep: i64) {
    if new.is_empty() {
        return;
    }
    let from = new[0].0;
    // i nuovi dati sono la verità per il loro intervallo (correzioni NMDB incluse)
    series.retain(|p| p.0 < from);
    series.extend(new);
    series.sort_by_key(|p| p.0);
    series.dedup_by_key(|p| p.0);
    if let Some(last) = series.last().map(|p| p.0) {
        series.retain(|p| p.0 >= last - keep);
    }
}

/// Aggiunge pochi punti recenti (feed "summary") senza cancellare la serie.
fn append(series: &mut Vec<(i64, f64)>, new: Vec<(i64, f64)>, keep: i64) {
    for p in new {
        if series.last().map_or(true, |l| p.0 > l.0) {
            series.push(p);
        }
    }
    if let Some(last) = series.last().map(|p| p.0) {
        series.retain(|p| p.0 >= last - keep);
    }
}

pub fn mean_between(series: &[(i64, f64)], from: i64, to: i64) -> Option<f64> {
    let (mut sum, mut n) = (0.0, 0usize);
    for p in series.iter().filter(|p| p.0 >= from && p.0 < to) {
        sum += p.1;
        n += 1;
    }
    (n > 0).then(|| sum / n as f64)
}

pub fn max_between(series: &[(i64, f64)], from: i64, to: i64) -> Option<f64> {
    series
        .iter()
        .filter(|p| p.0 >= from && p.0 < to)
        .map(|p| p.1)
        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.max(v))))
}

pub fn min_between(series: &[(i64, f64)], from: i64, to: i64) -> Option<f64> {
    series
        .iter()
        .filter(|p| p.0 >= from && p.0 < to)
        .map(|p| p.1)
        .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))))
}

/// Media degli ultimi `secs` secondi della serie (rispetto al suo ultimo punto),
/// solo se l'ultimo punto non è più vecchio di `max_age` rispetto a `now`.
fn recent_mean(series: &[(i64, f64)], secs: i64, now: i64, max_age: i64) -> Option<f64> {
    let last = series.last()?.0;
    if now - last > max_age {
        return None;
    }
    mean_between(series, last - secs + 1, last + 1)
}

pub fn last_fresh(series: &[(i64, f64)], now: i64, max_age: i64) -> Option<f64> {
    series.last().filter(|p| now - p.0 <= max_age).map(|p| p.1)
}

impl Data {
    /// Sorgenti da scaricare adesso. Una sorgente in errore riprova dopo 2 minuti.
    pub fn due(&self, now: i64, force: bool) -> Vec<Request> {
        Feed::ALL
            .iter()
            .filter(|feed| {
                if force {
                    return true;
                }
                let Some(last) = self.last_try.get(feed) else { return true };
                let every = if self.errors.contains_key(feed) {
                    feed.every_s().min(120)
                } else {
                    feed.every_s()
                };
                now - last >= every - 5
            })
            .map(|&feed| {
                let full = match feed {
                    Feed::Oulu => self.needs_full(&self.oulu, feed, now),
                    Feed::Rome => self.needs_full(&self.rome, feed, now),
                    _ => true,
                };
                Request { feed, full }
            })
            .collect()
    }

    fn needs_full(&self, series: &[(i64, f64)], feed: Feed, now: i64) -> bool {
        series.len() < 60
            || self.last_ok.get(&feed).map_or(true, |t| now - t > 2 * HOUR)
    }

    pub fn apply(&mut self, results: Vec<FetchResult>, now: i64) {
        for (feed, result) in results {
            self.last_try.insert(feed, now);
            let payload = match result {
                Ok(p) => p,
                Err(e) => {
                    self.errors.insert(feed, e);
                    continue;
                }
            };
            self.errors.remove(&feed);
            self.last_ok.insert(feed, now);
            match (feed, payload) {
                (Feed::Oulu, Payload::Nmdb(s)) => merge(&mut self.oulu, s, 26 * HOUR),
                (Feed::Rome, Payload::Nmdb(s)) => merge(&mut self.rome, s, 26 * HOUR),
                (Feed::Month, Payload::Month { oulu, rome }) => {
                    self.oulu_month = oulu;
                    if !rome.is_empty() {
                        self.rome_month = rome;
                    }
                }
                (Feed::Kp, Payload::Series(s)) => merge(&mut self.kp, s, 6 * HOUR),
                (Feed::KpForecast, Payload::KpForecast(b)) => self.kp_blocks = b,
                (Feed::Xray, Payload::Series(s)) => self.xray = s,
                (Feed::Protons, Payload::Series(s)) => self.protons = s,
                (Feed::Dst, Payload::Series(s)) => self.dst = s,
                (Feed::F107, Payload::Series(s)) => self.f107 = s,
                (Feed::Wind, Payload::Pair(speed, density)) => {
                    merge(&mut self.wind, speed, DAY);
                    merge(&mut self.density, density, DAY);
                }
                (Feed::Mag, Payload::Pair(bz, bt)) => {
                    merge(&mut self.bz, bz, DAY);
                    merge(&mut self.bt, bt, DAY);
                }
                (Feed::WindNow, Payload::Series(s)) => append(&mut self.wind, s, DAY),
                (Feed::MagNow, Payload::Pair(bz, bt)) => {
                    append(&mut self.bz, bz, DAY);
                    append(&mut self.bt, bt, DAY);
                }
                (Feed::Scales, Payload::Scales(s)) => self.scales = Some(s),
                (Feed::Alerts, Payload::Alerts(a)) => self.alerts = a,
                _ => {}
            }
        }
    }

    pub fn loaded_once(&self) -> bool {
        Feed::ALL.iter().all(|f| self.last_try.contains_key(f))
    }

    pub fn has_any(&self) -> bool {
        !self.last_ok.is_empty()
    }

    pub fn latest_update(&self) -> Option<i64> {
        self.last_ok.values().copied().max()
    }

    // ------------------------------------------------------------------
    // Grandezze derivate
    // ------------------------------------------------------------------

    /// Modulazione solare (%) per il modello: Oulu, media 30 min, rispetto alla media
    /// di lungo periodo. Usata ovunque (KPI, profilo di dose, mappa) per coerenza.
    pub fn modulation_pct(&self, now: i64) -> f64 {
        recent_mean(&self.oulu, 30 * 60, now, 6 * HOUR)
            .or_else(|| recent_mean(&self.oulu_month, 6 * HOUR, now, 2 * DAY))
            .map(|c| (c / OULU_LONG_MEAN - 1.0) * 100.0)
            .unwrap_or(0.0)
    }

    /// Oulu rispetto al minimo solare del 2009 (%), media dell'ultima ora.
    pub fn oulu_vs_min(&self, now: i64) -> Option<f64> {
        recent_mean(&self.oulu, HOUR, now, 6 * HOUR)
            .or_else(|| recent_mean(&self.oulu_month, 3 * HOUR, now, 2 * DAY))
            .map(|c| (c / OULU_SOLAR_MIN - 1.0) * 100.0)
    }

    pub fn rome_vs_min(&self, now: i64) -> Option<f64> {
        recent_mean(&self.rome, HOUR, now, 6 * HOUR)
            .or_else(|| recent_mean(&self.rome_month, 3 * HOUR, now, 2 * DAY))
            .map(|c| (c / ROME_SOLAR_MIN - 1.0) * 100.0)
    }

    /// Media dei 30 giorni (conteggi/s), base per le variazioni di breve periodo.
    fn baseline(&self, rome: bool) -> Option<f64> {
        let (month, minute) = if rome {
            (&self.rome_month, &self.rome)
        } else {
            (&self.oulu_month, &self.oulu)
        };
        let src = if month.len() >= 48 { month } else { minute };
        let n = src.len();
        (n > 0).then(|| src.iter().map(|p| p.1).sum::<f64>() / n as f64)
    }

    /// Serie al minuto in % rispetto alla media dei 30 giorni.
    pub fn short_variation(&self, rome: bool) -> Vec<(i64, f64)> {
        let series = if rome { &self.rome } else { &self.oulu };
        match self.baseline(rome) {
            Some(base) => series.iter().map(|p| (p.0, (p.1 / base - 1.0) * 100.0)).collect(),
            None => Vec::new(),
        }
    }

    /// Variazione rispetto alla media dei 30 giorni dell'ultima ora (%).
    pub fn oulu_vs_month(&self, now: i64) -> Option<f64> {
        let base = self.baseline(false)?;
        recent_mean(&self.oulu, HOUR, now, 6 * HOUR).map(|c| (c / base - 1.0) * 100.0)
    }

    /// Calo di Forbush: media delle ultime 3 ore di Oulu rispetto alla media 30 giorni (%).
    pub fn forbush_pct(&self, now: i64) -> Option<f64> {
        let base = self.baseline(false)?;
        if self.oulu_month.len() < 48 {
            return None;
        }
        recent_mean(&self.oulu, 3 * HOUR, now, HOUR).map(|c| (c / base - 1.0) * 100.0)
    }

    /// Indicatore GLE: media degli ultimi 10 minuti di Oulu rispetto alle 2 ore precedenti (%).
    pub fn gle_pct(&self, now: i64) -> Option<f64> {
        let last = self.oulu.last()?.0;
        if now - last > 20 * 60 {
            return None;
        }
        let recent_n = self.oulu.iter().filter(|p| p.0 > last - 600).count();
        if recent_n < 7 {
            return None;
        }
        let recent = mean_between(&self.oulu, last - 599, last + 1)?;
        let before = mean_between(&self.oulu, last - 600 - 2 * HOUR, last - 599)?;
        Some((recent / before - 1.0) * 100.0)
    }

    pub fn kp_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.kp, now, 3 * HOUR)
    }

    pub fn xray_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.xray, now, HOUR)
    }

    pub fn protons_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.protons, now, HOUR)
    }

    pub fn wind_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.wind, now, HOUR)
    }

    pub fn density_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.density, now, HOUR)
    }

    pub fn bz_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.bz, now, HOUR)
    }

    pub fn bt_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.bt, now, HOUR)
    }

    pub fn bz_mean30(&self, now: i64) -> Option<f64> {
        recent_mean(&self.bz, 30 * 60, now, HOUR)
    }

    pub fn wind_mean30(&self, now: i64) -> Option<f64> {
        recent_mean(&self.wind, 30 * 60, now, HOUR)
    }

    pub fn dst_now(&self, now: i64) -> Option<f64> {
        last_fresh(&self.dst, now, 6 * HOUR)
    }

    pub fn f107_now(&self) -> Option<f64> {
        self.f107.last().map(|p| p.1)
    }

    pub fn f107_mean(&self) -> Option<f64> {
        let n = self.f107.len();
        (n > 0).then(|| self.f107.iter().map(|p| p.1).sum::<f64>() / n as f64)
    }

    /// Kp medio di ogni ora (dal feed al minuto), cronologico.
    pub fn kp_hourly(&self) -> Vec<(i64, f64)> {
        let mut out: Vec<(i64, f64, usize)> = Vec::new();
        for (t, v) in &self.kp {
            let h = t.div_euclid(HOUR) * HOUR;
            match out.last_mut() {
                Some(last) if last.0 == h => {
                    last.1 += v;
                    last.2 += 1;
                }
                _ => out.push((h, *v, 1)),
            }
        }
        out.into_iter().map(|(h, s, n)| (h, s / n as f64)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_incrementale() {
        let mut s = vec![(0, 1.0), (60, 2.0), (120, 3.0)];
        merge(&mut s, vec![(120, 3.5), (180, 4.0)], 1_000);
        assert_eq!(s, vec![(0, 1.0), (60, 2.0), (120, 3.5), (180, 4.0)]);
        merge(&mut s, vec![(2_000, 5.0)], 1_000);
        assert_eq!(s, vec![(2_000, 5.0)]);
    }

    #[test]
    fn gle_rileva_un_aumento() {
        let mut d = Data::default();
        let now = 100_000;
        for i in 0..150 {
            let t = now - (150 - i) * 60;
            let v = if i >= 140 { 110.0 } else { 100.0 };
            d.oulu.push((t, v));
        }
        let gle = d.gle_pct(now).unwrap();
        assert!(gle > 8.0 && gle < 11.0, "{gle}");
    }

    #[test]
    fn kp_orario_e_una_media() {
        let mut d = Data::default();
        d.kp = vec![(3_600, 1.0), (3_660, 3.0), (7_200, 5.0)];
        assert_eq!(d.kp_hourly(), vec![(3_600, 2.0), (7_200, 5.0)]);
    }
}
