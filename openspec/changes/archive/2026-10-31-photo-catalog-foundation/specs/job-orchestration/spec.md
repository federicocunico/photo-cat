# Spec Delta

## Purpose

Esegue i lavori lunghi (import, analisi, export, organizzazione, download modelli) come job a fasi concorrenti, con percentuali di avanzamento per fase e complessive, ETA e controllo pausa/annulla/ripresa.

## ADDED Requirements

### Requirement: Job a fasi
Ogni operazione lunga SHALL essere eseguita come job composto da fasi nominate (per l'import: scansione, metadati, miniature, analisi AI, volti, scrittura catalogo); le fasi SHALL procedere in pipeline, così che un file possa essere in una fase mentre altri sono in fasi diverse.

#### Scenario: Pipeline sovrapposta
- **WHEN** un import di 5.000 file è in corso
- **THEN** l'estrazione metadati inizia prima che la scansione abbia terminato

### Requirement: Progresso percentuale per fase e complessivo
Per ogni job il sistema SHALL esporre, per ciascuna fase, elementi completati, totali noti, falliti e percentuale, oltre a una percentuale complessiva pesata e uno stato; durante la scansione, quando il totale non è ancora noto, la fase SHALL indicarlo come indeterminato mostrando il conteggio corrente.

#### Scenario: Avanzamento visibile
- **WHEN** l'analisi AI ha completato 620 file su 1.000
- **THEN** la fase "Analisi AI" mostra 62% e 620/1.000

#### Scenario: Totale sconosciuto
- **WHEN** la scansione sta ancora contando i file
- **THEN** la fase mostra un indicatore indeterminato con il numero di file trovati finora

### Requirement: Velocità ed ETA
Il sistema SHALL calcolare per ogni fase attiva la velocità (elementi al secondo) e un tempo stimato di completamento per fase e per job.

#### Scenario: ETA mostrata
- **WHEN** una fase è attiva da almeno 10 secondi
- **THEN** la UI mostra velocità e tempo residuo stimato

### Requirement: Aggiornamenti di progresso a frequenza limitata
Gli aggiornamenti di progresso verso l'interfaccia SHALL essere aggregati e inviati con frequenza limitata (non più di circa 10 al secondo per job) per non rallentare l'elaborazione.

#### Scenario: Job molto veloce
- **WHEN** una fase elabora migliaia di file al secondo
- **THEN** l'interfaccia riceve aggiornamenti aggregati senza blocchi né rallentamenti

### Requirement: Pausa, ripresa e annullamento
L'utente SHALL poter mettere in pausa, riprendere e annullare un job; l'annullamento SHALL fermare i lavori in modo ordinato lasciando il catalogo coerente con i file già completati.

#### Scenario: Annullamento
- **WHEN** l'utente annulla un'analisi al 40%
- **THEN** i risultati dei file già completati restano salvati e il job è marcato "Annullato"

### Requirement: Persistenza e ripresa dei job
Lo stato dei job SHALL essere persistito; i job interrotti dalla chiusura dell'app o da un crash SHALL essere proposti per la ripresa al successivo avvio.

#### Scenario: Crash durante import
- **WHEN** l'app si chiude inaspettatamente durante un import
- **THEN** al riavvio il job compare come "Interrotto" con l'opzione "Riprendi", che elabora solo i file mancanti

### Requirement: Errori per file non bloccanti
Un errore su un singolo file SHALL essere registrato, conteggiato nella fase come fallito e non SHALL interrompere il job; il job termina come "Completato con errori" quando almeno un file è fallito.

#### Scenario: File illeggibile
- **WHEN** uno dei 1.000 file di un import non è leggibile
- **THEN** il job elabora gli altri 999 e termina "Completato con errori" con 1 fallito e un collegamento all'errore

### Requirement: Coda dei job
I job SHALL essere accodati e la concorrenza tra job SHALL rispettare il budget di risorse; l'utente SHALL vedere i job in coda, attivi e conclusi di recente.

#### Scenario: Due import ravvicinati
- **WHEN** l'utente avvia un secondo import mentre il primo è in analisi AI
- **THEN** il secondo job viene accodato o eseguito in parallelo solo nelle fasi che rientrano nel budget, ed entrambi sono visibili nella lista job
