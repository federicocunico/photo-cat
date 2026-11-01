# Spec Delta

## MODIFIED Requirements

### Requirement: Tassonomia configurabile
Il sistema SHALL fornire una tassonomia predefinita di catalogazioni (tipo soggetto, locazione interno/esterno con sottocategorie, ambiente) e l'utente SHALL poter aggiungere, rinominare o disattivare voci con etichette testuali senza riaddestrare modelli. L'utente SHALL poter creare anche gruppi nuovi e voci prive di descrizione per il riconoscimento automatico ("voci manuali"), che l'analisi non assegna mai ma che l'utente può assegnare, filtrare e usare nei cataloghi smart.

#### Scenario: Nuova categoria
- **WHEN** l'utente aggiunge la voce "Barche" al tipo soggetto e rianalizza una selezione
- **THEN** le foto con barche possono ricevere la catalogazione "Barche"

#### Scenario: Tassonomia predefinita
- **WHEN** l'utente non ha modificato la tassonomia
- **THEN** sono disponibili almeno: tipo soggetto (ritratto, gruppo, natura, paesaggio, ambiente urbano, monumento, opera d'arte, animale, fiori, alberi, cibo, oggetto), locazione (interno: stanza, teatro, palazzetto, chiesa, museo; esterno urbano: via, piazza; esterno natura: mare, montagna, lago, fiume, bosco, campagna)

#### Scenario: Voce manuale
- **WHEN** l'utente aggiunge al tipo soggetto la voce "Matrimonio Anna" senza descrizione e rianalizza la libreria
- **THEN** nessuna foto riceve "Matrimonio Anna" dall'analisi, e le foto a cui l'utente l'ha assegnata la mantengono

#### Scenario: Gruppo nuovo solo manuale
- **WHEN** l'utente crea il gruppo "Evento" con le voci "Compleanno" e "Laurea", senza descrizioni
- **THEN** il gruppo compare nell'Inspector, nella barra bulk, nei filtri della libreria e nell'editor dei cataloghi smart, e l'analisi automatica lo ignora

#### Scenario: Voce resa automatica
- **WHEN** l'utente aggiunge alla voce manuale "Barche" la descrizione "a photo of boats"
- **THEN** la voce diventa automatica e il sistema propone di rianalizzare i file per assegnarla

#### Scenario: Eliminazione di una voce usata
- **WHEN** l'utente elimina la voce "Laurea" assegnata a 25 foto
- **THEN** il sistema chiede conferma indicando 25 foto e, dopo la conferma, le 25 foto non hanno più quella catalogazione

### Requirement: Versionamento delle analisi
Ogni risultato AI SHALL riportare la versione del modello e della tassonomia che l'ha prodotto, così che il sistema possa proporre la rianalisi dei soli file prodotti con versioni precedenti. Le modifiche alla tassonomia che non cambiano l'analisi automatica (voci o gruppi manuali, nomi visualizzati) MUST NOT rendere i file da rianalizzare.

#### Scenario: Modello aggiornato
- **WHEN** viene installata una nuova versione del modello di classificazione
- **THEN** il sistema indica quanti file sono stati analizzati con la versione precedente e permette di rianalizzarli

#### Scenario: Voce manuale aggiunta
- **WHEN** la libreria è interamente analizzata e l'utente aggiunge una voce manuale
- **THEN** il numero di file da rianalizzare resta zero

#### Scenario: Rinomina del nome visualizzato
- **WHEN** l'utente rinomina la voce "Fiori" in "Fiori e piante" senza cambiarne la descrizione
- **THEN** il numero di file da rianalizzare resta zero e le foto con quella voce mostrano il nuovo nome

#### Scenario: Prompt modificato
- **WHEN** l'utente modifica la descrizione di una voce automatica
- **THEN** il sistema indica i file analizzati con la versione precedente e permette di rianalizzarli

## ADDED Requirements

### Requirement: Creazione di voci durante la correzione
Nei selettori dei valori di catalogazione usati per correggere una foto o una selezione, l'utente SHALL poter creare una nuova voce del gruppo scrivendone solo il nome; la voce viene creata come voce manuale e assegnata subito come correzione manuale. Un nome già esistente nel gruppo, senza distinzione di maiuscole e accenti, SHALL riusare la voce esistente invece di crearne un doppione.

#### Scenario: Nuova voce dall'Inspector
- **WHEN** nell'Inspector l'utente sceglie il gruppo "Tipo soggetto", poi "Nuova voce…", scrive "Barche" e conferma
- **THEN** "Barche" viene aggiunta al tipo soggetto e la foto ha "Tipo soggetto = Barche" come correzione manuale

#### Scenario: Nuova voce in bulk
- **WHEN** l'utente seleziona 60 foto e, nell'azione "Imposta catalogazione", crea la voce "Gita scolastica"
- **THEN** la voce viene creata una sola volta e le 60 foto la ricevono come correzione manuale, con possibilità di annullare l'assegnazione

#### Scenario: Nome già esistente
- **WHEN** l'utente crea nel tipo soggetto la voce "fiori" e il gruppo contiene già "Fiori"
- **THEN** non viene creato un doppione e viene assegnata la voce "Fiori"

#### Scenario: Nome vuoto
- **WHEN** l'utente conferma "Nuova voce…" senza scrivere un nome
- **THEN** nessuna voce viene creata e il campo segnala che il nome è obbligatorio
