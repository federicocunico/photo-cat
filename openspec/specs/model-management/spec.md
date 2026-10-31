# model-management Specification

## Purpose
Garantisce che i modelli neurali necessari siano presenti, integri e funzionanti: li scarica automaticamente, ne verifica l'integrità, esegue un self-test e sceglie il provider di esecuzione (CPU di default, GPU se disponibile).

## Requirements

### Requirement: Manifest dei modelli
Il sistema SHALL descrivere i modelli richiesti in un manifest versionato con, per ciascun modello, nome, versione, ruolo, URL di download (con eventuali mirror), dimensione, hash SHA-256 e memoria stimata a runtime.

#### Scenario: Nuova versione del manifest
- **WHEN** un aggiornamento dell'app porta un manifest con una versione diversa di un modello
- **THEN** il modello viene segnalato come "aggiornamento disponibile" e scaricato secondo la politica di download

### Requirement: Verifica e auto-download all'avvio
All'avvio il sistema SHALL verificare la presenza e l'integrità dei modelli e scaricare automaticamente quelli mancanti o corrotti, in background e senza bloccare l'uso della libreria.

#### Scenario: Primo avvio
- **WHEN** l'app viene avviata per la prima volta
- **THEN** i modelli vengono scaricati mostrando percentuale, dimensione e velocità per ciascuno, e la libreria è comunque navigabile

#### Scenario: File modello corrotto
- **WHEN** l'hash di un modello su disco non corrisponde al manifest
- **THEN** il file viene scartato, riscaricato e l'evento è registrato in diagnostica

### Requirement: Download robusto
I download SHALL essere riprendibili dopo un'interruzione, scritti su file temporanei e resi definitivi solo dopo la verifica dell'hash; in caso di errore SHALL essere ritentati con backoff e poi provati sui mirror.

#### Scenario: Connessione interrotta
- **WHEN** la connessione cade al 70% del download di un modello
- **THEN** al ritorno della connessione il download riprende dal punto raggiunto

#### Scenario: Download fallito definitivamente
- **WHEN** tutti i tentativi e i mirror falliscono
- **THEN** il modello è marcato "Errore", le funzioni che lo richiedono sono disabilitate con un messaggio chiaro e l'utente può ritentare manualmente

### Requirement: Installazione offline
L'utente SHALL poter installare un modello da un file locale; il file SHALL essere accettato solo se il suo hash corrisponde al manifest.

#### Scenario: File locale valido
- **WHEN** l'utente seleziona un file modello con hash corretto
- **THEN** il modello risulta "Verificato" senza download

### Requirement: Self-test di funzionamento
Dopo l'installazione e su richiesta il sistema SHALL eseguire un self-test di ciascun modello su input di riferimento inclusi nell'app, verificando che l'output sia nei limiti attesi e misurando il tempo di inferenza.

#### Scenario: Self-test superato
- **WHEN** l'utente avvia "Test" sul modello di classificazione
- **THEN** viene mostrato "Superato" con il tempo medio di inferenza

#### Scenario: Self-test fallito
- **WHEN** l'output del modello sugli input di riferimento è fuori dai limiti
- **THEN** il modello è marcato "Test fallito", le analisi che lo usano non vengono avviate e il dettaglio è in diagnostica

### Requirement: Provider di esecuzione
Il sistema SHALL funzionare su sola CPU e SHALL rilevare acceleratori disponibili (es. DirectML, CUDA, CoreML), permettendo all'utente di scegliere il provider; se il provider scelto fallisce SHALL ripiegare sulla CPU registrando l'evento.

#### Scenario: GPU non disponibile
- **WHEN** il provider GPU selezionato non si inizializza
- **THEN** l'analisi prosegue su CPU e la diagnostica riporta il motivo

### Requirement: Stato dei modelli visibile
L'interfaccia SHALL mostrare per ogni modello versione, dimensione, stato (non presente, download %, verificato, errore, test fallito, aggiornamento disponibile) e risultato dell'ultimo self-test.

#### Scenario: Consultazione stato
- **WHEN** l'utente apre la schermata Modelli AI
- **THEN** vede per ciascun modello stato, versione, dimensione e ultimo self-test
