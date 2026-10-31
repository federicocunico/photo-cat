# Spec Delta

## ADDED Requirements

### Requirement: Runtime AI per piattaforma
Il sistema SHALL distribuire e caricare la libreria ONNX Runtime adatta al sistema operativo e all'architettura in uso (almeno Windows x64 e Linux x64), sia in sviluppo sia nella build distribuita, senza mai tentare di caricare la libreria di un'altra piattaforma.

#### Scenario: Avvio su Linux
- **WHEN** l'app si avvia su Linux x64 con gli asset del repository o della build
- **THEN** il runtime ONNX per Linux viene caricato e i modelli superano il self-test su CPU

#### Scenario: Runtime assente per la piattaforma
- **WHEN** gli asset non contengono un runtime ONNX per la piattaforma in uso
- **THEN** le analisi AI sono disattivate, la schermata Modelli AI e la diagnostica indicano che il runtime non è disponibile per questo sistema, e il resto dell'app funziona

### Requirement: Errore del runtime senza blocco
Un errore nel caricamento del runtime ONNX (file mancante, non caricabile, corrotto o di versione non supportata) SHALL disattivare solo le funzioni AI, registrando il motivo, e non SHALL mai impedire o ritardare l'apertura dell'interfaccia.

#### Scenario: Libreria non caricabile
- **WHEN** il file del runtime esiste ma il sistema operativo non riesce a caricarlo
- **THEN** l'interfaccia si apre normalmente e la diagnostica riporta il percorso e il motivo del fallimento

#### Scenario: Versione insufficiente
- **WHEN** il runtime trovato è di una versione inferiore a quella richiesta
- **THEN** le analisi AI sono disattivate e il messaggio indica la versione trovata e quella minima richiesta

### Requirement: Provider di esecuzione su Linux
Su Linux il sistema SHALL eseguire le analisi su CPU. Un provider GPU SHALL essere selezionabile solo se il runtime distribuito lo include ed è utilizzabile sulla macchina; gli altri SHALL restare visibili ma non selezionabili, ciascuno con il motivo indicato nella propria voce (solo su un'altra piattaforma, non incluso nel runtime, runtime non caricato).

#### Scenario: Scelta del provider su Linux
- **WHEN** l'utente apre la scelta del provider di esecuzione su Linux senza GPU supportate
- **THEN** sono selezionabili solo Automatico e CPU, e le voci DirectML e CoreML sono disattivate e indicano che sono disponibili solo su Windows e su macOS

#### Scenario: Runtime non caricato
- **WHEN** ONNX Runtime non è caricato
- **THEN** le voci dei provider GPU sono disattivate e indicano che il runtime non è caricato, e i self-test dei modelli ONNX vengono rimandati senza registrare un esito fallito

#### Scenario: Preferenza GPU importata da Windows
- **WHEN** la configurazione indica DirectML come provider preferito e l'app gira su Linux
- **THEN** le analisi girano su CPU e la diagnostica registra il ripiego
