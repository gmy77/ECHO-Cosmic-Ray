#!/usr/bin/env python3
"""Costruisce gli asset dell'Atlante dei raggi cosmici.

Scarica da NMDB i conteggi dei neutron monitor (corretti per efficienza e
pressione), li impacchetta in un formato binario compatto e salva anche la
serie mensile di Oulu e il numero di macchie solari SILSO.

NMDB a volte filtra gli indirizzi che fanno molte richieste: lo script prova
prima la connessione diretta e, se non risponde, passa dal reader r.jina.ai.

Uso:  python tools/atlas_build.py [--no-oulu]
"""

from __future__ import annotations

import datetime as dt
import json
import math
import os
import re
import struct
import sys
import time
import urllib.parse
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "assets", "atlas")

# code, nome, paese, lat, lon, rigidità di taglio GV, quota m
STATIONS = [
    ("APTY", "Apatity", "RU", 67.57, 33.40, 0.65, 181),
    ("OULU", "Oulu", "FI", 65.05, 25.47, 0.81, 15),
    ("INVK", "Inuvik", "CA", 68.36, -133.72, 0.30, 21),
    ("FSMT", "Fort Smith", "CA", 60.02, -111.93, 0.30, 180),
    ("THUL", "Thule", "GL", 76.60, -68.80, 0.30, 26),
    ("NAIN", "Nain", "CA", 56.55, -61.68, 0.30, 46),
    ("PWNK", "Peawanuck", "CA", 55.02, -85.53, 0.30, 53),
    ("JBGO", "Jang Bogo", "AQ", -74.62, 164.22, 0.30, 29),
    ("MCMU", "McMurdo", "AQ", -77.85, 166.72, 0.30, 48),
    ("SOPO", "South Pole", "AQ", -90.00, 0.00, 0.10, 2820),
    ("DOMC", "Dome C", "AQ", -75.10, 123.35, 0.01, 3233),
    ("TERA", "Terre Adélie", "AQ", -66.67, 140.02, 0.01, 32),
    ("MRNY", "Mirny", "AQ", -66.55, 93.02, 0.03, 30),
    ("MWSN", "Mawson", "AQ", -67.60, 62.88, 0.22, 30),
    ("NEU3", "Neumayer III", "AQ", -70.65, -8.25, 0.10, 40),
    ("SNAE", "SANAE IV", "AQ", -71.68, -2.85, 0.73, 856),
    ("KERG", "Kerguelen", "FR", -49.35, 70.25, 1.14, 33),
    ("CALG", "Calgary", "CA", 51.08, -114.13, 1.08, 1123),
    ("YKTK", "Yakutsk", "RU", 62.02, 129.72, 1.65, 105),
    ("IRKT", "Irkutsk", "RU", 52.47, 104.03, 3.64, 435),
    ("NVBK", "Novosibirsk", "RU", 54.85, 83.10, 2.91, 163),
    ("MGDN", "Magadan", "RU", 60.05, 150.80, 2.10, 220),
    ("MOSC", "Moscow", "RU", 55.47, 37.32, 2.43, 200),
    ("KIEL", "Kiel", "DE", 54.34, 10.12, 2.36, 54),
    ("DRBS", "Dourbes", "BE", 50.10, 4.60, 3.18, 225),
    ("LMKS", "Lomnický štít", "SK", 49.20, 20.22, 3.84, 2634),
    ("JUNG", "Jungfraujoch", "CH", 46.55, 7.98, 4.49, 3575),
    ("ZUGS", "Zugspitze", "DE", 47.42, 10.98, 4.24, 2960),
    ("CLMX", "Climax", "US", 39.37, -106.18, 3.00, 3400),
    ("NEWK", "Newark", "US", 39.68, -75.75, 2.40, 50),
    ("HRMS", "Hermanus", "ZA", -34.42, 19.22, 4.58, 26),
    ("PTFM", "Potchefstroom", "ZA", -26.70, 27.10, 6.98, 1351),
    ("TSMB", "Tsumeb", "NA", -19.20, 17.70, 9.15, 1240),
    ("ATHN", "Athens", "GR", 37.97, 23.72, 8.53, 260),
    ("ROME", "Rome", "IT", 41.86, 12.47, 6.27, 0),
    ("BKSN", "Baksan", "RU", 43.28, 42.69, 5.70, 1700),
    ("MXCO", "Mexico City", "MX", 19.33, -99.18, 8.28, 2274),
    ("HUAN", "Huancayo", "PE", -12.03, -75.33, 12.92, 3400),
    ("CHAC", "Chacaltaya", "BO", -16.35, -68.13, 11.80, 5270),
    ("AHMD", "Ahmedabad", "IN", 23.02, 72.60, 15.94, 50),
    ("PSNM", "Doi Inthanon", "TH", 18.59, 98.49, 16.80, 2565),
    ("KGSN", "Kingston", "AU", -42.98, 147.29, 1.88, 65),
]

CODES = [s[0] for s in STATIONS]
STATION_SET = set(CODES)
CODE_INDEX = {c: i for i, c in enumerate(CODES)}

BASE = "https://www.nmdb.eu/nest/draw_graph.php"

# id, tipo, etichetta, data, descrizione, inizio, fine
EVENTS = [
    ("gle66", "GLE", "GLE 66 · Halloween", "29 ottobre 2003",
     "La sequenza di brillamenti e tempeste del 28-30 ottobre 2003: il GLE più intenso "
     "e meglio osservato dell'era moderna, con aumenti al suolo fino a decine di punti "
     "percentuali alle stazioni polari.",
     "2003-10-28", "2003-10-31", "event_gle66.bin"),
    ("gle69", "GLE", "GLE 69 · gennaio 2005", "20 gennaio 2005",
     "Il brillamento X7 del 20 gennaio 2005 accelera protoni fino a GeV: le stazioni "
     "polari vedono un aumento improvviso, quelle a rigidità alta quasi nulla. È il "
     "ritratto del taglio geomagnetico in azione.",
     "2005-01-19", "2005-01-22", "event_gle69.bin"),
    ("gle71", "GLE", "GLE 71 · maggio 2012", "17 maggio 2012",
     "Un GLE associato a una tempesta geomagnetica in corso: l'aumento arriva mentre "
     "il campo magnetico è già disturbato, e la mappa mostra una risposta irregolare.",
     "2012-05-16", "2012-05-19", "event_gle71.bin"),
    ("gle74", "GLE", "GLE 74 · maggio 2024", "11 maggio 2024",
     "Il GLE della grande tempesta G5 del maggio 2024, osservato dalla rete attuale: "
     "l'evento più recente e quello con più stazioni simultanee.",
     "2024-05-09", "2024-05-12", "event_may2024.bin"),
    ("fd1991", "FD", "Forbush · giugno 1991", "12 giugno 1991",
     "Uno dei cali di Forbush più profondi registrati: oltre il 20 % in alcune stazioni. "
     "L'onda di depressione si propaga dal polo verso l'equatore in poche ore.",
     "1991-06-11", "1991-06-14", "event_fd1991.bin"),
    ("fd2003", "FD", "Forbush · novembre 2003", "20 novembre 2003",
     "Il calo del 20 novembre 2003 segue la grande tempesta di Halloween: la mappa "
     "mostra quanto la depressione sia globale e quanto dipenda dalla rigidità.",
     "2003-11-19", "2003-11-22", "event_fd2003.bin"),
    ("fd2015", "FD", "Forbush · giugno 2015", "22 giugno 2015",
     "Un calo di Forbush da tempesta geomagnetica moderata, utile per vedere la "
     "propagazione senza gli effetti estremi dei grandi eventi.",
     "2015-06-21", "2015-06-24", "event_fd2015.bin"),
    ("fd2024", "FD", "Forbush · maggio 2024", "10 maggio 2024",
     "La depressione che accompagna la tempesta G5 del maggio 2024, vista dalla rete "
     "completa: prima il calo, poi il GLE del giorno dopo.",
     "2024-05-09", "2024-05-12", "event_may2024.bin"),
    ("fd2023", "FD", "Forbush · aprile 2023", "23 aprile 2023",
     "Il calo di Forbush più profondo del ciclo solare 25: la tempesta geomagnetica G4 "
     "spazza via una parte dei raggi cosmici e la rete attuale la vede tutta insieme.",
     "2023-04-22", "2023-04-25", "event_fd2023.bin"),
]

UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AtlasCosmicRays/0.3"


def http_get(url: str, timeout: int = 90) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=timeout) as response:
        raw = response.read()
    return raw.decode("utf-8", "replace")


def nmdb(codes: list[str], start: str, end: str, resolution: int,
         table: str = "ori", last_days: int | None = None) -> str:
    params = [("stations[]", c) for c in codes]
    params += [
        ("output", "ascii"),
        ("tabchoice", table),
        ("dtype", "corr_for_efficiency"),
        ("tresolution", str(resolution)),
        ("yunits", "0"),
        ("wget", "1"),
    ]
    if last_days is None:
        s = dt.datetime.fromisoformat(start)
        e = dt.datetime.fromisoformat(end)
        params += [
            ("date_choice", "bydate"),
            ("start_year", str(s.year)), ("start_month", str(s.month)),
            ("start_day", str(s.day)), ("start_hour", "0"), ("start_min", "0"),
            ("end_year", str(e.year)), ("end_month", str(e.month)),
            ("end_day", str(e.day)), ("end_hour", "0"), ("end_min", "0"),
        ]
    else:
        params += [
            ("date_choice", "last"), ("last_days", str(last_days)),
            ("last_label", "days_label"),
        ]
    url = BASE + "?" + urllib.parse.urlencode(params, safe="[]")
    try:
        text = http_get(url)
        if "# STATION" in text or "start_date_time" in text:
            print(f"    diretto: {len(text) // 1024} kB", flush=True)
            return text
        print("    risposta inattesa dal diretto, provo il reader", flush=True)
    except Exception as error:  # noqa: BLE001
        print(f"    diretto non raggiungibile ({error}), uso il reader", flush=True)
    time.sleep(1.5)
    proxied = "https://r.jina.ai/" + url
    req = urllib.request.Request(proxied, headers={
        "User-Agent": UA,
        "X-Return-Format": "text",
        "Accept": "text/plain",
    })
    with urllib.request.urlopen(req, timeout=180) as response:
        text = response.read().decode("utf-8", "replace")
    print(f"    reader: {len(text) // 1024} kB", flush=True)
    return text


STAMP = re.compile(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2});")


def parse_table(text: str) -> dict[str, dict[str, float | None]]:
    """Ritorna {codice: {timestamp: valore}} da una risposta ASCII NMDB."""
    text = text.replace("\r", "")
    # il reader appiattisce le righe: separa i timestamp
    matches = list(STAMP.finditer(text))
    if not matches:
        raise ValueError("nessuna riga dati nella risposta")

    # l'intestazione delle colonne è l'ultima riga prima del primo timestamp
    prefix = text[: matches[0].start()]
    station_line = ""
    for line in prefix.replace(" # ", "\n").splitlines():
        if "STATION:" in line:
            station_line = line.split("STATION:", 1)[1]
            station_line = station_line.split("#", 1)[0].strip()
    head = ""
    for line in prefix.replace(" # ", "\n").splitlines():
        line = line.strip().lstrip("# ").strip()
        tokens = [t for t in re.split(r"[\s,]+", line) if t]
        if tokens and all(t in STATION_SET for t in tokens):
            head = " ".join(tokens)
    columns = head.split()
    if not columns:
        # una sola stazione: NMDB intestata la colonna col tipo di dato
        fallback = [c.strip() for c in station_line.split(",") if c.strip()]
        if len(fallback) != 1:
            raise ValueError("intestazione delle stazioni non trovata")
        columns = fallback

    out: dict[str, dict[str, float | None]] = {c: {} for c in columns}
    for index, match in enumerate(matches):
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        row = text[match.start():end].strip()
        # toglie eventuali separatori del reader rimasti in coda
        row = row.split(" # ")[0].strip()
        parts = [p.strip() for p in row.split(";")]
        stamp = parts[0][:19]
        values = parts[1:]
        if len(values) < len(columns):
            continue
        for code, value in zip(columns, values):
            if code not in out:
                continue
            cleaned = value.strip()
            if not cleaned or cleaned.lower() == "null":
                out[code][stamp] = None
            else:
                try:
                    number = float(cleaned)
                except ValueError:
                    continue
                if math.isfinite(number) and number > 0.0:
                    out[code][stamp] = number
    return out


def robust_series(values: dict[str, float | None]) -> tuple[list[str], list[float | None]]:
    """Ordina i timestamp e sostituisce i picchi isolati con la mediana locale."""
    stamps = sorted(values)
    series: list[float | None] = [values[s] for s in stamps]
    for i in range(2, len(series) - 2):
        if series[i] is None:
            continue
        neighbours = [series[j] for j in range(i - 2, i + 3)
                      if j != i and series[j] is not None]
        if len(neighbours) < 3:
            continue
        neighbours.sort()
        median = neighbours[len(neighbours) // 2]
        if median > 0 and (series[i] > median * 4.0 or series[i] < median / 4.0):
            series[i] = median
    return stamps, series


def write_grid(path: str, t0: int, step: int, columns: list[str],
               rows: list[list[float | None]]) -> None:
    """Formato: magic, versione, stazioni, passi, t0, passo, riservato, codici, f32."""
    nst = len(columns)
    nsteps = len(rows)
    with open(path, "wb") as fh:
        fh.write(b"NMDA")
        fh.write(struct.pack("<HHIqII", 1, nst, nsteps, t0, step, 0))
        for code in columns:
            fh.write(code.encode("ascii")[:4].ljust(4, b" "))
        for row in rows:
            for value in row:
                fh.write(struct.pack("<f", math.nan if value is None else value))
    print(f"  -> {os.path.relpath(path, ROOT)}  {os.path.getsize(path) // 1024} kB "
          f"({nst} stazioni × {nsteps} passi)", flush=True)


def epoch(text: str) -> int:
    return int(dt.datetime.fromisoformat(text).replace(tzinfo=dt.timezone.utc).timestamp())


def iso(ts: int) -> str:
    return dt.datetime.fromtimestamp(ts, dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%S")


def build_live() -> None:
    print("Serie live: ultimi 7 giorni, passo 1 ora", flush=True)
    merged: dict[str, dict[str, float | None]] = {c: {} for c in CODES}
    for batch in chunk(CODES, 14):
        table = fetch_batch(batch, "", "", 60, last_days=7)
        for code, series in table.items():
            merged.setdefault(code, {}).update(series)
        time.sleep(4.0)
    stamps = sorted({s for series in merged.values() for s in series})
    if not stamps:
        raise SystemExit("nessun dato live")
    # griglia oraria completa, allineata all'ora
    t0 = epoch(stamps[0])
    t1 = epoch(stamps[-1])
    step = 3600
    grid = [(t0 + i * step) for i in range((t1 - t0) // step + 1)]
    rows = []
    for ts in grid:
        key = iso(ts)
        rows.append([merged[c].get(key) for c in CODES])
    write_grid(os.path.join(ASSETS, "live.bin"), t0, step, CODES, rows)


def fetch_batch(codes: list[str], start: str, end: str, resolution: int,
                last_days: int | None = None) -> dict[str, dict[str, float | None]]:
    last_error: Exception | None = None
    for attempt in range(3):
        try:
            text = nmdb(codes, start, end, resolution, last_days=last_days)
            return parse_table(text)
        except Exception as error:  # noqa: BLE001
            last_error = error
            print(f"    tentativo {attempt + 1} fallito: {error}", flush=True)
            time.sleep(12.0 + attempt * 12.0)
    print(f"    lotto saltato: {last_error}", flush=True)
    return {}


def build_event(event: tuple) -> None:
    ident, kind, label, date_label, blurb, start, end, filename = event
    print(f"Evento {label} ({start} -> {end}), passo 10 min", flush=True)
    merged: dict[str, dict[str, float | None]] = {c: {} for c in CODES}
    for batch in chunk(CODES, 14):
        table = fetch_batch(batch, start, end, 10)
        for code, series in table.items():
            merged.setdefault(code, {}).update(series)
        time.sleep(4.0)
    if not any(merged.values()):
        raise SystemExit(f"nessun dato per {ident}")
    t0 = epoch(start + " 00:00:00")
    t1 = epoch(end + " 00:00:00")
    step = 600
    grid = list(range(t0, t1, step))
    rows = []
    for ts in grid:
        key = iso(ts)
        rows.append([merged[c].get(key) for c in CODES])
    path = os.path.join(ASSETS, "events", filename)
    write_grid(path, t0, step, CODES, rows)


def build_solar() -> None:
    print("Ciclo solare: Oulu mensile dal 1964", flush=True)
    text = nmdb(["OULU"], "1964-01-01", "2026-12-31", 43200, table="1h")
    table = parse_table(text)
    oulu = table.get("OULU", {})
    print(f"  {len(oulu)} mesi Oulu", flush=True)

    print("Macchie solari: SILSO mensile", flush=True)
    ssn_text = http_get("https://www.sidc.be/SILSO/DATA/SN_ms_tot_V2.0.csv")
    ssn: dict[str, float | None] = {}
    for line in ssn_text.replace("\r", "").splitlines():
        parts = line.split(";")
        if len(parts) < 5 or not parts[0].strip().isdigit():
            continue
        year, month, value = int(parts[0]), int(parts[1]), parts[3].strip()
        try:
            number = float(value)
        except ValueError:
            continue
        ssn[f"{year:04d}-{month:02d}"] = number if number >= 0 else None

    months = [f"{y:04d}-{m:02d}" for y in range(1964, 2027) for m in range(1, 13)]
    months = [m for m in months if m <= "2026-11"]
    path = os.path.join(ASSETS, "solar.csv")
    with open(path, "w", encoding="utf-8", newline="\n") as fh:
        fh.write("month;oulu;ssn\n")
        for month in months:
            o = oulu.get(month + "-01 00:00:00")
            # NMDB restituisce il timestamp d'inizio del mese
            if o is None:
                for key, value in oulu.items():
                    if key.startswith(month):
                        o = value
                        break
            s = ssn.get(month)
            fh.write(f"{month};{'' if o is None else f'{o:.3f}'};"
                     f"{'' if s is None else f'{s:.1f}'}\n")
    print(f"  -> {os.path.relpath(path, ROOT)}  {os.path.getsize(path) // 1024} kB",
          flush=True)


def chunk(items: list[str], size: int):
    for i in range(0, len(items), size):
        yield items[i:i + size]


def main() -> None:
    os.makedirs(os.path.join(ASSETS, "events"), exist_ok=True)
    if "--no-solar" not in sys.argv:
        try:
            build_solar()
        except Exception as error:  # noqa: BLE001
            print(f"  ciclo solare non scaricato: {error}", flush=True)
    only = [i for a in sys.argv if a.startswith("--only=")
            for i in a.split("=", 1)[1].split(",") if i]
    skip = [i for a in sys.argv if a.startswith("--skip=")
            for i in a.split("=", 1)[1].split(",") if i]
    for event in EVENTS:
        if only and event[0] not in only:
            continue
        if event[0] in skip:
            continue
        try:
            build_event(event)
        except Exception as error:  # noqa: BLE001
            print(f"  evento {event[0]} non scaricato: {error}", flush=True)
    if "--no-live" not in sys.argv:
        try:
            build_live()
        except Exception as error:  # noqa: BLE001
            print(f"  serie live non scaricata: {error}", flush=True)


if __name__ == "__main__":
    main()
