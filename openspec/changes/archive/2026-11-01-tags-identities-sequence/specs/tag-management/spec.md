# Spec Delta

## Purpose

Rende i tag dell'utente entità gestibili da un'unica schermata e li organizza in set con tasti rapidi, per assegnarli velocemente e in modo coerente, anche come categoria esclusiva per l'export.

## ADDED Requirements

### Requirement: Elenco dei tag
Il sistema SHALL mostrare in una schermata dedicata tutti i tag della libreria con il numero di file a cui sono assegnati e i set a cui appartengono, con filtro per testo.

#### Scenario: Conteggi dei tag
- **WHEN** il tag "Tramonti" è assegnato a 38 file e appartiene ai set "Base" e "Viaggio USA 2026"
- **THEN** la schermata Tag mostra "Tramonti" con 38 file e i due set

#### Scenario: Apertura dei file di un tag
- **WHEN** l'utente sceglie "Mostra le foto" su un tag
- **THEN** la Libreria si apre filtrata su quel tag

### Requirement: Rinomina di un tag
L'utente SHALL poter rinominare un tag; tutte le assegnazioni e le appartenenze ai set SHALL restare invariate. Se il nuovo nome coincide (senza distinzione di maiuscole) con un tag esistente, il sistema SHALL proporre l'unione invece di rinominare.

#### Scenario: Rinomina
- **WHEN** l'utente rinomina "Tramonto" in "Tramonti" e non esiste già "Tramonti"
- **THEN** tutti i file che avevano "Tramonto" hanno ora "Tramonti" e i set lo elencano con il nuovo nome

#### Scenario: Nome già esistente
- **WHEN** l'utente rinomina "tramonto" in "Tramonti" ed esiste già "Tramonti"
- **THEN** il sistema chiede conferma per unire i due tag invece di creare un duplicato

### Requirement: Unione di tag
L'utente SHALL poter unire un tag in un altro: i file del tag sorgente ricevono il tag di destinazione, il tag sorgente viene eliminato e i set che lo contenevano contengono il tag di destinazione.

#### Scenario: Unione
- **WHEN** l'utente unisce "Sunset" (12 file) in "Tramonti" (38 file, 4 in comune)
- **THEN** "Tramonti" ha 46 file e "Sunset" non esiste più

### Requirement: Eliminazione di un tag
L'utente SHALL poter eliminare un tag dopo una conferma che indica il numero di file coinvolti; l'eliminazione rimuove il tag dai file e dai set e non tocca i file su disco.

#### Scenario: Eliminazione confermata
- **WHEN** l'utente elimina il tag "Scartate" assegnato a 410 file
- **THEN** la conferma indica 410 file e, dopo conferma, nessun file ha più quel tag

### Requirement: Annullamento delle modifiche ai tag
Rinomina, unione ed eliminazione di un tag SHALL poter essere annullate con "Annulla" come le altre azioni bulk sui metadati.

#### Scenario: Annulla unione
- **WHEN** l'utente unisce "Sunset" in "Tramonti" e poi sceglie "Annulla"
- **THEN** "Sunset" esiste di nuovo con i suoi 12 file e "Tramonti" torna a 38 file

### Requirement: Colore dei tag
Ogni tag SHALL avere un colore scelto da una palette fissa, usato nei chip dei tag in Libreria, Inspector e Sequenza; i tag nuovi ricevono un colore predefinito.

#### Scenario: Cambio colore
- **WHEN** l'utente imposta il colore giallo al tag "Tramonti"
- **THEN** i chip "Tramonti" appaiono gialli in Libreria e in Sequenza

### Requirement: Set di tag
L'utente SHALL poter creare, rinominare, duplicare ed eliminare set di tag: elenchi ordinati di tag in cui ogni tag occupa una posizione da 1 a 9 che corrisponde al suo tasto rapido. Eliminare un set non elimina i tag né le assegnazioni.

#### Scenario: Creazione di un set
- **WHEN** l'utente crea il set "Viaggio USA 2026" con Paesaggi, Città, Tramonti, Persone, Cibo
- **THEN** i cinque tag hanno i tasti da 1 a 5 nell'ordine indicato

#### Scenario: Limite dei tasti
- **WHEN** un set contiene già 9 tag e l'utente prova ad aggiungerne un decimo
- **THEN** il sistema rifiuta l'aggiunta indicando il limite di 9 tag per set

#### Scenario: Eliminazione di un set
- **WHEN** l'utente elimina il set "Eventi"
- **THEN** i tag del set e le loro assegnazioni restano invariati

### Requirement: Set esclusivo
Un set SHALL poter essere marcato esclusivo: in quel caso ogni file può avere al massimo un tag del set, e assegnare un tag del set a un file sostituisce l'eventuale altro tag dello stesso set.

#### Scenario: Sostituzione nel set esclusivo
- **WHEN** il set "Viaggio USA 2026" è esclusivo, una foto ha "Paesaggi" e l'utente le assegna "Tramonti"
- **THEN** la foto ha "Tramonti" e non ha più "Paesaggi"

#### Scenario: Attivazione dell'esclusività con conflitti
- **WHEN** l'utente rende esclusivo un set e 6 file hanno già due tag del set
- **THEN** il sistema segnala i 6 file in conflitto e permette di aprirli in Libreria, senza rimuovere tag automaticamente

### Requirement: Set Base a livello di app
Il sistema SHALL fornire un set "Base" e conservare i set di tag a livello di app, indipendentemente dalla libreria: aprendo una libreria nuova o diversa, i set e i nomi dei loro tag sono disponibili e i tag mancanti vengono creati al primo uso.

#### Scenario: Libreria nuova
- **WHEN** l'utente cambia cartella dati e apre una libreria vuota
- **THEN** la schermata Tag mostra il set "Base" e gli altri set già definiti

### Requirement: Set attivo e tagging da tastiera
L'utente SHALL poter scegliere un set attivo; in Libreria e in Sequenza i tasti 1–9 assegnano alla selezione il tag in quella posizione del set attivo e il tasto 0 rimuove dalla selezione i tag del set attivo. I tasti non agiscono mentre il fuoco è in un campo di testo.

#### Scenario: Assegnazione con tasto
- **WHEN** il set attivo è "Viaggio USA 2026", 6 foto sono selezionate e l'utente preme 3
- **THEN** le 6 foto ricevono "Tramonti"

#### Scenario: Rimozione con 0
- **WHEN** 6 foto selezionate hanno tag del set attivo e l'utente preme 0
- **THEN** i tag del set attivo sono rimossi dalle 6 foto e gli altri tag restano

#### Scenario: Fuoco in un campo di testo
- **WHEN** l'utente sta scrivendo nella casella di ricerca e preme 3
- **THEN** il carattere viene scritto nella casella e nessun tag viene assegnato
