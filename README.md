# ECHO Cosmic Ray

**ECHO Cosmic Ray** è una dashboard desktop per Windows dedicata al monitoraggio in tempo quasi reale della **radiazione cosmica** e delle principali **condizioni di meteo spaziale** che ne influenzano l’intensità.

L’applicazione raccoglie dati pubblici da reti di neutron monitor e da servizi NOAA/SWPC, li elabora con un modello fisico semplificato e li presenta in un’interfaccia grafica compatta con indicatori, serie temporali, profilo di dose in quota e mappa mondiale dell’intensità relativa al suolo.

## Funzionalità

- **Raggi cosmici al suolo (NMDB)**: visualizzazione della variazione percentuale del conteggio dei neutron monitor, con confronto tra **Oulu** (Finlandia, rigidità di taglio ~0,8 GV) e **Roma** (~6,2 GV). La UI mostra sia il dato grezzo al minuto sia una media mobile per evidenziare modulazione solare ed effetti Forbush.
- **Indicatori live**: KPI per intensità GCR, indice geomagnetico **Kp**, velocità del **vento solare** e componente **Bz** del campo magnetico interplanetario (IMF), flusso **X-ray GOES** (0,1–0,8 nm, classi A/B/C/M/X) e stima della dose alla quota selezionata.
- **Driver solari/geomagnetici**: pannelli dedicati a **Dst** (72 h), **Bz IMF** (24 h), **vento solare** (24 h) e **protoni integrali ≥10 MeV** (6 h, GOES).
- **Attività solare**: medie orarie dell’indice Kp e andamento del flusso X-ray GOES.
- **Profilo di dose per quota**: stima della dose equivalente da raggi cosmici (µSv/h) da 0 a 40 km, con selettore di quota, riferimento al livello del suolo e indicatori derivati (pressione ISA approssimata, fattore rispetto al suolo, tempo equivalente a 1 mSv).
- **Mappa mondiale**: intensità relativa modellata al suolo in funzione della latitudine/taglio geomagnetico dipolare e della modulazione solare osservata, con posizione dei neutron monitor NMDB e riferimento a Roma.

## Sorgenti dati

- **NMDB** (Neutron Monitor Database, NEST): serie storiche recenti dei conteggi corretti per efficienza (es. Oulu, Roma).
- **NOAA SWPC**:
  - indice planetario **Kp** (stime al minuto);
  - flusso **X-ray** GOES primario (banda 0,1–0,8 nm);
  - **protoni integrali** GOES (canale ≥10 MeV);
  - **Dst** (Kyoto/NOAA products);
  - flusso radio solare **F10.7**;
  - dati RTSW per **vento solare** e **Bz** (feed “deep”).

L’applicazione aggiorna i dati periodicamente; alcuni feed vengono richiesti con cadenza differenziata per bilanciare completezza e carico.

## Modello e note tecniche

- Il profilo di dose in quota è un **modello semplificato** basato su ancore derivate da misure pubblicate (es. UNSCEAR/ICRU e campagne di volo), con interpolazione e scalatura in base alla modulazione solare osservata. **Non è una misura dosimetrica in tempo reale**.
- La mappa usa una rappresentazione semplificata (dipendenza principalmente dalla latitudine/taglio geomagnetico dipolare) e non include effetti locali complessi (es. anomalia dell’Atlantico meridionale).
- I conteggi dei neutron monitor sono intrinsecamente rumorosi: la dashboard applica aggregazioni/medie mobili per rendere leggibili le variazioni.

## Stack tecnologico

- **Linguaggio**: Rust (edition 2021)
- **UI**: `windows-reactor` (componenti grafici e rendering WinUI/Windows)
- **Networking**: `ureq`
- **Parsing dati**: `serde_json`
- **Target**: applicazione desktop Windows (subsystem Windows)

## Struttura del progetto

```text
cosmic-ray-dashboard/
├─ src/
│  ├─ main.rs      # UI, layout, aggiornamento dati, componenti grafici
│  ├─ api.rs       # download/parsing NMDB e NOAA SWPC
│  ├─ charts.rs    # primitive per grafici (assi, serie, barre Kp)
│  └─ model.rs     # modello fisico semplificato, stazioni, scale/classificazioni
├─ assets/         # icone e risorse grafiche (es. mappa)
├─ tools/          # script di supporto (es. generazione icona/mappa)
├─ Cargo.toml
└─ Cargo.lock
```

## Build ed esecuzione

Prerequisiti:
- Rust toolchain (stable) su Windows
- ambiente Windows compatibile con le dipendenze grafiche utilizzate dal progetto

Comandi:

```powershell
cargo build
cargo run
```

Build ottimizzata:

```powershell
cargo build --release
```

> Nota: `target/` e `dist/` sono esclusi dal versionamento tramite `.gitignore`.

## Avvertenze

- I dati provengono da servizi esterni pubblici: disponibilità, latenza e formato possono cambiare.
- Le informazioni mostrate hanno scopo informativo/visuale; per valutazioni operative, sanitarie o radioprotezionistiche usare fonti e strumenti certificati.

## Copyright

Copyright (c) 2026 **PIGNOLO GIMMY**. Tutti i diritti riservati.
