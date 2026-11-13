//! Atlante dei raggi cosmici: rete mondiale dei neutron monitor, eventi storici
//! e ciclo solare. I dati sono impacchettati in asset binari compatti.

/// Stazione della rete NMDB: codice, nome, paese, coordinate, rigidità di taglio
/// verticale (GV) e quota (m).
pub struct Station {
    pub code: &'static str,
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub rc: f64,
    pub altitude_m: f64,
}

/// 42 stazioni della rete NMDB, scelte per coprire tutte le latitudini
/// geomagnetiche e per avere serie storiche lunghe.
pub const STATIONS: &[Station] = &[
    Station { code: "APTY", name: "Apatity", country: "RU", lat: 67.57, lon: 33.40, rc: 0.65, altitude_m: 181.0 },
    Station { code: "OULU", name: "Oulu", country: "FI", lat: 65.05, lon: 25.47, rc: 0.81, altitude_m: 15.0 },
    Station { code: "INVK", name: "Inuvik", country: "CA", lat: 68.36, lon: -133.72, rc: 0.30, altitude_m: 21.0 },
    Station { code: "FSMT", name: "Fort Smith", country: "CA", lat: 60.02, lon: -111.93, rc: 0.30, altitude_m: 180.0 },
    Station { code: "THUL", name: "Thule", country: "GL", lat: 76.60, lon: -68.80, rc: 0.30, altitude_m: 26.0 },
    Station { code: "NAIN", name: "Nain", country: "CA", lat: 56.55, lon: -61.68, rc: 0.30, altitude_m: 46.0 },
    Station { code: "PWNK", name: "Peawanuck", country: "CA", lat: 55.02, lon: -85.53, rc: 0.30, altitude_m: 53.0 },
    Station { code: "JBGO", name: "Jang Bogo", country: "AQ", lat: -74.62, lon: 164.22, rc: 0.30, altitude_m: 29.0 },
    Station { code: "MCMU", name: "McMurdo", country: "AQ", lat: -77.85, lon: 166.72, rc: 0.30, altitude_m: 48.0 },
    Station { code: "SOPO", name: "South Pole", country: "AQ", lat: -90.00, lon: 0.00, rc: 0.10, altitude_m: 2820.0 },
    Station { code: "DOMC", name: "Dome C", country: "AQ", lat: -75.10, lon: 123.35, rc: 0.01, altitude_m: 3233.0 },
    Station { code: "TERA", name: "Terre Adélie", country: "AQ", lat: -66.67, lon: 140.02, rc: 0.01, altitude_m: 32.0 },
    Station { code: "MRNY", name: "Mirny", country: "AQ", lat: -66.55, lon: 93.02, rc: 0.03, altitude_m: 30.0 },
    Station { code: "MWSN", name: "Mawson", country: "AQ", lat: -67.60, lon: 62.88, rc: 0.22, altitude_m: 30.0 },
    Station { code: "NEU3", name: "Neumayer III", country: "AQ", lat: -70.65, lon: -8.25, rc: 0.10, altitude_m: 40.0 },
    Station { code: "SNAE", name: "SANAE IV", country: "AQ", lat: -71.68, lon: -2.85, rc: 0.73, altitude_m: 856.0 },
    Station { code: "KERG", name: "Kerguelen", country: "FR", lat: -49.35, lon: 70.25, rc: 1.14, altitude_m: 33.0 },
    Station { code: "CALG", name: "Calgary", country: "CA", lat: 51.08, lon: -114.13, rc: 1.08, altitude_m: 1123.0 },
    Station { code: "YKTK", name: "Yakutsk", country: "RU", lat: 62.02, lon: 129.72, rc: 1.65, altitude_m: 105.0 },
    Station { code: "IRKT", name: "Irkutsk", country: "RU", lat: 52.47, lon: 104.03, rc: 3.64, altitude_m: 435.0 },
    Station { code: "NVBK", name: "Novosibirsk", country: "RU", lat: 54.85, lon: 83.10, rc: 2.91, altitude_m: 163.0 },
    Station { code: "MGDN", name: "Magadan", country: "RU", lat: 60.05, lon: 150.80, rc: 2.10, altitude_m: 220.0 },
    Station { code: "MOSC", name: "Moscow", country: "RU", lat: 55.47, lon: 37.32, rc: 2.43, altitude_m: 200.0 },
    Station { code: "KIEL", name: "Kiel", country: "DE", lat: 54.34, lon: 10.12, rc: 2.36, altitude_m: 54.0 },
    Station { code: "DRBS", name: "Dourbes", country: "BE", lat: 50.10, lon: 4.60, rc: 3.18, altitude_m: 225.0 },
    Station { code: "LMKS", name: "Lomnický štít", country: "SK", lat: 49.20, lon: 20.22, rc: 3.84, altitude_m: 2634.0 },
    Station { code: "JUNG", name: "Jungfraujoch", country: "CH", lat: 46.55, lon: 7.98, rc: 4.49, altitude_m: 3575.0 },
    Station { code: "ZUGS", name: "Zugspitze", country: "DE", lat: 47.42, lon: 10.98, rc: 4.24, altitude_m: 2960.0 },
    Station { code: "CLMX", name: "Climax", country: "US", lat: 39.37, lon: -106.18, rc: 3.00, altitude_m: 3400.0 },
    Station { code: "NEWK", name: "Newark", country: "US", lat: 39.68, lon: -75.75, rc: 2.40, altitude_m: 50.0 },
    Station { code: "HRMS", name: "Hermanus", country: "ZA", lat: -34.42, lon: 19.22, rc: 4.58, altitude_m: 26.0 },
    Station { code: "PTFM", name: "Potchefstroom", country: "ZA", lat: -26.70, lon: 27.10, rc: 6.98, altitude_m: 1351.0 },
    Station { code: "TSMB", name: "Tsumeb", country: "NA", lat: -19.20, lon: 17.70, rc: 9.15, altitude_m: 1240.0 },
    Station { code: "ATHN", name: "Athens", country: "GR", lat: 37.97, lon: 23.72, rc: 8.53, altitude_m: 260.0 },
    Station { code: "ROME", name: "Rome", country: "IT", lat: 41.86, lon: 12.47, rc: 6.27, altitude_m: 0.0 },
    Station { code: "BKSN", name: "Baksan", country: "RU", lat: 43.28, lon: 42.69, rc: 5.70, altitude_m: 1700.0 },
    Station { code: "MXCO", name: "Mexico City", country: "MX", lat: 19.33, lon: -99.18, rc: 8.28, altitude_m: 2274.0 },
    Station { code: "HUAN", name: "Huancayo", country: "PE", lat: -12.03, lon: -75.33, rc: 12.92, altitude_m: 3400.0 },
    Station { code: "CHAC", name: "Chacaltaya", country: "BO", lat: -16.35, lon: -68.13, rc: 11.80, altitude_m: 5270.0 },
    Station { code: "AHMD", name: "Ahmedabad", country: "IN", lat: 23.02, lon: 72.60, rc: 15.94, altitude_m: 50.0 },
    Station { code: "PSNM", name: "Doi Inthanon", country: "TH", lat: 18.59, lon: 98.49, rc: 16.80, altitude_m: 2565.0 },
    Station { code: "KGSN", name: "Kingston", country: "AU", lat: -42.98, lon: 147.29, rc: 1.88, altitude_m: 65.0 },
];

pub fn station_index(code: &str) -> Option<usize> {
    STATIONS.iter().position(|s| s.code == code)
}

// ---------------------------------------------------------------------------
// Formato binario degli asset
// ---------------------------------------------------------------------------

/// Istante iniziale (epoch), passo (s) e griglia di valori in un asset.
#[derive(Clone)]
pub struct Grid {
    pub t0: i64,
    pub step: i64,
    pub steps: usize,
    /// Colonna -> indice in `STATIONS`.
    pub columns: Vec<usize>,
    /// Valori riga per riga (`steps * columns.len()`), NaN = buco.
    pub values: Vec<f32>,
}

impl Grid {
    /// Legge il formato: `NMDA`, versione, stazioni, passi, t0, passo,
    /// riservato, codici ASCII da 4 byte, poi f32 little endian.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 28 || &bytes[0..4] != b"NMDA" {
            return None;
        }
        let read_u16 = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let read_u32 = |at: usize| {
            u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
        };
        let version = read_u16(4);
        if version != 1 {
            return None;
        }
        let nst = read_u16(6) as usize;
        let nsteps = read_u32(8) as usize;
        let t0 = i64::from_le_bytes([
            bytes[12], bytes[13], bytes[14], bytes[15],
            bytes[16], bytes[17], bytes[18], bytes[19],
        ]);
        let step = read_u32(20) as i64;
        let start = 28;
        let end = start + nst * 4;
        if bytes.len() < end {
            return None;
        }
        let count = nsteps.checked_mul(nst)?;
        let data = &bytes[end..];
        if data.len() < count * 4 {
            return None;
        }
        let mut columns = Vec::with_capacity(nst);
        let mut file_to_known = vec![usize::MAX; nst];
        for i in 0..nst {
            let code = std::str::from_utf8(&bytes[start + i * 4..start + i * 4 + 4]).ok()?;
            let code = code.trim();
            if let Some(index) = station_index(code) {
                file_to_known[i] = columns.len();
                columns.push(index);
            }
        }
        let ncol = columns.len();
        let mut values = vec![f32::NAN; nsteps * ncol];
        for step_index in 0..nsteps {
            for file_col in 0..nst {
                let known = file_to_known[file_col];
                if known == usize::MAX {
                    continue;
                }
                let at = (step_index * nst + file_col) * 4;
                let value = f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
                values[step_index * ncol + known] = value;
            }
        }
        Some(Self { t0, step, steps: nsteps, columns, values })
    }

    pub fn value(&self, step: usize, col: usize) -> Option<f32> {
        let value = *self.values.get(step * self.columns.len() + col)?;
        value.is_finite().then_some(value)
    }
}

// ---------------------------------------------------------------------------
// Eventi storici
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Gle,
    Forbush,
}

pub struct Event {
    /// Identificativo usato nei test e nei messaggi di errore.
    #[allow(dead_code)]
    pub id: &'static str,
    pub kind: EventKind,
    pub label: &'static str,
    pub date: &'static str,
    pub blurb: &'static str,
    pub bytes: &'static [u8],
}

pub const EVENTS: &[Event] = &[
    Event {
        id: "gle66",
        kind: EventKind::Gle,
        label: "GLE 66 · Halloween",
        date: "29 ottobre 2003",
        blurb: "La sequenza di brillamenti e tempeste del 28-30 ottobre 2003: il GLE più intenso e meglio osservato dell'era moderna. Gli aumenti al suolo arrivano a decine di punti percentuali alle stazioni polari, mentre a rigidità alta non si vede quasi nulla.",
        bytes: include_bytes!("../assets/atlas/events/event_gle66.bin"),
    },
    Event {
        id: "gle69",
        kind: EventKind::Gle,
        label: "GLE 69 · gennaio 2005",
        date: "20 gennaio 2005",
        blurb: "Il brillamento X7 del 20 gennaio 2005 accelera protoni fino a GeV. Le stazioni polari vedono un aumento improvviso, quelle a rigidità alta quasi nulla: è il taglio geomagnetico in azione.",
        bytes: include_bytes!("../assets/atlas/events/event_gle69.bin"),
    },
    Event {
        id: "gle71",
        kind: EventKind::Gle,
        label: "GLE 71 · maggio 2012",
        date: "17 maggio 2012",
        blurb: "Un GLE che arriva mentre è già in corso una tempesta geomagnetica: il campo è disturbato e la risposta delle stazioni è irregolare, con aumenti e cali che convivono sulla stessa mappa.",
        bytes: include_bytes!("../assets/atlas/events/event_gle71.bin"),
    },
    Event {
        id: "gle74",
        kind: EventKind::Gle,
        label: "GLE 74 · maggio 2024",
        date: "11 maggio 2024",
        blurb: "Il GLE della grande tempesta G5 del maggio 2024, osservato dalla rete attuale: è l'evento più recente e quello con più stazioni simultanee.",
        bytes: include_bytes!("../assets/atlas/events/event_may2024.bin"),
    },
    Event {
        id: "fd1991",
        kind: EventKind::Forbush,
        label: "Forbush · giugno 1991",
        date: "12 giugno 1991",
        blurb: "Uno dei cali di Forbush più profondi registrati: oltre il 20 % in alcune stazioni. La depressione si propaga dal polo verso l'equatore in poche ore. Nel 1991 la rete era molto più piccola di oggi: i punti colorati sono soltanto otto, e questo fa parte della storia.",
        bytes: include_bytes!("../assets/atlas/events/event_fd1991.bin"),
    },
    Event {
        id: "fd2003",
        kind: EventKind::Forbush,
        label: "Forbush · novembre 2003",
        date: "20 novembre 2003",
        blurb: "Il calo del 20 novembre 2003 segue la grande tempesta di Halloween: la mappa mostra quanto la depressione sia globale e quanto dipenda dalla rigidità di taglio.",
        bytes: include_bytes!("../assets/atlas/events/event_fd2003.bin"),
    },
    Event {
        id: "fd2015",
        kind: EventKind::Forbush,
        label: "Forbush · giugno 2015",
        date: "22 giugno 2015",
        blurb: "Un calo di Forbush da tempesta geomagnetica moderata, utile per seguire la propagazione senza gli effetti estremi dei grandi eventi.",
        bytes: include_bytes!("../assets/atlas/events/event_fd2015.bin"),
    },
    Event {
        id: "fd2024",
        kind: EventKind::Forbush,
        label: "Forbush · maggio 2024",
        date: "10 maggio 2024",
        blurb: "La depressione che accompagna la tempesta G5 del maggio 2024, vista dalla rete completa: prima il calo, poi il GLE del giorno dopo.",
        bytes: include_bytes!("../assets/atlas/events/event_may2024.bin"),
    },
    Event {
        id: "fd2023",
        kind: EventKind::Forbush,
        label: "Forbush · aprile 2023",
        date: "23 aprile 2023",
        blurb: "Il calo di Forbush più profondo del ciclo solare 25: la tempesta geomagnetica G4 spazza via una parte dei raggi cosmici e la rete attuale la vede tutta insieme, dal polo all'equatore.",
        bytes: include_bytes!("../assets/atlas/events/event_fd2023.bin"),
    },
];

pub const LIVE: &[u8] = include_bytes!("../assets/atlas/live.bin");
pub const SOLAR_CSV: &str = include_str!("../assets/atlas/solar.csv");

// ---------------------------------------------------------------------------
// Serie temporale pronta per il disegno
// ---------------------------------------------------------------------------

pub struct Dataset {
    pub t0: i64,
    pub step: i64,
    pub steps: usize,
    /// Indice della stazione per ogni colonna.
    pub columns: Vec<usize>,
    /// Valori percentuali rispetto alla base, riga per riga.
    pub pct: Vec<f32>,
    /// Larghezza della scala verticale dei grafici, in punti percentuali.
    pub scale_pct: f64,
}

impl Dataset {
    pub fn from_grid(grid: Grid, live: bool, baseline_steps: Option<usize>) -> Self {
        let ncol = grid.columns.len();
        let mut baseline = vec![f32::NAN; ncol];
        let window = baseline_steps.unwrap_or(grid.steps).min(grid.steps);
        for col in 0..ncol {
            let mut values: Vec<f32> = (0..window)
                .filter_map(|step| grid.value(step, col))
                .collect();
            if values.is_empty() {
                continue;
            }
            values.sort_by(|a, b| a.total_cmp(b));
            baseline[col] = values[values.len() / 2];
        }
        let mut pct = vec![f32::NAN; grid.steps * ncol];
        for step in 0..grid.steps {
            for col in 0..ncol {
                if let Some(value) = grid.value(step, col) {
                    let base = baseline[col];
                    if base.is_finite() && base > 0.0 {
                        pct[step * ncol + col] = (value / base - 1.0) * 100.0;
                    }
                }
            }
        }
        let mut spread: Vec<f32> = pct.iter().copied().filter(|v| v.is_finite()).collect();
        spread.sort_by(|a, b| a.total_cmp(b));
        let percentile = |q: f64| -> f64 {
            if spread.is_empty() {
                0.0
            } else {
                let at = ((spread.len() - 1) as f64 * q).round() as usize;
                spread[at].abs() as f64
            }
        };
        let scale_pct = if live {
            percentile(0.98).clamp(2.0, 12.0)
        } else {
            percentile(0.99).clamp(5.0, 60.0)
        };
        Self { t0: grid.t0, step: grid.step, steps: grid.steps,
               columns: grid.columns, pct, scale_pct }
    }

    pub fn ncol(&self) -> usize {
        self.columns.len()
    }

    pub fn col_of(&self, station: usize) -> Option<usize> {
        self.columns.iter().position(|&s| s == station)
    }

    pub fn pct_at(&self, step: usize, col: usize) -> Option<f64> {
        let value = *self.pct.get(step * self.ncol() + col)?;
        value.is_finite().then_some(value as f64)
    }

    /// `true` se la stazione ha almeno un valore valido nella serie attiva.
    pub fn has_data(&self, station: usize) -> bool {
        let Some(col) = self.col_of(station) else {
            return false;
        };
        (0..self.steps).any(|step| self.pct_at(step, col).is_some())
    }

    /// Numero di stazioni che hanno almeno un valore valido nella serie attiva.
    pub fn active_count(&self) -> usize {
        (0..self.ncol())
            .filter(|&col| (0..self.steps).any(|step| self.pct_at(step, col).is_some()))
            .count()
    }

    pub fn step_time(&self, step: usize) -> i64 {
        self.t0 + step as i64 * self.step
    }

    /// Media, minimo e massimo della variazione allo step, sulle stazioni con dato.
    pub fn stats(&self, step: usize) -> (f64, f64, f64, usize) {
        let mut sum = 0.0;
        let mut count = 0;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for col in 0..self.ncol() {
            if let Some(v) = self.pct_at(step, col) {
                sum += v;
                count += 1;
                min = min.min(v);
                max = max.max(v);
            }
        }
        if count == 0 {
            (0.0, 0.0, 0.0, 0)
        } else {
            (sum / count as f64, min, max, count)
        }
    }
}

/// Carica un evento con la base presa nelle prime 6 ore della finestra.
pub fn load_event(event: &Event) -> Option<Dataset> {
    let grid = Grid::parse(event.bytes)?;
    let baseline = Some(((6 * 3600) / grid.step.max(1)) as usize);
    Some(Dataset::from_grid(grid, false, baseline))
}

pub fn load_live() -> Option<Dataset> {
    let grid = Grid::parse(LIVE)?;
    Some(Dataset::from_grid(grid, true, None))
}

// ---------------------------------------------------------------------------
// Colori e scala delle variazioni
// ---------------------------------------------------------------------------

/// Scala continua per la variazione percentuale: blu (calo) → grigio (zero)
/// → giallo/rosso (aumento).
pub fn variation_color(pct: f64) -> (u8, u8, u8) {
    const STOPS: &[(f64, (u8, u8, u8))] = &[
        (-20.0, (37, 99, 235)),
        (-10.0, (14, 165, 233)),
        (-4.0, (34, 211, 238)),
        (0.0, (148, 163, 184)),
        (3.0, (250, 204, 21)),
        (8.0, (251, 146, 60)),
        (15.0, (244, 63, 94)),
        (30.0, (190, 24, 93)),
    ];
    let v = pct.clamp(STOPS[0].0, STOPS[STOPS.len() - 1].0);
    for pair in STOPS.windows(2) {
        let (v0, c0) = pair[0];
        let (v1, c1) = pair[1];
        if v >= v0 && v <= v1 {
            let t = (v - v0) / (v1 - v0);
            let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
            return (mix(c0.0, c1.0), mix(c0.1, c1.1), mix(c0.2, c1.2));
        }
    }
    STOPS[0].1
}

pub fn kind_color(kind: EventKind) -> (u8, u8, u8) {
    match kind {
        EventKind::Gle => (251, 146, 60),
        EventKind::Forbush => (56, 189, 248),
    }
}

// ---------------------------------------------------------------------------
// Ciclo solare (Oulu mensile + macchie solari SILSO)
// ---------------------------------------------------------------------------

pub struct SolarMonth {
    /// Anno frazionario (centro del mese).
    pub year: f64,
    /// Variazione percentuale di Oulu rispetto alla media 1964-2026.
    pub oulu: Option<f64>,
    pub ssn: Option<f64>,
}

pub fn solar_series() -> Vec<SolarMonth> {
    let mut raw: Vec<(f64, Option<f64>, Option<f64>)> = Vec::new();
    for line in SOLAR_CSV.lines().skip(1) {
        let mut parts = line.split(';');
        let (Some(month), Some(oulu), Some(ssn)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let (Some(y), Some(m)) = (month.get(0..4), month.get(5..7)) else {
            continue;
        };
        let (Ok(y), Ok(m)) = (y.parse::<i64>(), m.parse::<i64>()) else {
            continue;
        };
        let oulu = oulu.trim().replace(',', ".").parse::<f64>().ok();
        let ssn = ssn.trim().replace(',', ".").parse::<f64>().ok();
        raw.push((y as f64 + (m as f64 - 0.5) / 12.0, oulu, ssn));
    }
    let mut valid: Vec<f64> = raw.iter().filter_map(|r| r.1).collect();
    if valid.is_empty() {
        return Vec::new();
    }
    valid.sort_by(|a, b| a.total_cmp(b));
    let mean = valid[valid.len() / 2];
    raw.into_iter()
        .map(|(year, oulu, ssn)| SolarMonth {
            year,
            oulu: oulu.map(|v| (v / mean - 1.0) * 100.0),
            ssn,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rete_completa_e_ordinabile() {
        assert!(STATIONS.len() >= 40, "{}", STATIONS.len());
        for station in STATIONS {
            assert!(station.rc > 0.0 && station.rc < 18.0);
            assert!((-90.0..=90.0).contains(&station.lat));
            assert!((-180.0..=180.0).contains(&station.lon));
            assert_eq!(station.code.len(), 4);
        }
        let mut codes: Vec<&str> = STATIONS.iter().map(|s| s.code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), STATIONS.len());
    }

    #[test]
    fn variazione_centrata_su_zero() {
        assert_eq!(variation_color(0.0), (148, 163, 184));
        let cold = variation_color(-15.0);
        let hot = variation_color(15.0);
        assert!(cold.2 > cold.0);
        assert!(hot.0 > hot.2);
    }

    #[test]
    fn asset_live_valido() {
        let dataset = load_live().expect("live.bin");
        assert!(dataset.steps > 24, "{}", dataset.steps);
        assert!(dataset.ncol() >= 35, "{}", dataset.ncol());
        assert!(dataset.scale_pct >= 2.0 && dataset.scale_pct <= 12.0);
    }

    #[test]
    fn eventi_leggibili() {
        for event in EVENTS {
            let dataset = load_event(event).unwrap_or_else(|| panic!("{}", event.id));
            assert!(dataset.steps > 100, "{}", event.id);
            // la copertura storica varia molto: nel 1991 la rete era molto più piccola
            assert!(dataset.ncol() >= 7, "{} {}", event.id, dataset.ncol());
            let (_, min, max, count) = dataset.stats(dataset.steps / 2);
            assert!(count > 5, "{}", event.id);
            assert!(max >= min, "{}", event.id);
        }
    }

    #[test]
    fn ciclo_solare_dal_1964() {
        let months = solar_series();
        assert!(months.len() > 700, "{}", months.len());
        assert!((months[0].year - 1964.0).abs() < 1.0, "{}", months[0].year);
        assert!(months.iter().any(|m| m.oulu.map(|v| v > 2.0).unwrap_or(false)));
        assert!(months.iter().any(|m| m.ssn.map(|v| v > 100.0).unwrap_or(false)));
    }
}
