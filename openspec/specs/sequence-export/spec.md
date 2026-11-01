# sequence-export Specification

## Purpose
Esporta una sequenza di foto in ordine cronologico reale come copie rinominate secondo una convenzione di nomi, raggruppate anche per tag, in un formato uniforme e verificato, pronte da condividere o archiviare.

## Requirements

### Requirement: Modello del nome dei file
L'utente SHALL poter definire il nome dei file esportati con un modello di testo e token: `{seq}`, `{seq:0Nd}`, `{date}`, `{time}`, `{time_full}`, `{datetime}`, `{tag}`, `{device}`, `{people}`, `{place}`, `{orig}`. Un token sconosciuto o un modello che produce caratteri non validi nei nomi file SHALL essere segnalato prima dell'avvio.

#### Scenario: Modello con progressivo e tag
- **WHEN** il modello è `{seq:04d}_{date}_{time}_{tag}` e la prima foto è delle 10:43 del 19 agosto 2026 con tag "Paesaggi"
- **THEN** il suo nome è `0001_2026-08-19_1043_Paesaggi.jpg`

#### Scenario: Token vuoto
- **WHEN** una foto non ha tag del set esclusivo e il modello contiene `_{tag}`
- **THEN** il token e il separatore che lo precede vengono omessi, per esempio `0005_2026-08-19_1052.jpg`

#### Scenario: Token sconosciuto
- **WHEN** il modello contiene `{camera}`
- **THEN** l'export non può essere avviato e il token è evidenziato come sconosciuto

### Requirement: Anteprima dei nomi
Mentre l'utente modifica il modello, il sistema SHALL mostrare i nomi risultanti per le prime foto della sequenza accanto ai nomi originali, aggiornandoli a ogni modifica.

#### Scenario: Anteprima live
- **WHEN** l'utente cambia `{time}` in `{time_full}` nel modello
- **THEN** l'anteprima mostra subito i nomi con ore, minuti e secondi

### Requirement: Ordine e ora usati nell'export
La numerazione `{seq}` SHALL seguire l'ordine per istante reale dopo le correzioni, e i token di data e ora SHALL usare l'ora locale del gruppo di sorgente di ciascuna foto. Nomi uguali SHALL ricevere un suffisso deterministico (`-2`, `-3`).

#### Scenario: Due sorgenti intercalate
- **WHEN** la sequenza alterna scatti X-T5 e iPhone dopo uno scostamento applicato alla X-T5
- **THEN** i progressivi seguono l'ordine corretto e alternano i due dispositivi

### Requirement: Struttura dell'export
L'export SHALL produrre nella destinazione, secondo le opzioni scelte: una cartella `sequenza/` con tutte le foto rinominate, una cartella `per-tag/<Tag>/` per ogni tag del set esclusivo scelto con le stesse foto, un file `manifest.csv` e due archivi zip non compressi (sequenza e per-tag). Le foto senza tag del set compaiono solo in `sequenza/`.

#### Scenario: Struttura completa
- **WHEN** l'utente esporta 912 foto con tutte le opzioni attive e 905 foto hanno un tag del set esclusivo
- **THEN** `sequenza/` contiene 912 file, le cartelle in `per-tag/` contengono in tutto 905 file, e sono presenti `manifest.csv`, `sequenza.zip` e `per-tag.zip`

#### Scenario: Cartelle per tag senza spazio aggiuntivo
- **WHEN** la destinazione supporta gli hard link
- **THEN** i file in `per-tag/` sono hard link ai file in `sequenza/`; altrimenti sono copie e la stima dello spazio ne tiene conto

### Requirement: Manifest dell'export
Il file `manifest.csv` SHALL contenere una riga per foto con nome finale, percorso originale, istante UTC, ora locale con fuso, dispositivo, tag, identità presenti, qualità di codifica usata e, se calcolate, le metriche di verifica.

#### Scenario: Riga del manifest
- **WHEN** una foto è stata ricodificata a qualità 99 dopo una verifica fallita a 97
- **THEN** la sua riga riporta qualità 99 e le metriche misurate a 99

### Requirement: Conversione verificata
Con il formato JPEG il sistema SHALL ricodificare ogni foto alla qualità scelta (predefinita 97, crominanza 4:4:4), confrontarla con l'originale a piena risoluzione e, se le metriche sono sotto le soglie configurate, ricodificarla a qualità crescente fino a 100. Con il formato "originale" i file vengono copiati senza ricodifica.

#### Scenario: Ricodifica automatica
- **WHEN** una foto codificata a 97 ha SSIM sotto la soglia
- **THEN** viene ricodificata a qualità maggiore finché supera la soglia o arriva a 100, e l'esito è registrato

#### Scenario: Soglia mai raggiunta
- **WHEN** anche a qualità 100 una foto resta sotto soglia
- **THEN** la foto viene esportata a 100 e segnalata nell'esito del job e nel manifest

### Requirement: Fedeltà dell'immagine esportata
Le copie esportate SHALL mantenere la risoluzione originale e il profilo colore incorporato, e avere l'orientamento applicato ai pixel.

#### Scenario: PNG con profilo colore
- **WHEN** viene esportato un PNG con profilo Display P3
- **THEN** il JPEG risultante incorpora lo stesso profilo e ha le stesse dimensioni in pixel

### Requirement: Metadati delle copie
Se l'opzione è attiva, le copie esportate SHALL contenere: data e ora corrette con fuso, fotocamera, obiettivo, coordinate GPS, i tag del set come parole chiave e i nomi delle identità presenti. Gli originali MUST NOT essere modificati.

#### Scenario: Istante corretto nei metadati
- **WHEN** una foto con scostamento di +2 min 10 s viene esportata
- **THEN** la data originale nei metadati della copia include lo scostamento e il fuso dell'ora locale del gruppo

### Requirement: Verifica preliminare dello spazio
Prima di scrivere il sistema SHALL stimare lo spazio necessario per le opzioni scelte e confrontarlo con lo spazio libero della destinazione, impedendo l'avvio se insufficiente; SHALL anche indicare se la destinazione supporta gli hard link.

#### Scenario: Spazio insufficiente
- **WHEN** servono 31 GB e la destinazione ne ha 20
- **THEN** l'avvio è bloccato con un messaggio che indica i due valori

### Requirement: Export come job annullabile
L'export della sequenza SHALL essere eseguito come job con progresso; annullandolo o in caso di errore fatale MUST NOT restare nella destinazione file o archivi parziali, e una destinazione non vuota MUST NOT essere sovrascritta.

#### Scenario: Annullamento
- **WHEN** l'utente annulla l'export al 40%
- **THEN** nella destinazione non resta nulla dell'export annullato

#### Scenario: Destinazione già usata
- **WHEN** la destinazione contiene già una cartella `sequenza/`
- **THEN** il sistema chiede di scegliere un'altra destinazione o una sottocartella nuova
