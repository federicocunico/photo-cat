# diagnostics-logging Specification

## Purpose
Raccoglie in modo strutturato log, errori e avanzamento di tutte le fasi del programma, li rende consultabili e filtrabili nell'app ed esportabili in un pacchetto diagnostico per l'analisi dei problemi.

## Requirements

### Requirement: Log su file con rotazione
Il sistema SHALL scrivere log strutturati (data/ora, livello, componente, job, fase, messaggio, contesto) in file locali nella cartella dati dell'app, con rotazione per dimensione o giorno e conservazione limitata configurabile.

#### Scenario: Rotazione
- **WHEN** il file di log corrente supera la dimensione massima
- **THEN** viene creato un nuovo file e i più vecchi oltre il limite di conservazione vengono eliminati

### Requirement: Livello di log configurabile
L'utente SHALL poter scegliere il livello di dettaglio dei log (errore, avviso, info, debug) dalle impostazioni, con effetto immediato.

#### Scenario: Attivazione debug
- **WHEN** l'utente imposta il livello "debug"
- **THEN** i nuovi eventi di debug compaiono nei log senza riavviare l'app

### Requirement: Registro errori strutturato
Ogni errore o avviso relativo a un file o a un'operazione SHALL essere registrato in un registro persistente con codice errore, messaggio leggibile, fase, job, percorso file (se applicabile), dettaglio tecnico e data/ora.

#### Scenario: Errore di decodifica
- **WHEN** la decodifica di un'immagine fallisce durante le miniature
- **THEN** il registro contiene una voce con fase "miniature", percorso del file, codice e messaggio

### Requirement: Messaggi comprensibili
Ogni errore mostrato all'utente SHALL avere un messaggio in linguaggio semplice e, quando possibile, un'azione suggerita, mantenendo il dettaglio tecnico consultabile separatamente.

#### Scenario: Accesso negato
- **WHEN** una cartella non è leggibile per permessi
- **THEN** l'utente vede "Accesso negato alla cartella" con il suggerimento di verificare i permessi, e il dettaglio tecnico è disponibile espandendo la voce

### Requirement: Vista diagnostica
L'interfaccia SHALL offrire una schermata di diagnostica con il registro errori filtrabile per livello, fase, job e testo, il dettaglio di ogni voce e la coda dei log recenti.

#### Scenario: Filtro per job
- **WHEN** l'utente apre gli errori dal riepilogo di un job
- **THEN** la diagnostica mostra solo gli errori di quel job

### Requirement: Riprova dei file falliti
Dalla diagnostica l'utente SHALL poter rilanciare l'elaborazione dei soli file falliti, per fase o per job.

#### Scenario: Riprova
- **WHEN** l'utente sceglie "Riprova file falliti" per un job con 5 errori
- **THEN** viene creato un job che rielabora solo quei 5 file

### Requirement: Pacchetto diagnostico
L'utente SHALL poter esportare un pacchetto diagnostico (archivio con log recenti, registro errori, informazioni di sistema, versioni di app e modelli e stato dei job) che MUST NOT contenere immagini, miniature o dati dei volti.

#### Scenario: Esportazione
- **WHEN** l'utente sceglie "Esporta pacchetto diagnostica"
- **THEN** viene creato un archivio con i contenuti indicati e senza immagini né dati biometrici

### Requirement: Cattura dei crash
Il sistema SHALL registrare nei log gli errori fatali e i panic prima della chiusura e, al riavvio successivo, segnalare all'utente che la sessione precedente si è chiusa in modo anomalo.

#### Scenario: Panic
- **WHEN** un componente va in panic
- **THEN** il log contiene messaggio e posizione del panic e al riavvio l'utente vede l'avviso con collegamento alla diagnostica

### Requirement: Storico del progresso
Il sistema SHALL registrare nei log inizio, fine, durata, conteggi e percentuale finale di ogni fase di ogni job.

#### Scenario: Riepilogo fase
- **WHEN** termina la fase "miniature" di un job
- **THEN** il log contiene durata, elementi completati, falliti e percentuale finale di quella fase

### Requirement: Apertura della cartella dei log
Dalla diagnostica l'utente SHALL poter aprire la cartella dei log nel file manager predefinito del sistema su Windows, Linux e macOS. Se l'apertura non è possibile, l'utente SHALL vedere un messaggio che spiega cosa non ha funzionato e indica il percorso della cartella.

#### Scenario: Apertura su Linux
- **WHEN** su Linux l'utente sceglie "Apri cartella log"
- **THEN** la cartella dei log si apre nel file manager predefinito del desktop

#### Scenario: Nessun file manager disponibile
- **WHEN** il sistema non ha un programma in grado di aprire cartelle
- **THEN** l'utente vede un messaggio che indica che non è stato possibile aprire la cartella e ne mostra il percorso
