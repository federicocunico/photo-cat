# Spec Delta

## Purpose

Collega photo-cat alle applicazioni di gestione o fotoritocco che l'utente usa già, permettendo di aprire i file della libreria con esse dal doppio clic o dal menu "Apri con", senza mai modificare i file da parte di photo-cat.

## ADDED Requirements

### Requirement: Configurazione delle applicazioni esterne
L'utente SHALL poter configurare nelle impostazioni un elenco di applicazioni esterne, ciascuna con nome visualizzato, percorso dell'eseguibile e argomenti opzionali in cui `{file}` indica il percorso della foto; il sistema SHALL verificare che l'eseguibile esista al salvataggio. L'elenco SHALL persistere tra i riavvii.

#### Scenario: Aggiunta di un'applicazione
- **WHEN** l'utente aggiunge "Lightroom" indicando l'eseguibile di Lightroom e salva
- **THEN** "Lightroom" compare nell'elenco delle applicazioni esterne e resta disponibile dopo il riavvio

#### Scenario: Eseguibile inesistente
- **WHEN** l'utente indica un percorso che non esiste
- **THEN** il salvataggio viene rifiutato con un messaggio che indica il percorso non valido

#### Scenario: Argomenti personalizzati
- **WHEN** l'applicazione ha argomenti `--edit {file}` e l'utente vi apre `D:\Foto\a.jpg`
- **THEN** l'applicazione viene avviata con gli argomenti `--edit` e `D:\Foto\a.jpg`

### Requirement: Azione del doppio clic
L'utente SHALL poter scegliere nelle impostazioni cosa fa il doppio clic su una miniatura: aprire la foto con una delle applicazioni esterne configurate, aprirla con l'applicazione predefinita del sistema oppure nessuna azione. Il valore predefinito SHALL essere l'applicazione predefinita del sistema.

#### Scenario: Doppio clic con applicazione configurata
- **WHEN** l'azione del doppio clic è "Lightroom" e l'utente fa doppio clic su una miniatura
- **THEN** la foto viene aperta in Lightroom

#### Scenario: Doppio clic predefinito
- **WHEN** l'utente non ha configurato applicazioni e fa doppio clic su una miniatura
- **THEN** la foto viene aperta con l'applicazione predefinita del sistema per quel formato

### Requirement: Apri con
Dal menu contestuale di una miniatura l'utente SHALL poter aprire la foto, o tutte le foto selezionate, con una qualsiasi applicazione esterna configurata, con l'applicazione predefinita del sistema, oppure mostrarla nella cartella del file manager.

#### Scenario: Apri con su selezione multipla
- **WHEN** l'utente seleziona 5 foto e sceglie "Apri con ▸ Lightroom"
- **THEN** le 5 foto vengono passate a Lightroom

#### Scenario: Mostra nella cartella
- **WHEN** l'utente sceglie "Mostra nella cartella" su una foto
- **THEN** il file manager del sistema si apre sulla cartella della foto con il file evidenziato dove il sistema lo consente

### Requirement: Apertura sicura e segnalazione degli errori
L'apertura con un'applicazione esterna MUST NOT bloccare l'interfaccia né modificare il file; i percorsi SHALL essere passati come argomenti distinti senza interpretazione da parte di una shell. Se il file è mancante o l'applicazione non si avvia, il sistema SHALL mostrare un errore comprensibile e registrarlo in diagnostica.

#### Scenario: File mancante
- **WHEN** l'utente prova ad aprire una foto marcata "mancante"
- **THEN** l'applicazione non viene avviata e viene mostrato che il file non è presente sul disco

#### Scenario: Applicazione rimossa
- **WHEN** l'eseguibile configurato è stato disinstallato e l'utente usa "Apri con" su di esso
- **THEN** viene mostrato un errore che indica l'applicazione non trovata e l'evento compare in diagnostica

#### Scenario: Nomi di file con caratteri speciali
- **WHEN** il percorso della foto contiene spazi, `&` o virgolette
- **THEN** l'applicazione riceve il percorso esatto come un unico argomento
