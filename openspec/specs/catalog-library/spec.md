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
L'utente SHALL poter assegnare e rimuovere tag liberi ai file; i tag utente SHALL essere distinti dalle catalogazioni automatiche e non essere mai sovrascritti da un'analisi. Ogni tag SHALL essere un'entità con nome univoco (senza distinzione di maiuscole) e colore, così che rinominarlo o unirlo a un altro mantenga le assegnazioni ai file.

#### Scenario: Rianalisi non tocca i tag
- **WHEN** un file con tag utente viene rianalizzato
- **THEN** i suoi tag utente restano invariati

#### Scenario: Stesso tag con maiuscole diverse
- **WHEN** l'utente aggiunge il tag "matrimonio" e in libreria esiste già "Matrimonio"
- **THEN** il file riceve il tag esistente "Matrimonio" e non ne viene creato uno nuovo

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
Il sistema SHALL segnalare i file della libreria non più presenti sul disco come "mancanti" senza rimuoverli automaticamente, e permettere all'utente di ricollegarli o rimuoverli. Il ricollegamento SHALL poter indicare la nuova posizione di una cartella e ritrovare i file per percorso relativo o per impronta del contenuto; la rimozione SHALL poter riguardare tutti i mancanti di una sorgente in un'unica azione confermata.

#### Scenario: File spostato fuori dal programma
- **WHEN** un file catalogato viene cancellato o spostato esternamente
- **THEN** al successivo controllo il file è marcato "mancante" e mantiene le sue catalogazioni

#### Scenario: Cartella spostata su un altro disco
- **WHEN** l'utente ha spostato `D:\Foto\Viaggio` in `E:\Archivio\Viaggio`, la libreria segna 400 file mancanti e l'utente sceglie "Ricollega…" indicando la nuova cartella
- **THEN** i 400 file tornano disponibili con le loro catalogazioni, i tag, i cataloghi e le persone, e il riepilogo indica quanti file sono stati ritrovati e quanti restano mancanti

#### Scenario: File rinominati nella nuova cartella
- **WHEN** nella cartella indicata per il ricollegamento alcuni file hanno un nome diverso ma lo stesso contenuto
- **THEN** quei file vengono ricollegati tramite impronta del contenuto

#### Scenario: Rimozione dei mancanti
- **WHEN** una sorgente ha 5 file mancanti e l'utente sceglie "Rimuovi mancanti…"
- **THEN** dopo una conferma che indica 5 file, quei file non sono più in libreria e gli altri file della sorgente restano invariati

### Requirement: Albero delle cartelle
Il sistema SHALL mostrare, sotto ogni sorgente, l'albero delle cartelle che contengono file della libreria con il numero di file per cartella (incluse le sottocartelle); l'albero SHALL caricarsi un livello alla volta per restare reattivo su librerie grandi.

#### Scenario: Espansione di una sorgente
- **WHEN** l'utente espande la sorgente `D:\Foto` che contiene `Viaggio Giappone` con 3 sottocartelle
- **THEN** compare `Viaggio Giappone` con il conteggio totale e, espandendola, le 3 sottocartelle con i rispettivi conteggi

#### Scenario: Seleziona tutto in una cartella
- **WHEN** l'utente sceglie una cartella nell'albero e poi "Seleziona tutto"
- **THEN** risultano selezionati tutti i file di quella cartella, pronti per un'azione bulk

### Requirement: Elenco completo dei valori dei filtri
Ogni lista di filtro della libreria (catalogazioni AI per gruppo, luoghi, anni, formati, tag) SHALL mostrare in forma compatta le voci con più foto, il numero totale di voci e un comando per vederle tutte. Ogni voce esistente nei risultati SHALL essere raggiungibile e selezionabile, senza limiti fissi al numero di voci, con ricerca testuale e ordinamento per numero di foto o alfabetico.

#### Scenario: Molte località
- **WHEN** la libreria contiene foto in 340 luoghi diversi
- **THEN** la lista Luoghi mostra le voci principali e "Mostra tutti (340)", e dall'elenco completo è possibile selezionare il luogo con meno foto

#### Scenario: Ricerca nell'elenco
- **WHEN** l'utente apre l'elenco completo dei luoghi e digita "sie"
- **THEN** compaiono solo i luoghi il cui nome contiene "sie" (senza distinzione di maiuscole e accenti), con i rispettivi conteggi

#### Scenario: Ordinamento alfabetico
- **WHEN** l'utente sceglie l'ordinamento alfabetico nell'elenco completo dei tag
- **THEN** i tag sono elencati in ordine alfabetico invece che per numero di foto

#### Scenario: Voce selezionata sempre visibile
- **WHEN** l'utente ha selezionato un luogo che non è tra le voci principali e richiude l'elenco completo
- **THEN** il luogo selezionato resta visibile e spuntato nella lista compatta

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 foto e 5.000 tag e l'utente apre l'elenco completo dei tag
- **THEN** le prime voci compaiono entro 1 secondo e lo scorrimento dell'elenco resta fluido

### Requirement: Conteggi dei filtri indipendenti dalla propria selezione
Le voci e i conteggi di una lista di filtro SHALL essere calcolati applicando tutti gli altri filtri attivi ma non la selezione della lista stessa, così che le altre voci della stessa lista restino visibili e selezionabili.

#### Scenario: Selezione di più luoghi
- **WHEN** l'utente seleziona il luogo "Siena" e la lista Luoghi conteneva anche "Firenze" con 80 foto
- **THEN** "Firenze" resta nella lista con 80 foto e può essere aggiunto alla selezione

#### Scenario: Altri filtri applicati
- **WHEN** è attivo il filtro "Esterno" e l'utente guarda la lista Luoghi
- **THEN** i conteggi dei luoghi considerano solo le foto esterne

### Requirement: Ordinamento per istante reale
La libreria e i cataloghi SHALL poter essere ordinati per istante reale di scatto (dopo le correzioni di fuso e scostamento) e il filtro per intervallo di date SHALL usare l'ora locale corretta di ciascuna foto.

#### Scenario: Ordinamento dopo una correzione
- **WHEN** l'utente applica uno scostamento di +2 minuti a un gruppo di sorgente e ordina la libreria per data di scatto
- **THEN** le foto di quel gruppo occupano la posizione corrispondente all'istante corretto
