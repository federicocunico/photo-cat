# Spec Delta

## Purpose

Conserva le persone note all'utente in un archivio dell'app, indipendente da libreria e cataloghi, così che nomi e riconoscimento valgano per ogni libreria e sopravvivano a cambi di cartella dati.

## ADDED Requirements

### Requirement: Archivio identità a livello di app
Il sistema SHALL conservare le identità in un archivio dell'app separato dalla libreria, nella cartella di configurazione dell'utente; l'archivio MUST restare invariato quando l'utente cambia cartella dati, svuota o sostituisce la libreria.

#### Scenario: Cambio cartella dati
- **WHEN** l'utente ha 14 identità e sposta la cartella dati su un altro disco con una libreria nuova
- **THEN** la schermata Identità mostra le stesse 14 identità

#### Scenario: Libreria rimossa
- **WHEN** l'utente rimuove tutti i file dalla libreria
- **THEN** le identità e i loro volti di riferimento restano

### Requirement: Dati di un'identità
Ogni identità SHALL avere un nome visualizzato obbligatorio e univoco, un nome breve per i nomi dei file, alias facoltativi, un gruppo facoltativo e una soglia di somiglianza propria con un valore predefinito.

#### Scenario: Nome duplicato
- **WHEN** l'utente crea un'identità "Giulia Bianchi" e ne esiste già una con lo stesso nome
- **THEN** il sistema rifiuta il salvataggio indicando il conflitto

#### Scenario: Nome breve predefinito
- **WHEN** l'utente crea "Giulia Bianchi" senza indicare il nome breve
- **THEN** il nome breve è "Giulia"

### Requirement: Gestione delle identità
L'utente SHALL poter creare, modificare, unire ed eliminare identità da una schermata dedicata con filtro per gruppo e ricerca su nome e alias.

#### Scenario: Ricerca per alias
- **WHEN** l'utente cerca "Nonna Rita"
- **THEN** compare l'identità "Rita Rossi" che ha quell'alias

#### Scenario: Unione di identità
- **WHEN** l'utente unisce "G. Bianchi" in "Giulia Bianchi"
- **THEN** volti di riferimento, alias e collegamenti di "G. Bianchi" passano a "Giulia Bianchi" e "G. Bianchi" non esiste più

### Requirement: Volti di riferimento
L'utente SHALL poter aggiungere a un'identità volti di riferimento scelti tra i volti della libreria o da un file immagine, e rimuoverli; per ogni volto di riferimento l'archivio conserva il ritaglio e l'embedding, non la foto originale.

#### Scenario: Riferimento da file
- **WHEN** l'utente aggiunge a "Giulia Bianchi" un'immagine con un solo volto
- **THEN** l'identità ha un volto di riferimento in più e il file scelto non viene copiato nell'archivio

#### Scenario: Immagine con più volti
- **WHEN** l'utente aggiunge come riferimento un'immagine con tre volti
- **THEN** il sistema chiede quale dei tre volti usare

### Requirement: Collegamento con le persone della libreria
In ogni libreria una persona (gruppo di volti) SHALL poter essere collegata a un'identità; il nome mostrato della persona è quello dell'identità e la schermata Identità indica per ogni identità il numero di foto nella libreria aperta o "non collegata".

#### Scenario: Collegamento
- **WHEN** l'utente collega il gruppo "Senza nome 12" all'identità "Marco Russo"
- **THEN** il gruppo appare come "Marco Russo" in Persone e l'identità mostra il numero di foto del gruppo

### Requirement: Esportazione e importazione delle identità
L'utente SHALL poter esportare le identità in un file e importarle su un'altra installazione, scegliendo se includere i volti di riferimento; all'importazione le identità con lo stesso nome SHALL essere proposte per l'unione invece di essere duplicate.

#### Scenario: Esportazione senza dati biometrici
- **WHEN** l'utente esporta le identità senza volti di riferimento
- **THEN** il file contiene nomi, alias e gruppi e nessun ritaglio o embedding

#### Scenario: Importazione con nomi già presenti
- **WHEN** l'utente importa un file con "Giulia Bianchi", già presente
- **THEN** il sistema propone di unire le due identità e non ne crea una seconda

### Requirement: Migrazione delle persone con nome
Alla prima apertura di una libreria dopo l'aggiornamento, ogni persona con nome SHALL essere collegata a un'identità con lo stesso nome, creandola se non esiste.

#### Scenario: Libreria esistente
- **WHEN** una libreria ha le persone con nome "Giulia" e "Luca" e l'archivio identità è vuoto
- **THEN** dopo l'aggiornamento esistono le identità "Giulia" e "Luca", collegate alle rispettive persone
