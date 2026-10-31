# media-ingestion Specification

## Purpose
Porta i file nella libreria: gestisce le sorgenti, scansiona le cartelle, estrae caratteristiche tecniche, EXIF e date, genera miniature e riconosce file modificati, spostati o duplicati.

## Requirements

### Requirement: Sorgenti di import
L'utente SHALL poter aggiungere una o più cartelle sorgente, con opzioni di scansione ricorsiva, formati inclusi e pattern da ignorare; le sorgenti SHALL essere persistite e rimovibili.

#### Scenario: Aggiunta sorgente ricorsiva
- **WHEN** l'utente aggiunge `D:\Foto` con scansione ricorsiva attiva
- **THEN** vengono considerati i file supportati in `D:\Foto` e in tutte le sottocartelle, esclusi quelli che corrispondono ai pattern ignorati

#### Scenario: Rimozione sorgente
- **WHEN** l'utente rimuove una sorgente
- **THEN** i file di quella sorgente vengono rimossi dalla libreria senza toccare i file sul disco

### Requirement: Formati supportati
Il sistema SHALL importare almeno JPEG, PNG, TIFF, WebP, BMP e GIF; HEIC e RAW SHALL essere importati con i soli metadati disponibili quando la decodifica dell'immagine non è supportata, registrando un avviso.

#### Scenario: Formato non decodificabile
- **WHEN** viene scansionato un file HEIC non decodificabile
- **THEN** il file è in libreria con i metadati estraibili, senza miniatura né analisi AI, ed è registrato un avviso in diagnostica

### Requirement: Caratteristiche tecniche
Per ogni file il sistema SHALL estrarre formato, larghezza, altezza, risoluzione orizzontale e verticale (DPI), profondità in bit, spazio colore se disponibile e dimensione in byte.

#### Scenario: Estrazione caratteristiche
- **WHEN** viene importato un JPEG 6000×4000 a 300 DPI
- **THEN** la libreria riporta formato JPEG, 6000×4000, 300×300 DPI, 8 bit per canale e la dimensione in byte

### Requirement: Date di acquisizione e modifica
Il sistema SHALL determinare la data/ora di prima acquisizione (EXIF DateTimeOriginal, poi CreateDate, poi data di creazione del file) e la data/ora di ultima modifica, memorizzando la fonte usata.

#### Scenario: EXIF assente
- **WHEN** un file non ha date EXIF
- **THEN** la data di acquisizione è presa dal file system e la fonte è indicata come "file system"

### Requirement: Metadati EXIF
Il sistema SHALL estrarre e conservare i principali metadati EXIF (fotocamera, obiettivo, esposizione, orientamento, coordinate GPS); un EXIF corrotto SHALL generare un avviso senza interrompere l'import del file.

#### Scenario: EXIF corrotto
- **WHEN** un file ha un blocco EXIF illeggibile
- **THEN** il file viene importato con le caratteristiche tecniche e l'errore è registrato con fase "metadati"

### Requirement: Miniature
Il sistema SHALL generare miniature in cache locale rispettando l'orientamento EXIF, senza modificare i file originali.

#### Scenario: Foto ruotata
- **WHEN** una foto ha orientamento EXIF 90°
- **THEN** la miniatura è mostrata correttamente orientata

### Requirement: Originali in sola lettura
Il processo di import e analisi MUST NOT modificare, spostare o cancellare i file originali.

#### Scenario: Import completo
- **WHEN** termina un import con analisi AI
- **THEN** contenuto e data di modifica dei file originali sono invariati

### Requirement: Scansione incrementale
Una nuova scansione di una sorgente SHALL rielaborare solo i file nuovi o modificati (dimensione o data di modifica diverse) e marcare come mancanti i file scomparsi.

#### Scenario: Rescan senza modifiche
- **WHEN** una sorgente già importata viene riscansionata senza cambiamenti sul disco
- **THEN** nessun file viene rianalizzato

#### Scenario: File spostato rilevato
- **WHEN** un file viene spostato in un'altra cartella della stessa sorgente
- **THEN** il sistema lo riconosce tramite impronta del contenuto e conserva le sue catalogazioni e i cataloghi di appartenenza

### Requirement: Rilevamento duplicati
Il sistema SHALL calcolare un'impronta del contenuto per ogni file e segnalare i duplicati esatti presenti in percorsi diversi.

#### Scenario: Due copie identiche
- **WHEN** lo stesso file è presente in due cartelle
- **THEN** entrambi sono in libreria e sono marcati come duplicati l'uno dell'altro

### Requirement: Monitoraggio delle sorgenti
L'utente SHALL poter attivare per ogni sorgente il monitoraggio delle modifiche, che accoda automaticamente un import incrementale quando la cartella cambia mentre l'app è aperta.

#### Scenario: Nuova foto aggiunta
- **WHEN** l'utente copia una nuova foto in una sorgente monitorata
- **THEN** il file viene importato e accodato per l'analisi senza azioni manuali

### Requirement: Decodifica HEIC su Linux
Su Linux il sistema SHALL decodificare i file HEIC usando la libreria libheif installata nel sistema, quando è presente e include un decoder HEVC; in sua assenza SHALL applicare il comportamento previsto per i formati non decodificabili.

#### Scenario: libheif di sistema presente
- **WHEN** su Linux con libheif e decoder HEVC installati viene scansionato un file HEIC
- **THEN** il file è in libreria con miniatura e analisi AI come gli altri formati supportati

#### Scenario: libheif di sistema assente
- **WHEN** su Linux senza libheif viene scansionato un file HEIC
- **THEN** il file è in libreria con i soli metadati, e l'avviso in diagnostica suggerisce di installare libheif
