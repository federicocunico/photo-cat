# Spec Delta

## MODIFIED Requirements

### Requirement: Data di scatto impostata a mano
L'utente SHALL poter impostare a mano la data di scatto di una o più foto dall'Inspector o in bulk, con precisione giorno, mese oppure solo anno, e ripristinare la data letta dal file. Con più foto selezionate l'Inspector SHALL mostrare la data di scatto comune o "Valori diversi" e permettere di impostarla o ripristinarla su tutta la selezione; sia dall'Inspector sia dal menu contestuale SHALL potersi impostare giorno, mese e anno o una precisione ridotta, e senza una data manuale comune i campi SHALL partire vuoti. La data manuale SHALL avere fonte "manuale", prevalere su quella dei metadati in ordinamento, filtri per data, Sequenza, cataloghi smart ed export, ed essere annullabile. I file originali MUST NOT essere modificati.

#### Scenario: Negativo scansionato
- **WHEN** una scansione ha data EXIF 2019-03-02 e l'utente imposta "giugno 1984"
- **THEN** l'Inspector mostra "giugno 1984 (impostata a mano)" e la foto compare nel filtro per data sotto 1984 › giugno

#### Scenario: Solo anno
- **WHEN** l'utente imposta l'anno 1962 su 40 stampe scansionate
- **THEN** le 40 foto compaiono sotto l'anno 1962 nel filtro per data, senza mese, e l'Inspector mostra "1962 (impostata a mano)"

#### Scenario: Ordinamento con precisione ridotta
- **WHEN** in un catalogo ci sono una foto con data manuale "1984" e una con data EXIF 1984-06-10 14:00
- **THEN** la foto con solo anno è ordinata al 1° gennaio 1984 alle 00:00, quindi prima dell'altra

#### Scenario: Ripristino
- **WHEN** l'utente sceglie "Ripristina data del file" su una foto con data manuale
- **THEN** la foto torna alla data e alla fonte lette dai metadati o dal file system

#### Scenario: Esclusa dalle foto senza data
- **WHEN** una foto con fonte "file system" riceve una data manuale
- **THEN** non compare più nell'area delle foto senza data della Sequenza

#### Scenario: Correzioni di gruppo
- **WHEN** una foto ha una data manuale e l'utente applica uno scostamento al suo gruppo di sorgente
- **THEN** la data manuale della foto non cambia

#### Scenario: Data completa su più foto dall'Inspector
- **WHEN** l'utente seleziona le 120 scansioni di "Praga", tutte con data del file nel 2008, apre "Data di scatto" nell'Inspector e conferma 14 agosto 1976 con precisione "giorno"
- **THEN** le 120 foto hanno data di scatto 14 agosto 1976 con precisione "giorno" e fonte "manuale", l'Inspector mostra "14 agosto 1976 (impostata a mano)" e "Annulla operazione" ripristina le date precedenti

#### Scenario: Solo anno su più foto
- **WHEN** l'utente seleziona 60 scansioni, apre "Data di scatto" nell'Inspector, sceglie la precisione "anno" e conferma 1976
- **THEN** le 60 foto hanno data di scatto 1976 con precisione "anno" e fonte "manuale"

#### Scenario: Date diverse nella selezione
- **WHEN** l'utente seleziona 3 foto con date di scatto diverse
- **THEN** la riga "Data di scatto" dell'Inspector mostra "Valori diversi" e l'editor parte con precisione "giorno" e i campi vuoti

#### Scenario: Data manuale comune
- **WHEN** l'utente seleziona 5 foto che hanno tutte la data manuale "giugno 1984" e apre l'editor della data
- **THEN** l'editor parte con precisione "mese", giugno e 1984

#### Scenario: Ripristino su più foto
- **WHEN** l'utente seleziona 10 foto con data manuale e sceglie "Ripristina data del file" nell'Inspector
- **THEN** le 10 foto tornano alla data e alla fonte lette dai metadati o dal file system

#### Scenario: Data completa dal menu contestuale
- **WHEN** l'utente seleziona 60 scansioni senza data manuale, sceglie "Imposta data di scatto…" dal menu contestuale e conferma 3 marzo 1987
- **THEN** il dialogo era partito con precisione "giorno" e i campi vuoti, e le 60 foto hanno data di scatto 3 marzo 1987 con precisione "giorno" e fonte "manuale"

#### Scenario: Una sola foto
- **WHEN** l'utente apre l'editor della data su una sola foto senza data manuale
- **THEN** l'editor parte con precisione "giorno", come prima di questa modifica
