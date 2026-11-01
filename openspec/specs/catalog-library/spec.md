# catalog-library Specification

## Purpose
Gestisce la libreria unica di tutti i media aggiunti e i cataloghi tematici (manuali e smart) con cui l'utente organizza, cerca e filtra i propri file.

## Requirements

### Requirement: Libreria unica dei media
Il sistema SHALL mantenere una libreria persistente e locale che contiene ogni file importato, una sola volta per percorso, con i suoi metadati tecnici, le date, le catalogazioni e lo stato di analisi.

#### Scenario: File importato appare in libreria
- **WHEN** un file supportato viene importato da una sorgente
- **THEN** il file compare in libreria con percorso completo, nome file e caratteristiche tecniche

#### Scenario: Persistenza tra riavvii
- **WHEN** l'utente chiude e riapre l'applicazione
- **THEN** la libreria mostra gli stessi file, catalogazioni e cataloghi di prima della chiusura

### Requirement: Modello di media estendibile
Ogni elemento della libreria SHALL avere un tipo di media; la prima versione supporta il tipo "foto" e il modello dati SHALL consentire di aggiungere altri tipi (es. video, documenti) e nuove catalogazioni senza invalidare i dati esistenti.

#### Scenario: Aggiornamento dello schema
- **WHEN** una nuova versione dell'applicazione introduce un nuovo campo di catalogazione
- **THEN** la libreria esistente viene migrata automaticamente e i dati precedenti restano disponibili

### Requirement: Cataloghi manuali
L'utente SHALL poter creare, rinominare ed eliminare cataloghi tematici manuali e aggiungervi o rimuovervi file; un file può appartenere a più cataloghi ed eliminare un catalogo non elimina i file.

#### Scenario: Aggiunta di file a un catalogo
- **WHEN** l'utente aggiunge 3 file al catalogo "Viaggio Toscana"
- **THEN** il catalogo mostra quei 3 file e il conteggio del catalogo è 3

#### Scenario: Eliminazione catalogo
- **WHEN** l'utente elimina un catalogo
- **THEN** i file restano in libreria e negli altri cataloghi

### Requirement: Cataloghi smart
L'utente SHALL poter definire cataloghi smart basati su regole combinate (AND/OR) su metadati, date, luogo, persone, catalogazioni AI e tag; il contenuto SHALL aggiornarsi automaticamente quando cambiano i dati dei file.

#### Scenario: Regola smart
- **WHEN** l'utente crea un catalogo smart "Tipo soggetto = Ritratto AND Anno >= 2022"
- **THEN** il catalogo contiene esattamente i file che soddisfano entrambe le condizioni

#### Scenario: Aggiornamento automatico
- **WHEN** un nuovo file analizzato soddisfa le regole di un catalogo smart
- **THEN** il file compare nel catalogo senza azioni manuali

### Requirement: Tag utente
L'utente SHALL poter assegnare e rimuovere tag liberi ai file; i tag utente SHALL essere distinti dalle catalogazioni automatiche e non essere mai sovrascritti da un'analisi.

#### Scenario: Rianalisi non tocca i tag
- **WHEN** un file con tag utente viene rianalizzato
- **THEN** i suoi tag utente restano invariati

### Requirement: Ricerca e filtri
Il sistema SHALL permettere di filtrare la libreria o un catalogo per sorgente, cartella (con o senza sottocartelle), tipo soggetto, interno/esterno, persone, luogo, intervallo di date, formato e tag, e di cercare per testo su nome file, percorso, luogo, persone e tag.

#### Scenario: Filtri combinati
- **WHEN** l'utente seleziona i filtri "Esterno" e "Persona: Giulia"
- **THEN** vengono mostrati solo i file esterni in cui è presente Giulia

#### Scenario: Risposta su librerie grandi
- **WHEN** la libreria contiene 100.000 file e l'utente applica un filtro
- **THEN** il primo blocco di risultati viene mostrato entro 1 secondo su hardware consumer

#### Scenario: Filtro per cartella con sottocartelle
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone` includendo le sottocartelle
- **THEN** vengono mostrati i file di `Viaggio Giappone` e di tutte le sue sottocartelle (`D7000`, `Telefono`, …)

#### Scenario: Filtro per cartella esatta
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone\D7000` senza sottocartelle
- **THEN** vengono mostrati solo i file contenuti direttamente in quella cartella

### Requirement: Stato dei file sul disco
Il sistema SHALL segnalare i file della libreria non più presenti sul disco come "mancanti" senza rimuoverli automaticamente, e permettere all'utente di ricollegarli o rimuoverli.

#### Scenario: File spostato fuori dal programma
- **WHEN** un file catalogato viene cancellato o spostato esternamente
- **THEN** al successivo controllo il file è marcato "mancante" e mantiene le sue catalogazioni

### Requirement: Albero delle cartelle
Il sistema SHALL mostrare, sotto ogni sorgente, l'albero delle cartelle che contengono file della libreria con il numero di file per cartella (incluse le sottocartelle); l'albero SHALL caricarsi un livello alla volta per restare reattivo su librerie grandi.

#### Scenario: Espansione di una sorgente
- **WHEN** l'utente espande la sorgente `D:\Foto` che contiene `Viaggio Giappone` con 3 sottocartelle
- **THEN** compare `Viaggio Giappone` con il conteggio totale e, espandendola, le 3 sottocartelle con i rispettivi conteggi

#### Scenario: Seleziona tutto in una cartella
- **WHEN** l'utente sceglie una cartella nell'albero e poi "Seleziona tutto"
- **THEN** risultano selezionati tutti i file di quella cartella, pronti per un'azione bulk
