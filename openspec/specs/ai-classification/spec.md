# ai-classification Specification

## Purpose
Assegna automaticamente e in locale le catalogazioni semantiche alle foto (tipo soggetto, interno/esterno e ambiente, presenza e numero di persone, luogo) tramite modelli neurali eseguiti sul computer dell'utente.

## Requirements

### Requirement: Esecuzione esclusivamente locale
Tutta l'analisi AI MUST essere eseguita sul computer dell'utente; nessuna immagine, miniatura, embedding o risultato SHALL essere inviato in rete.

#### Scenario: Analisi offline
- **WHEN** i modelli sono già scaricati e il computer è offline
- **THEN** l'analisi AI viene completata normalmente

### Requirement: Tassonomia configurabile
Il sistema SHALL fornire una tassonomia predefinita di catalogazioni (tipo soggetto, locazione interno/esterno con sottocategorie, ambiente) e l'utente SHALL poter aggiungere, rinominare o disattivare voci con etichette testuali senza riaddestrare modelli.

#### Scenario: Nuova categoria
- **WHEN** l'utente aggiunge la voce "Barche" al tipo soggetto e rianalizza una selezione
- **THEN** le foto con barche possono ricevere la catalogazione "Barche"

#### Scenario: Tassonomia predefinita
- **WHEN** l'utente non ha modificato la tassonomia
- **THEN** sono disponibili almeno: tipo soggetto (ritratto, gruppo, natura, paesaggio, ambiente urbano, monumento, opera d'arte, animale, fiori, alberi, cibo, oggetto), locazione (interno: stanza, teatro, palazzetto, chiesa, museo; esterno urbano: via, piazza; esterno natura: mare, montagna, lago, fiume, bosco, campagna)

### Requirement: Classificazione zero-shot con confidenza
Per ogni foto analizzata il sistema SHALL assegnare per ciascun gruppo della tassonomia le voci più probabili con un punteggio di confidenza da 0 a 1, conservando solo quelle sopra una soglia configurabile.

#### Scenario: Paesaggio montano
- **WHEN** viene analizzata la foto di un paesaggio montano
- **THEN** la foto riceve "Esterno natura" e "Montagna" con le relative confidenze e le voci sotto soglia non sono assegnate

### Requirement: Rilevamento e conteggio persone
Il sistema SHALL rilevare se nella foto sono presenti persone e quante, memorizzando il conteggio e le aree rilevate.

#### Scenario: Foto di gruppo
- **WHEN** viene analizzata una foto con 4 persone
- **THEN** la foto risulta "Persone: presenti" con conteggio 4

#### Scenario: Nessuna persona
- **WHEN** viene analizzata una foto senza persone
- **THEN** la foto risulta "Persone: assenti" con conteggio 0

### Requirement: Geolocalizzazione
Il sistema SHALL determinare il luogo dalle coordinate GPS EXIF quando presenti (con nome località da un dataset geografico offline) e altrimenti dal nome delle directory del percorso, indicando sempre la fonte del luogo.

#### Scenario: GPS presente
- **WHEN** una foto contiene coordinate GPS di Firenze
- **THEN** il luogo è "Firenze" con fonte "GPS"

#### Scenario: Luogo da directory
- **WHEN** una foto senza GPS si trova in `D:\Foto\2023\Toscana\Siena`
- **THEN** il luogo proposto è "Siena" con fonte "directory"

### Requirement: Correzione manuale prevale
L'utente SHALL poter correggere qualsiasi catalogazione automatica; le correzioni manuali MUST prevalere sulle analisi successive.

#### Scenario: Rianalisi dopo correzione
- **WHEN** l'utente cambia il tipo soggetto da "Natura" a "Opera d'arte" e poi la foto viene rianalizzata
- **THEN** il tipo soggetto resta "Opera d'arte"

### Requirement: Versionamento delle analisi
Ogni risultato AI SHALL riportare la versione del modello e della tassonomia che l'ha prodotto, così che il sistema possa proporre la rianalisi dei soli file prodotti con versioni precedenti.

#### Scenario: Modello aggiornato
- **WHEN** viene installata una nuova versione del modello di classificazione
- **THEN** il sistema indica quanti file sono stati analizzati con la versione precedente e permette di rianalizzarli

### Requirement: Analisi riprendibile
L'analisi AI SHALL essere idempotente per file: un'analisi interrotta SHALL poter riprendere elaborando solo i file non ancora completati.

#### Scenario: Chiusura durante l'analisi
- **WHEN** l'app viene chiusa con l'analisi al 60%
- **THEN** alla ripresa vengono elaborati solo i file rimanenti
