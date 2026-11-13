# ECHO Cosmic Ray

**ECHO Cosmic Ray** è una dashboard desktop per Windows dedicata al monitoraggio in tempo quasi reale della **radiazione cosmica** e delle **condizioni di meteo spaziale** che ne influenzano l'intensità.

Raccoglie dati pubblici dalla rete dei neutron monitor (NMDB) e da NOAA SWPC, li elabora con un modello fisico semplificato e li presenta con indicatori, serie temporali, profilo di dose in quota e mappa mondiale dell'intensità al suolo. Avvisa con le notifiche di Windows quando succede qualcosa di rilevante.

## Funzionalità

- **Raggi cosmici al suolo (NMDB)**: neutron monitor di **Oulu** (Finlandia, 0,8 GV) e **Roma** (6,2 GV).
  - livello rispetto al **minimo solare** (Oulu: dicembre 2009; Roma: dicembre 2020), che misura la modulazione solare;
  - ultime 24 ore al minuto, come variazione rispetto alla media dei 30 giorni (cali di Forbush, GLE);
  - ultimi 30 giorni in medie orarie.
- **Eventi e notifiche**: tempeste geomagnetiche (G), tempeste di radiazione (S), brillamenti (R), calo di Forbush, possibile **GLE** (aumento improvviso al suolo), Bz fortemente negativo e vento solare molto veloce. Le soglie hanno l'isteresi, così una notifica arriva una volta sola. Gli eventi già in corso all'avvio non generano notifiche.
- **Meteo spaziale**: Kp al minuto, blocchi Kp di 3 ore con **previsione a 3 giorni**, X-ray GOES, protoni ≥10 MeV, vento solare, densità, Bz/Bt, Dst, F10.7, **scale NOAA R/S/G** con probabilità e ultimi bollettini SWPC.
- **Profilo di dose per quota** (0-40 km) per la località scelta, confrontato con una zona polare e una equatoriale. Tiene conto della latitudine geomagnetica e della modulazione solare osservata.
- **Mappa mondiale** dell'intensità al suolo con **dipolo inclinato** (polo geomagnetico IGRF 2025) e i neutron monitor della rete NMDB (dettagli al passaggio del mouse).
- **Località** selezionabili (Rive d'Arcano, Udine, Milano, Roma, Palermo, Oulu, Singapore) e orario anche in **ora italiana**.
- **Storico orario in CSV** (separatore `;`, virgola decimale, si apre con Excel).

## Atlante dei raggi cosmici

Una seconda pagina dedicata alla rete mondiale dei neutron monitor, raggiungibile dal pulsante in alto.

- **Mappa animata**: le 42 stazioni NMDB dell'atlante, colorate secondo la variazione percentuale dei conteggi (blu = calo, giallo/rosso = aumento). Passa il mouse su un punto per nome, rigidità di taglio, quota e valore.
- **Eventi storici riproducibili**: GLE 66 (ottobre 2003), GLE 69 (gennaio 2005), GLE 71 (maggio 2012), GLE 74 (maggio 2024), cali di Forbush del giugno 1991, novembre 2003, giugno 2015, maggio 2024 e aprile 2023. Il cursore temporale scorre la finestra dell'evento (3 giorni a passi di 10 minuti) e la riproduzione automatica ha velocità 0,5×, 1×, 2× e 4×.
- **Heatmap stazioni × tempo**: 42 righe ordinate per rigidità di taglio, una colonna per passo. È la vista della propagazione: si vede il fronte dell'evento scendere dalle stazioni polari a quelle equatoriali, e si vede dove la rete tace (grigio scuro).
- **Spettro di rigidità**: a ogni istante, variazione percentuale contro rigidità di taglio in scala logaritmica, con il fit Δ% = a − γ·ln(Rc). Nei GLE la risposta crolla passando alle stazioni equatoriali (γ grande), nei Forbush resta piatta.
- **Eventi a confronto**: i nove eventi sovrapposti come mediana della rete e allineati al picco. I GLE sono picchi stretti, i cali di Forbush depressioni larghe e simmetriche: la forma racconta il fenomeno meglio di qualunque numero.
- **Piccoli multipli**: una curva per stazione, ordinate per **rigidità di taglio crescente**, così si vede che le stazioni polari rispondono anche alle particelle meno energetiche mentre quelle equatoriali vedono solo gli eventi più duri. Clicca un pannello per evidenziare la stazione sulla mappa.
- **Ciclo solare dal 1964**: variazione mensile di Oulu contro il numero di macchie solari SILSO, sullo stesso grafico, per vedere la modulazione solare su scala di sessant'anni.
- **Dati live**: gli ultimi 7 giorni di tutte le stazioni vengono riscaricati da NMDB all'apertura della pagina e ogni 30 minuti; se la rete non risponde resta l'istantanea incorporata nell'eseguibile, e l'atlante funziona comunque.
- **Proxy NMDB**: `www.nmdb.eu` non è raggiungibile da tutte le reti, e il reader pubblico `r.jina.ai` è lento e rate-limita. Le richieste passano quindi da un piccolo Cloudflare Worker (`nmdb-proxy.gimmy077.workers.dev`) che inoltra solo `/nest/draw_graph.php`, estrae la tabella ASCII e la tiene in cache 60 secondi. Se il proxy non risponde, l'app prova la connessione diretta e poi il reader.
- **Archivio proprio**: un secondo Worker (`nmdb-archive.gimmy077.workers.dev`) salva ogni ora le misure delle 42 stazioni in un bucket R2 dedicato, perché NMDB conserva solo gli ultimi 7 giorni. Gli oggetti sono in `rete/AAAA/MM/GG/HH.json`; `GET /archive?giorno=AAAA-MM-GG` restituisce un giorno intero. L'archivio cresce da solo e servirà a estendere la finestra dell'atlante oltre la settimana.
- Gli eventi storici e l'istantanea live sono **incorporati nell'exe** in un formato binario compatto (`NMDA`): la pagina si apre senza rete e l'animazione non dipende da download.

## Cartella dati

`%APPDATA%\ECHO Cosmic Ray\` (pulsante **Cartella dati** in fondo alla finestra):

| File | Contenuto |
|---|---|
| `impostazioni.txt` | località, quota, notifiche attive/spente |
| `storico_orario.csv` | una riga per ora: conteggi Oulu/Roma, % dal minimo solare, Kp max, X-ray max, protoni max, vento, Bz min, Dst |

## Sorgenti dati e cadenze

Ogni fonte ha la propria cadenza: si scarica solo ciò che è scaduto, in parallelo. Per i neutron monitor si scaricano solo le ultime 3 ore e le si unisce alla serie già in memoria.

| Fonte | Dato | Cadenza |
|---|---|---|
| NMDB | Oulu e Roma al minuto (conteggi/s corretti per efficienza) | 2 min |
| NMDB | Oulu e Roma, medie orarie 30 giorni | 1 h |
| NOAA SWPC | Kp stimato al minuto, vento solare e IMF (valore più recente) | 2 min |
| NOAA SWPC | X-ray GOES 0,1-0,8 nm | 3 min |
| NOAA SWPC | protoni GOES ≥10 MeV | 5 min |
| NOAA SWPC | scale NOAA, bollettini | 10 min |
| NOAA SWPC | serie vento solare e IMF 24 h (RTSW) | 15 min |
| NOAA SWPC / WDC Kyoto | Dst; previsione Kp 3 giorni | 30 min |
| NOAA SWPC | F10.7 | 6 h |

Una fonte non raggiungibile riprova dopo 2 minuti, senza bloccare le altre. I valori anomali di NMDB (per esempio un'ora di Roma a 14 109 conteggi/s il 28/10/2026) vengono scartati con un filtro sulla mediana.

## Modello e note tecniche

- **Riferimenti fissi** (medie mensili NMDB 2008-2026): Oulu, minimo solare 114,07 conteggi/s e media 2008-2025 106,65; Roma, minimo 129,16 e media 2018-2025 124,46. Per Roma si usano solo i dati successivi al salto strumentale dell'agosto 2017.
- **Dose**: ancore UNSCEAR/ICRU per Rc ≈ 5 GV e attività solare media, con interpolazione log-lineare. L'effetto della latitudine cresce con la quota (rapporto polo/equatore ≈ 1,25 al suolo e ≈ 3 a 11 km). La modulazione solare, misurata a Oulu rispetto alla media del ciclo, viene scalata per quota e taglio. **Non è una misura dosimetrica.**
- **Mappa**: taglio di Störmer da dipolo inclinato. Non include le anomalie locali del campo, come quella dell'Atlantico meridionale.
- Le variazioni di modulazione si calcolano su medie a 30-60 minuti, perché il dato al minuto dei neutron monitor è rumoroso (circa ±1,5 %).

## Stack tecnologico

- **Rust** (edition 2021), UI `windows-reactor` (WinUI), rete `ureq` (gzip), parsing `serde_json`
- icona e informazioni di versione incorporate nell'exe (`build.rs` + `winresource`)
- notifiche tramite il notificatore di Windows (PowerShell nascosto, nessuna dipendenza aggiuntiva)

## Struttura del progetto

```text
cosmic-ray-dashboard/
├─ src/
│  ├─ main.rs      # interfaccia, layout, ciclo di aggiornamento
│  ├─ api.rs       # fonti, cadenze, download in parallelo e parsing
│  ├─ state.rs     # serie unite, grandezze derivate (modulazione, Forbush, GLE)
│  ├─ alerts.rs    # eventi con isteresi, notifiche, impostazioni, storico CSV
│  ├─ charts.rs    # grafici (assi, serie, barre Kp osservate/previste)
│  ├─ atlas.rs     # stazioni NMDB, formato binario, eventi, ciclo solare
│  ├─ atlas_draw.rs# piccoli multipli e ciclo solare disegnati in bitmap RGB
│  ├─ atlas_ui.rs  # pagina Atlante: stato, mappa animata, pannelli
│  └─ model.rs     # modello fisico, riferimenti, località, scale NOAA, mappa
├─ assets/         # icona, mappa, logo delle fonti, dati dell'atlante (assets/atlas)
├─ tools/          # script di supporto (icona, mappa, dati dell'atlante)
├─ build.rs
└─ Cargo.toml
```

## Build ed esecuzione

```powershell
cargo test --release
cargo build --release
```

L'exe in `target\release` funziona da solo: la mappa, i logo e l'icona sono incorporati.

## Ringraziamenti

Un grazie sincero a tutti gli enti, gli osservatori e le persone che misurano, conservano e condividono liberamente questi dati:

- **NMDB, Neutron Monitor Database** ([www.nmdb.eu](https://www.nmdb.eu)), con i conteggi in tempo reale e storici
- **Università di Oulu, Osservatorio Geofisico di Sodankylä** ([cosmicrays.oulu.fi](https://cosmicrays.oulu.fi)), per il neutron monitor di Oulu
- **Osservatorio SVIRCO, INAF-IAPS e Università Roma Tre**, per il neutron monitor di Roma
- **NOAA Space Weather Prediction Center** ([www.swpc.noaa.gov](https://www.swpc.noaa.gov)), per Kp, GOES, vento solare, F10.7, scale e bollettini
- **World Data Center for Geomagnetism, Kyoto** ([wdc.kugi.kyoto-u.ac.jp](https://wdc.kugi.kyoto-u.ac.jp)), per l'indice Dst
- **SILSO, Royal Observatory of Belgium** ([sidc.be/silso](https://www.sidc.be/silso/)), per il numero mensile di macchie solari dell'atlante
- **Natural Earth** ([naturalearthdata.com](https://www.naturalearthdata.com)), per le linee di costa (dominio pubblico)

Riconoscimento richiesto da NMDB:

> We acknowledge the NMDB database (www.nmdb.eu), founded under the European Union's FP7 programme (contract no. 213007), and the PIs of individual neutron monitors at: Oulu (Sodankylä Geophysical Observatory of the University of Oulu, Finland) and Rome (SVIRCO Observatory, INAF/IFSI-UniRomaTre, Italy) for providing data.

ECHO Cosmic Ray è un progetto personale e non è affiliato né approvato dagli enti citati. I loghi servono solo a riconoscere la provenienza dei dati.

## Avvertenze

- I dati provengono da servizi esterni pubblici: disponibilità, latenza e formato possono cambiare.
- Le informazioni hanno scopo informativo e divulgativo: per valutazioni operative, sanitarie o di radioprotezione usare fonti e strumenti certificati.

## Copyright

Copyright (c) 2026 **PIGNOLO GIMMY**. Tutti i diritti riservati.
