# Spec Delta

## ADDED Requirements

### Requirement: Esecuzione su Linux
L'applicazione SHALL avviarsi e funzionare su Linux x86_64 con un desktop grafico (Wayland o X11), sia in sviluppo sia dalla build distribuita, con le stesse funzioni di Windows salvo quelle dichiarate specifiche di una piattaforma.

#### Scenario: Avvio in sviluppo
- **WHEN** su Linux, con le dipendenze di sistema documentate installate, lo sviluppatore esegue l'avvio locale in debug
- **THEN** la finestra principale si apre e diventa interattiva senza variabili d'ambiente aggiuntive

#### Scenario: Avvio dall'archivio
- **WHEN** l'utente estrae l'archivio Linux di una release e avvia l'eseguibile
- **THEN** l'app si apre, trova le proprie risorse (modelli, runtime, dati di riferimento) e usa la cartella dati utente standard di Linux

#### Scenario: Installazione del pacchetto deb
- **WHEN** l'utente installa il pacchetto deb di una release su Ubuntu o Debian
- **THEN** le dipendenze di sistema necessarie vengono installate dal gestore pacchetti, photo-cat compare nel menu delle applicazioni e si avvia trovando le proprie risorse

### Requirement: Verifica Linux nella pipeline
Ogni push e pull request SHALL compilare e verificare l'app anche su Linux, con controllo di formattazione, analisi statica, test Rust e build di release, oltre alla build Windows già prevista.

#### Scenario: Regressione solo su Linux
- **WHEN** una modifica compila su Windows ma non su Linux
- **THEN** la pipeline fallisce sul job Linux

### Requirement: Pacchetti Linux nelle release
Su un tag di versione valido, la release SHALL includere, oltre all'installer Windows, un pacchetto deb per Linux x64 e un archivio Linux x64 con l'eseguibile e le risorse necessarie, ciascuno con il relativo checksum SHA-256, senza fotografie né dati personali.

#### Scenario: Tag di versione
- **WHEN** viene pubblicato un tag `v<versione>` valido
- **THEN** la release contiene `photo-cat_<versione>_amd64.deb` e `photo-cat-<versione>-linux-x64.tar.gz`, ciascuno con un file `.sha256` verificabile con `sha256sum -c`
