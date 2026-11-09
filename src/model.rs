//! Modello fisico semplificato: dose da raggi cosmici in funzione di quota e taglio
//! geomagnetico, più le etichette di severità per i parametri solari.

/// Stazione del network di neutron monitor (nome, paese, lat, lon, rigidità di taglio GV, quota m).
pub struct Station {
    pub name: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub rc: f64,
    pub altitude_m: f64,
}

pub const STATIONS: &[Station] = &[
    Station { name: "Oulu", country: "FI", lat: 65.05, lon: 25.47, rc: 0.81, altitude_m: 15.0 },
    Station { name: "Apatity", country: "RU", lat: 67.55, lon: 33.33, rc: 0.65, altitude_m: 177.0 },
    Station { name: "Barentsburg", country: "NO", lat: 78.07, lon: 14.23, rc: 0.10, altitude_m: 40.0 },
    Station { name: "Thule", country: "GL", lat: 76.50, lon: -68.70, rc: 0.10, altitude_m: 260.0 },
    Station { name: "Inuvik", country: "CA", lat: 68.35, lon: -133.72, rc: 0.30, altitude_m: 21.0 },
    Station { name: "Fort Smith", country: "CA", lat: 60.02, lon: -111.93, rc: 0.30, altitude_m: 200.0 },
    Station { name: "Mawson", country: "AQ", lat: -67.60, lon: 62.88, rc: 0.22, altitude_m: 30.0 },
    Station { name: "Syowa", country: "AQ", lat: -69.00, lon: 39.58, rc: 0.24, altitude_m: 20.0 },
    Station { name: "Terra Nova Bay", country: "AQ", lat: -74.70, lon: 164.10, rc: 0.10, altitude_m: 40.0 },
    Station { name: "McMurdo", country: "AQ", lat: -77.85, lon: 166.72, rc: 0.10, altitude_m: 45.0 },
    Station { name: "South Pole", country: "AQ", lat: -90.00, lon: 0.00, rc: 0.10, altitude_m: 2820.0 },
    Station { name: "Vostok", country: "AQ", lat: -78.47, lon: 106.83, rc: 0.10, altitude_m: 3488.0 },
    Station { name: "Kerguelen", country: "FR", lat: -49.35, lon: 70.25, rc: 1.14, altitude_m: 30.0 },
    Station { name: "Jungfraujoch", country: "CH", lat: 46.55, lon: 7.98, rc: 4.48, altitude_m: 3571.0 },
    Station { name: "Lomnicky Stit", country: "SK", lat: 49.20, lon: 20.22, rc: 3.84, altitude_m: 2634.0 },
    Station { name: "Climax", country: "US", lat: 39.37, lon: -106.18, rc: 2.97, altitude_m: 3400.0 },
    Station { name: "Newark", country: "US", lat: 39.68, lon: -75.75, rc: 2.40, altitude_m: 50.0 },
    Station { name: "Rome", country: "IT", lat: 41.90, lon: 12.52, rc: 6.20, altitude_m: 60.0 },
    Station { name: "Athens", country: "GR", lat: 37.97, lon: 23.72, rc: 8.53, altitude_m: 260.0 },
    Station { name: "Mexico City", country: "MX", lat: 19.33, lon: -99.18, rc: 8.90, altitude_m: 2274.0 },
    Station { name: "Huancayo", country: "PE", lat: -12.03, lon: -75.33, rc: 13.00, altitude_m: 3400.0 },
    Station { name: "Cape Schmidt", country: "RU", lat: 68.87, lon: 179.47, rc: 0.50, altitude_m: 30.0 },
    Station { name: "Yakutsk", country: "RU", lat: 62.02, lon: 129.72, rc: 1.65, altitude_m: 100.0 },
    Station { name: "Tixie Bay", country: "RU", lat: 71.60, lon: 128.90, rc: 0.53, altitude_m: 30.0 },
    Station { name: "Baksan", country: "RU", lat: 43.28, lon: 42.69, rc: 5.61, altitude_m: 1700.0 },
    Station { name: "Norilsk", country: "RU", lat: 69.00, lon: 88.00, rc: 0.63, altitude_m: 50.0 },
];

/// Quota dell'osservatore predefinita (Roma).
pub const HOME_LAT: f64 = 41.90;
pub const HOME_LON: f64 = 12.52;
pub const HOME_ALT_M: f64 = 60.0;

/// Quote selezionabili nella UI: (metri, etichetta).
pub const ALTITUDE_CHOICES: &[(f64, &str)] = &[
    (0.0, "Livello del mare · 0 m"),
    (100.0, "Collina · 100 m"),
    (500.0, "500 m"),
    (1000.0, "1000 m"),
    (2000.0, "2000 m · media montagna"),
    (3000.0, "3000 m · alta montagna"),
    (4200.0, "4200 m · Cervino"),
    (5500.0, "5500 m · campo base Everest"),
    (7000.0, "7000 m · quota di crociera alta"),
    (8500.0, "8500 m · jet tipico"),
    (10700.0, "10 700 m · crociera di linea"),
    (12000.0, "12 000 m · bizjet"),
    (15000.0, "15 000 m · massimo Pfotzer"),
    (18000.0, "18 000 m · alta stratosfera"),
    (21000.0, "21 000 m · Concorde"),
    (30000.0, "30 000 m · pallone stratosferico"),
    (40000.0, "40 000 m · bordo atmosfera"),
];

/// Profilo di dose equivalente da raggi cosmici (µSv/h) in condizioni di sole quieto.
/// Ancore da misure pubblicate (UNSCEAR, ICRU, campagne di volo); interpolazione log-lineare.
const DOSE_ANCHORS: &[(f64, f64)] = &[
    (0.0, 0.030),
    (500.0, 0.039),
    (1000.0, 0.052),
    (2000.0, 0.095),
    (3000.0, 0.170),
    (4000.0, 0.300),
    (5000.0, 0.490),
    (5500.0, 0.620),
    (7000.0, 1.050),
    (8500.0, 1.650),
    (10700.0, 2.700),
    (12000.0, 3.600),
    (15000.0, 4.700),
    (18000.0, 4.450),
    (21000.0, 3.600),
    (25000.0, 2.300),
    (30000.0, 1.150),
    (35000.0, 0.720),
    (40000.0, 0.450),
];

/// Dose equivalente da raggi cosmici (µSv/h) alla quota data, a sole quieto.
pub fn dose_ush(altitude_m: f64) -> f64 {
    let h = altitude_m.max(0.0);
    if let Some(last) = DOSE_ANCHORS.last() {
        if h >= last.0 {
            return last.1;
        }
    }
    for pair in DOSE_ANCHORS.windows(2) {
        let (h0, d0) = pair[0];
        let (h1, d1) = pair[1];
        if h >= h0 && h <= h1 {
            let t = (h - h0) / (h1 - h0);
            return (d0.ln() + t * (d1.ln() - d0.ln())).exp();
        }
    }
    DOSE_ANCHORS[0].1
}

/// Rigidità di taglio geomagnetico approssimata (dipolo) dalla latitudine.
pub fn cutoff_from_lat(lat_deg: f64) -> f64 {
    let c = lat_deg.to_radians().cos();
    (14.5 * c.powi(4)).clamp(0.10, 17.0)
}

/// Fattore geomagnetico normalizzato (1.0 = stazione polare tipo Oulu).
fn geomagnetic_factor(rc: f64) -> f64 {
    1.0 / (1.0 + 0.08 * rc.clamp(0.05, 20.0).powf(1.2))
}

/// Intensità relativa dei raggi cosmici al suolo (1.0 ≈ Oulu a sole quieto).
pub fn ground_intensity(rc: f64) -> f64 {
    geomagnetic_factor(rc) / geomagnetic_factor(0.81)
}

/// Intensità modellata a quota `altitude_m` e taglio `rc`, con la modulazione solare
/// osservata dal neutron monitor (`variation_pct`, in punti percentuali).
pub fn modeled_intensity(rc: f64, altitude_m: f64, variation_pct: f64) -> f64 {
    let alt = dose_ush(altitude_m) / dose_ush(0.0);
    let solar = 1.0 + (variation_pct / 100.0) * (1.0 + altitude_m / 6000.0);
    alt * ground_intensity(rc) * solar.clamp(0.5, 1.5)
}

/// Dose modellata alla quota scelta, applicando la modulazione solare corrente.
pub fn modeled_dose(altitude_m: f64, variation_pct: f64) -> f64 {
    let solar = 1.0 + (variation_pct / 100.0) * (1.0 + altitude_m / 6000.0);
    dose_ush(altitude_m) * solar.clamp(0.6, 1.4)
}

/// Pressione atmosferica ISA approssimata (hPa).
pub fn pressure_hpa(altitude_m: f64) -> f64 {
    let h = altitude_m.max(0.0);
    1013.25 * (1.0 - 2.25577e-5 * h).powf(5.25588)
}

/// Classificazione del flusso X-ray GOES (W/m²) in classe standard (A, B, C, M, X).
pub fn flare_class(flux: f64) -> String {
    let (letter, base) = if flux >= 1e-4 {
        ('X', 1e-4)
    } else if flux >= 1e-5 {
        ('M', 1e-5)
    } else if flux >= 1e-6 {
        ('C', 1e-6)
    } else if flux >= 1e-7 {
        ('B', 1e-7)
    } else {
        ('A', 1e-8)
    };
    format!("{letter}{:.1}", flux / base)
}

/// Descrizione sintetica del disturbo geomagnetico (scala NOAA G).
pub fn kp_storm(kp: f64) -> &'static str {
    match kp {
        k if k < 5.0 => "quiete",
        k if k < 6.0 => "tempesta G1",
        k if k < 7.0 => "tempesta G2",
        k if k < 8.0 => "tempesta G3",
        k if k < 9.0 => "tempesta G4",
        _ => "tempesta G5",
    }
}

/// Colore associato al livello Kp.
pub fn kp_color(kp: f64) -> (u8, u8, u8) {
    match kp {
        k if k < 4.0 => (56, 189, 248),
        k if k < 5.0 => (52, 211, 153),
        k if k < 6.0 => (250, 204, 21),
        k if k < 7.0 => (251, 146, 60),
        k if k < 8.0 => (248, 113, 113),
        _ => (244, 63, 94),
    }
}

/// Scala colore continua per la mappa di intensità (0.25 → 1.15).
pub fn intensity_color(value: f64) -> (u8, u8, u8, u8) {
    const STOPS: &[(f64, (u8, u8, u8))] = &[
        (0.20, (14, 32, 61)),
        (0.42, (13, 92, 120)),
        (0.62, (34, 197, 220)),
        (0.80, (52, 211, 153)),
        (0.95, (251, 191, 36)),
        (1.10, (248, 113, 60)),
    ];
    let v = value.clamp(STOPS[0].0, STOPS[STOPS.len() - 1].0);
    for pair in STOPS.windows(2) {
        let (v0, c0) = pair[0];
        let (v1, c1) = pair[1];
        if v >= v0 && v <= v1 {
            let t = (v - v0) / (v1 - v0);
            let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
            return (mix(c0.0, c1.0), mix(c0.1, c1.1), mix(c0.2, c1.2), 200);
        }
    }
    (14, 32, 61, 200)
}
