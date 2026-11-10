//! Modello fisico semplificato: dose da raggi cosmici in funzione di quota, taglio
//! geomagnetico e modulazione solare, più le etichette di severità per i parametri solari.

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
    Station { name: "South Pole", country: "AQ", lat: -89.5, lon: 0.00, rc: 0.10, altitude_m: 2820.0 },
    Station { name: "Vostok", country: "AQ", lat: -78.47, lon: 106.83, rc: 0.10, altitude_m: 3488.0 },
    Station { name: "Kerguelen", country: "FR", lat: -49.35, lon: 70.25, rc: 1.14, altitude_m: 30.0 },
    Station { name: "Jungfraujoch", country: "CH", lat: 46.55, lon: 7.98, rc: 4.48, altitude_m: 3571.0 },
    Station { name: "Lomnicky Stit", country: "SK", lat: 49.20, lon: 20.22, rc: 3.84, altitude_m: 2634.0 },
    Station { name: "Climax", country: "US", lat: 39.37, lon: -106.18, rc: 2.97, altitude_m: 3400.0 },
    Station { name: "Newark", country: "US", lat: 39.68, lon: -75.75, rc: 2.40, altitude_m: 50.0 },
    Station { name: "Roma", country: "IT", lat: 41.86, lon: 12.47, rc: 6.20, altitude_m: 60.0 },
    Station { name: "Atene", country: "GR", lat: 37.97, lon: 23.72, rc: 8.53, altitude_m: 260.0 },
    Station { name: "Città del Messico", country: "MX", lat: 19.33, lon: -99.18, rc: 8.90, altitude_m: 2274.0 },
    Station { name: "Huancayo", country: "PE", lat: -12.03, lon: -75.33, rc: 13.00, altitude_m: 3400.0 },
    Station { name: "Cape Schmidt", country: "RU", lat: 68.87, lon: 179.47, rc: 0.50, altitude_m: 30.0 },
    Station { name: "Yakutsk", country: "RU", lat: 62.02, lon: 129.72, rc: 1.65, altitude_m: 100.0 },
    Station { name: "Tixie Bay", country: "RU", lat: 71.60, lon: 128.90, rc: 0.53, altitude_m: 30.0 },
    Station { name: "Baksan", country: "RU", lat: 43.28, lon: 42.69, rc: 5.61, altitude_m: 1700.0 },
    Station { name: "Norilsk", country: "RU", lat: 69.00, lon: 88.00, rc: 0.63, altitude_m: 50.0 },
];

// ---------------------------------------------------------------------------
// Riferimenti fissi dei neutron monitor (conteggi/s corretti per efficienza, NMDB).
// Calcolati il 10/11/2026 dalle medie mensili NMDB 2008-2026.
// ---------------------------------------------------------------------------

/// Oulu: massimo mensile al minimo solare (dicembre 2009, record dell'era spaziale).
pub const OULU_SOLAR_MIN: f64 = 114.07;
/// Oulu: media 2008-2025 (circa un ciclo solare e mezzo).
pub const OULU_LONG_MEAN: f64 = 106.65;
/// Roma: massimo mensile al minimo solare (dicembre 2020). Prima di agosto 2017 la
/// serie ha un salto strumentale, quindi il riferimento usa solo i dati successivi.
pub const ROME_SOLAR_MIN: f64 = 129.16;
/// Roma: media 2018-2025.
#[allow(dead_code)] // riferimento documentato, utile per confronti futuri
pub const ROME_LONG_MEAN: f64 = 124.46;

// ---------------------------------------------------------------------------
// Località selezionabili (dose al suolo / in quota e cerchio sulla mappa).
// Rc ≈ rigidità di taglio verticale stimata dalle mappe IGRF e dai monitor vicini.
// ---------------------------------------------------------------------------

pub struct Place {
    pub name: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: f64,
    pub rc: f64,
}

pub const PLACES: &[Place] = &[
    Place { name: "Rive d'Arcano (UD)", lat: 46.13, lon: 13.03, altitude_m: 180.0, rc: 4.6 },
    Place { name: "Udine", lat: 46.06, lon: 13.24, altitude_m: 113.0, rc: 4.6 },
    Place { name: "Milano", lat: 45.46, lon: 9.19, altitude_m: 120.0, rc: 4.9 },
    Place { name: "Roma", lat: 41.90, lon: 12.50, altitude_m: 20.0, rc: 6.2 },
    Place { name: "Palermo", lat: 38.12, lon: 13.36, altitude_m: 14.0, rc: 7.6 },
    Place { name: "Oulu (FI) · polare", lat: 65.05, lon: 25.47, altitude_m: 15.0, rc: 0.81 },
    Place { name: "Singapore · equatoriale", lat: 1.35, lon: 103.82, altitude_m: 15.0, rc: 14.0 },
];

/// Rigidità di taglio usate per le curve di confronto nel profilo verticale.
pub const RC_POLAR: f64 = 0.8;
pub const RC_EQUATOR: f64 = 14.0;

/// Quote selezionabili nella UI: (metri, etichetta).
pub const ALTITUDE_CHOICES: &[(f64, &str)] = &[
    (0.0, "Livello del mare · 0 m"),
    (180.0, "Pianura friulana · 180 m"),
    (500.0, "500 m"),
    (1000.0, "1000 m"),
    (2000.0, "2000 m · media montagna"),
    (3000.0, "3000 m · alta montagna"),
    (4200.0, "4200 m · Cervino"),
    (5500.0, "5500 m · campo base Everest"),
    (7000.0, "7000 m"),
    (8500.0, "8500 m · Everest"),
    (10700.0, "10 700 m · crociera di linea"),
    (12000.0, "12 000 m · bizjet"),
    (15000.0, "15 000 m · massimo Pfotzer"),
    (18000.0, "18 000 m · alta stratosfera"),
    (21000.0, "21 000 m · Concorde"),
    (30000.0, "30 000 m · pallone stratosferico"),
    (40000.0, "40 000 m · bordo atmosfera"),
];

/// Profilo di dose equivalente da raggi cosmici (µSv/h) per medie latitudini europee
/// (Rc ≈ 5 GV) e attività solare media. Ancore da misure pubblicate (UNSCEAR, ICRU,
/// campagne di volo); interpolazione log-lineare.
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
const ANCHOR_RC: f64 = 5.0;

/// Dose (µSv/h) delle ancore alla quota data: Rc ≈ 5 GV, attività solare media.
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

/// Effetto latitudine sulla dose: lo schermo geomagnetico pesa poco al suolo
/// (polo/equatore ≈ 1,25) e molto in quota (≈ 3 a 11 km, ≈ 4 oltre i 18 km).
/// Sotto ~0,8 GV il taglio è atmosferico, non magnetico: la curva si appiattisce.
fn latitude_factor(altitude_m: f64, rc: f64) -> f64 {
    let a = (altitude_m.max(0.0) / 18_000.0).min(1.0);
    let c = 0.02 + 0.11 * a.powf(0.8);
    (-c * (rc - 0.8).max(0.0).powf(0.9)).exp()
}

/// Sensibilità alla modulazione solare rispetto a Oulu: a taglio alto arrivano solo
/// particelle energetiche, meno modulate (Roma oscilla circa metà di Oulu nel ciclo).
pub fn solar_sensitivity(rc: f64) -> f64 {
    (1.0 + 0.18 * 0.81) / (1.0 + 0.18 * rc.max(0.0))
}

/// Fattore solare: `variation_pct` è la variazione di Oulu rispetto alla media di lungo
/// periodo; l'ampiezza cresce con la quota (lo sciame primario è meno filtrato).
fn solar_factor(altitude_m: f64, rc: f64, variation_pct: f64) -> f64 {
    let gain = (1.0 + altitude_m.max(0.0) / 12_000.0).min(2.5);
    (1.0 + variation_pct / 100.0 * gain * solar_sensitivity(rc)).clamp(0.5, 1.6)
}

/// Dose modellata (µSv/h) a quota, taglio e modulazione solare dati.
pub fn modeled_dose(altitude_m: f64, rc: f64, variation_pct: f64) -> f64 {
    dose_ush(altitude_m) * latitude_factor(altitude_m, rc) / latitude_factor(altitude_m, ANCHOR_RC)
        * solar_factor(altitude_m, rc, variation_pct)
}

// ---------------------------------------------------------------------------
// Geomagnetismo: dipolo inclinato (polo geomagnetico IGRF-14, epoca 2025).
// ---------------------------------------------------------------------------

const GM_POLE_LAT: f64 = 80.85;
const GM_POLE_LON: f64 = -72.68;

/// Latitudine geomagnetica (gradi) nel modello a dipolo inclinato.
pub fn geomagnetic_lat(lat_deg: f64, lon_deg: f64) -> f64 {
    let (p, l) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (pp, lp) = (GM_POLE_LAT.to_radians(), GM_POLE_LON.to_radians());
    let s = p.sin() * pp.sin() + p.cos() * pp.cos() * (l - lp).cos();
    s.clamp(-1.0, 1.0).asin().to_degrees()
}

/// Rigidità di taglio verticale di Störmer (GV) dal dipolo inclinato.
pub fn cutoff_dipole(lat_deg: f64, lon_deg: f64) -> f64 {
    let c = geomagnetic_lat(lat_deg, lon_deg).to_radians().cos();
    (14.5 * c.powi(4)).clamp(0.05, 17.0)
}

/// Fattore geomagnetico del conteggio di un neutron monitor (1.0 ≈ polare).
fn monitor_factor(rc: f64) -> f64 {
    1.0 / (1.0 + 0.08 * rc.clamp(0.05, 20.0).powf(1.2))
}

/// Intensità relativa dei raggi cosmici al suolo (1.0 ≈ Oulu a modulazione media),
/// con la modulazione solare osservata scalata per il taglio locale.
pub fn ground_intensity(rc: f64, variation_pct: f64) -> f64 {
    let solar = 1.0 + variation_pct / 100.0 * solar_sensitivity(rc);
    monitor_factor(rc) / monitor_factor(0.81) * solar.clamp(0.5, 1.5)
}

/// Pressione atmosferica ISA approssimata (hPa).
pub fn pressure_hpa(altitude_m: f64) -> f64 {
    let h = altitude_m.max(0.0).min(11_000.0);
    let base = 1013.25 * (1.0 - 2.25577e-5 * h).powf(5.25588);
    if altitude_m <= 11_000.0 {
        base
    } else {
        // stratosfera isoterma (216,65 K)
        base * (-(altitude_m - 11_000.0) / 6341.6).exp()
    }
}

// ---------------------------------------------------------------------------
// Classificazioni.
// ---------------------------------------------------------------------------

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

/// Scala NOAA G (0-5) dall'indice Kp.
pub fn g_scale(kp: f64) -> u8 {
    match kp {
        k if k >= 9.0 => 5,
        k if k >= 8.0 => 4,
        k if k >= 7.0 => 3,
        k if k >= 6.0 => 2,
        k if k >= 5.0 => 1,
        _ => 0,
    }
}

/// Scala NOAA S (0-5) dal flusso di protoni ≥10 MeV (pfu).
pub fn s_scale(pfu: f64) -> u8 {
    match pfu {
        p if p >= 1e5 => 5,
        p if p >= 1e4 => 4,
        p if p >= 1e3 => 3,
        p if p >= 1e2 => 2,
        p if p >= 10.0 => 1,
        _ => 0,
    }
}

/// Scala NOAA R (0-5) dal flusso X-ray 0,1-0,8 nm (W/m²).
pub fn r_scale(flux: f64) -> u8 {
    match flux {
        f if f >= 2e-3 => 5,
        f if f >= 1e-3 => 4,
        f if f >= 1e-4 => 3,
        f if f >= 5e-5 => 2,
        f if f >= 1e-5 => 1,
        _ => 0,
    }
}

/// Descrizione sintetica del disturbo geomagnetico (scala NOAA G).
pub fn kp_storm(kp: f64) -> &'static str {
    match g_scale(kp) {
        0 if kp >= 4.0 => "attivo",
        0 => "quiete",
        1 => "tempesta G1 · minore",
        2 => "tempesta G2 · moderata",
        3 => "tempesta G3 · forte",
        4 => "tempesta G4 · severa",
        _ => "tempesta G5 · estrema",
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

/// Colore per un livello di scala NOAA (0-5).
pub fn scale_color(level: u8) -> (u8, u8, u8) {
    match level {
        0 => (52, 211, 153),
        1 => (250, 204, 21),
        2 => (251, 146, 60),
        3 => (248, 113, 113),
        _ => (244, 63, 94),
    }
}

/// Scala colore continua per la mappa di intensità (0.20 → 1.10).
pub fn intensity_color(value: f64) -> (u8, u8, u8) {
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
            return (mix(c0.0, c1.0), mix(c0.1, c1.1), mix(c0.2, c1.2));
        }
    }
    STOPS[0].1
}

/// Immagine BMP (24 bit) della mappa di intensità al suolo, `w`×`h` celle
/// equirettangolari, già fusa con il colore di sfondo `bg` (opacità 200/255).
pub fn intensity_bmp(w: usize, h: usize, variation_pct: f64, bg: (u8, u8, u8)) -> Vec<u8> {
    let row_bytes = (w * 3 + 3) & !3;
    let data_len = row_bytes * h;
    let mut out = Vec::with_capacity(54 + data_len);
    let le32 = |v: u32| v.to_le_bytes();
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&le32((54 + data_len) as u32));
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&le32(54));
    out.extend_from_slice(&le32(40));
    out.extend_from_slice(&le32(w as u32));
    out.extend_from_slice(&le32(h as u32)); // righe dal basso verso l'alto
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    let alpha = 200.0 / 255.0;
    let blend = |c: u8, b: u8| (c as f64 * alpha + b as f64 * (1.0 - alpha)).round() as u8;
    for row in (0..h).rev() {
        let lat = 90.0 - (row as f64 + 0.5) * 180.0 / h as f64;
        let start = out.len();
        for col in 0..w {
            let lon = -180.0 + (col as f64 + 0.5) * 360.0 / w as f64;
            let value = ground_intensity(cutoff_dipole(lat, lon), variation_pct);
            let (r, g, b) = intensity_color(value);
            out.extend_from_slice(&[blend(b, bg.2), blend(g, bg.1), blend(r, bg.0)]);
        }
        while out.len() - start < row_bytes {
            out.push(0);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ancore_invariate_a_rc_di_riferimento() {
        for alt in [0.0, 10_700.0, 15_000.0] {
            let d = modeled_dose(alt, ANCHOR_RC, 0.0);
            assert!((d - dose_ush(alt)).abs() < 1e-9);
        }
    }

    #[test]
    fn effetto_latitudine_plausibile() {
        let ratio_ground = modeled_dose(0.0, RC_POLAR, 0.0) / modeled_dose(0.0, RC_EQUATOR, 0.0);
        let ratio_cruise =
            modeled_dose(10_700.0, RC_POLAR, 0.0) / modeled_dose(10_700.0, RC_EQUATOR, 0.0);
        assert!(ratio_ground > 1.1 && ratio_ground < 1.5, "{ratio_ground}");
        assert!(ratio_cruise > 2.2 && ratio_cruise < 3.5, "{ratio_cruise}");
    }

    #[test]
    fn dipolo_inclinato() {
        // Roma ≈ 42° geomagnetici, polo geomagnetico a 90°
        let rome = geomagnetic_lat(41.9, 12.5);
        assert!((rome - 42.0).abs() < 1.5, "{rome}");
        assert!((geomagnetic_lat(GM_POLE_LAT, GM_POLE_LON) - 90.0).abs() < 1e-6);
    }

    #[test]
    fn scale_noaa() {
        assert_eq!(r_scale(1.2e-5), 1);
        assert_eq!(r_scale(2.0e-4), 3);
        assert_eq!(s_scale(12.0), 1);
        assert_eq!(g_scale(5.33), 1);
        assert_eq!(flare_class(2.3e-6), "C2.3");
    }

    #[test]
    fn bmp_dimensioni() {
        let bmp = intensity_bmp(36, 18, 0.0, (0, 0, 0));
        assert_eq!(bmp.len(), 54 + 36 * 3 * 18);
    }
}
