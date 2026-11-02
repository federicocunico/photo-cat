# Spec Delta

## MODIFIED Requirements

### Requirement: Formati supportati
Il sistema SHALL importare almeno JPEG, PNG, TIFF, WebP, BMP e GIF; HEIC e RAW SHALL essere importati con i soli metadati disponibili quando la decodifica dell'immagine non è supportata, registrando un avviso. Quando la decodifica fallisce, il sistema SHALL tentare un decodificatore alternativo per le varianti JPEG note e SHALL registrare la causa riconosciuta (file troncato o danneggiato, variante non supportata, anteprima RAW assente, immagine oltre i limiti) invece di un errore generico.

#### Scenario: Formato non decodificabile
- **WHEN** viene scansionato un file HEIC non decodificabile
- **THEN** il file è in libreria con i metadati estraibili, senza miniatura né analisi AI, ed è registrato un avviso in diagnostica

#### Scenario: JPEG CMYK
- **WHEN** viene importato un JPEG in CMYK che il decodificatore principale non legge
- **THEN** la miniatura viene prodotta con il decodificatore alternativo e non viene registrato alcun errore

#### Scenario: File troncato
- **WHEN** viene importato un JPEG troncato a metà
- **THEN** il file è in libreria e l'errore indica "file danneggiato o incompleto", distinto da "formato non supportato"

#### Scenario: Immagine enorme
- **WHEN** viene importato un PNG da 30.000×20.000 pixel
- **THEN** la miniatura viene prodotta senza superare il budget di memoria, oppure il file viene registrato come "immagine oltre i limiti" senza far chiudere l'app

## ADDED Requirements

### Requirement: Import di librerie grandi
Il sistema SHALL importare e analizzare una libreria di 100.000 foto in un'unica sorgente senza chiudersi, entro il budget di memoria, mantenendo l'interfaccia utilizzabile durante tutto l'import, e SHALL poter riprendere l'import interrotto senza rielaborare i file già completati. L'import MUST NOT modificare né cancellare file nella sorgente.

#### Scenario: Centomila foto
- **WHEN** l'utente importa una cartella con 100.000 foto su un computer con 16 GB di RAM e profilo "Bilanciato"
- **THEN** l'import termina senza chiusure anomale, la memoria dell'app resta entro il budget più la quota dell'interfaccia, e i file della sorgente sono invariati

#### Scenario: Navigazione durante l'import
- **WHEN** l'import di 100.000 foto è al 50% e l'utente scorre la libreria e apre l'Inspector
- **THEN** la griglia e l'Inspector rispondono senza blocchi percepibili

#### Scenario: Ripresa dopo chiusura
- **WHEN** l'app viene chiusa al 70% dell'import e riaperta
- **THEN** l'import riprende elaborando solo i file non ancora completati
