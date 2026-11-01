# media-ingestion Specification

## Purpose
Porta i file nella libreria: gestisce le sorgenti, scansiona le cartelle, estrae caratteristiche tecniche, EXIF e date, genera miniature e riconosce file modificati, spostati o duplicati.

## Requirements

### Requirement: Sorgenti di import
L'utente SHALL poter aggiungere una o più cartelle sorgente, con opzioni di scansione ricorsiva, formati inclusi e pattern da ignorare; le sorgenti SHALL essere persistite e rimovibili. Aggiornamento e rimozione di una sorgente SHALL essere disponibili sia dalla pagina Import sia dalla barra laterale della Libreria.

#### Scenario: Aggiunta sorgente ricorsiva
- **WHEN** l'utente aggiunge `D:\Foto` con scansione ricorsiva attiva
- **THEN** vengono considerati i file supportati in `D:\Foto` e in tutte le sottocartelle, esclusi quelli che corrispondono ai pattern ignorati

#### Scenario: Rimozione sorgente
- **WHEN** l'utente rimuove una sorgente
- **THEN** i file di quella sorgente vengono rimossi dalla libreria senza toccare i file sul disco

#### Scenario: Rimozione dalla Libreria
- **WHEN** l'utente apre il menu di una sorgente nella barra laterale della Libreria e sceglie "Rimuovi dalla libreria…"
- **THEN** viene mostrata la stessa conferma con riepilogo della pagina Import

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
Il sistema SHALL determinare la data/ora di prima acquisizione (EXIF DateTimeOriginal, poi CreateDate, poi data di creazione del file) e la data/ora di ultima modifica, memorizzando la fonte usata. Insieme alla data di acquisizione SHALL memorizzare il fuso dichiarato nei metadati (OffsetTimeOriginal o equivalente), quando presente, e il dispositivo che ha prodotto il file.

#### Scenario: EXIF assente
- **WHEN** un file non ha date EXIF
- **THEN** la data di acquisizione è presa dal file system e la fonte è indicata come "file system"

#### Scenario: Fuso dichiarato
- **WHEN** un HEIC di iPhone ha DateTimeOriginal 2026-08-19 10:44:12 e OffsetTimeOriginal -06:00
- **THEN** la libreria memorizza la data 2026-08-19 10:44:12 con fuso -06:00 e dispositivo "Apple iPhone 17 Pro Max"

#### Scenario: Libreria esistente
- **WHEN** una libreria importata prima dell'aggiornamento viene aperta
- **THEN** i fusi mancanti vengono letti nuovamente dai file in un job in background, senza ripetere analisi AI né rigenerare miniature

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
Una nuova scansione di una sorgente SHALL rielaborare solo i file nuovi o modificati (dimensione o data di modifica diverse) e marcare come mancanti i file scomparsi. I file il cui contenuto è cambiato SHALL essere rianalizzati, conservando correzioni manuali, tag e cataloghi.

#### Scenario: Rescan senza modifiche
- **WHEN** una sorgente già importata viene riscansionata senza cambiamenti sul disco
- **THEN** nessun file viene rianalizzato

#### Scenario: File spostato rilevato
- **WHEN** un file viene spostato in un'altra cartella della stessa sorgente
- **THEN** il sistema lo riconosce tramite impronta del contenuto e conserva le sue catalogazioni e i cataloghi di appartenenza

#### Scenario: Foto ritoccata
- **WHEN** l'utente ritaglia una foto con un altro programma, ne ha corretto a mano il tipo soggetto, e aggiorna la sorgente
- **THEN** la foto ha una nuova miniatura e una nuova analisi automatica, e il tipo soggetto corretto a mano, i tag e i cataloghi restano invariati

#### Scenario: File mancante tornato presente
- **WHEN** un file marcato "mancante" ricompare nello stesso percorso e la sorgente viene aggiornata
- **THEN** il file torna disponibile con tutte le sue catalogazioni

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

### Requirement: Rimozione completa di una sorgente
Prima di rimuovere una sorgente il sistema SHALL mostrare un riepilogo: numero di foto, foto con correzioni manuali o tag, cataloghi manuali che perderanno foto e persone con nome che resteranno senza volti. Dopo la conferma il sistema SHALL annullare i job in corso su quella sorgente e cancellare anche miniature, embedding e dati dei volti delle sue foto, MUST NOT toccare i file sul disco, e SHALL eliminare le persone senza nome rimaste prive di volti.

#### Scenario: Importazione fatta per errore
- **WHEN** l'utente ha importato per errore `D:\Download` con 1.200 foto e sceglie "Rimuovi dalla libreria…"
- **THEN** la conferma indica 1.200 foto e, dopo la conferma, le foto non compaiono più in libreria, le loro miniature sono cancellate dalla cache e i file in `D:\Download` sono intatti

#### Scenario: Rimozione durante l'import
- **WHEN** l'import di una sorgente è in corso e l'utente la rimuove
- **THEN** l'import viene annullato prima della rimozione e nessuna foto di quella sorgente ricompare in libreria al termine

#### Scenario: Persone coinvolte
- **WHEN** la persona con nome "Giulia" compare solo in foto della sorgente da rimuovere
- **THEN** il riepilogo indica che "Giulia" resterà senza foto, e dopo la rimozione "Giulia" resta tra le persone con nome con 0 foto

#### Scenario: Persone senza nome
- **WHEN** un cluster "Senza nome" contiene solo volti della sorgente rimossa
- **THEN** dopo la rimozione quel cluster non esiste più

### Requirement: Riepilogo dell'aggiornamento
Al termine dell'aggiornamento di una sorgente il sistema SHALL mostrare quanti file sono nuovi, modificati, spostati o rinominati, diventati mancanti e tornati presenti, con accesso diretto alla libreria filtrata su ciascun gruppo.

#### Scenario: Cartella riorganizzata
- **WHEN** l'utente ha aggiunto 30 foto, spostato 12 foto in un'altra sottocartella e cancellato 5 foto, e poi aggiorna la sorgente
- **THEN** il riepilogo indica 30 nuove, 12 spostate e 5 mancanti, e un clic su "5 mancanti" mostra in libreria quei 5 file

#### Scenario: Nessuna modifica
- **WHEN** l'utente aggiorna una sorgente senza cambiamenti
- **THEN** il riepilogo indica che non ci sono modifiche

### Requirement: Aggiornamento di una cartella
L'utente SHALL poter aggiornare una sola sottocartella di una sorgente; l'aggiornamento SHALL considerare solo i file in quella cartella e nelle sue sottocartelle e marcare come mancanti solo i file al suo interno.

#### Scenario: Sottocartella modificata
- **WHEN** l'utente aggiunge foto in `D:\Foto\2024\Mare` e sceglie "Aggiorna cartella…" su quella cartella
- **THEN** vengono importate le nuove foto di `Mare` e nessun file fuori da `Mare` viene marcato mancante o rielaborato
