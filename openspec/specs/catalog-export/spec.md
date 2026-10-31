# catalog-export Specification

## Purpose
Produce il file strutturato di catalogazione richiesto dal prodotto (percorso, nome, caratteristiche tecniche, catalogazioni) per l'intera libreria, un catalogo o una selezione, in formati leggibili da altri strumenti.

## Requirements

### Requirement: File strutturato di catalogazione
Il sistema SHALL esportare, per ogni file nell'ambito scelto, almeno: percorso completo, nome file, formato, larghezza, altezza, DPI orizzontale e verticale, profondità in bit, dimensione, data/ora di acquisizione, data/ora di ultima modifica, luogo e fonte, presenza e numero di persone, nomi delle persone, tipo soggetto, locazione, tag e cataloghi.

#### Scenario: Export della libreria
- **WHEN** l'utente esporta l'intera libreria
- **THEN** il file contiene una riga/record per ogni file della libreria con tutti i campi indicati

### Requirement: Formati di export
L'export SHALL essere disponibile in CSV (UTF-8, intestazione, separatore configurabile) e JSON; i campi multi-valore SHALL essere rappresentati in modo non ambiguo in entrambi i formati.

#### Scenario: CSV apribile in un foglio di calcolo
- **WHEN** l'utente esporta in CSV con separatore ";"
- **THEN** il file si apre correttamente in un foglio di calcolo con caratteri accentati leggibili

#### Scenario: Multi-valore in JSON
- **WHEN** un file ha tre persone
- **THEN** in JSON le persone sono un array di tre elementi

### Requirement: Ambito e campi selezionabili
L'utente SHALL poter scegliere l'ambito (libreria, catalogo, selezione, risultati del filtro) e i campi da includere, inclusa la confidenza delle catalogazioni AI.

#### Scenario: Export di un catalogo
- **WHEN** l'utente esporta il catalogo "Ritratti 2023" con soli percorso, data e persone
- **THEN** il file contiene solo i file del catalogo e solo quei campi

### Requirement: Schema versionato
L'export SHALL includere la versione dello schema di export così che strumenti esterni possano gestire l'aggiunta di nuovi campi nelle versioni future.

#### Scenario: Versione nel JSON
- **WHEN** l'utente esporta in JSON
- **THEN** il documento riporta la versione dello schema e la data di generazione

### Requirement: Export come job
L'export SHALL essere eseguito come job con progresso e SHALL scrivere il file in modo atomico, senza lasciare file parziali in caso di errore o annullamento.

#### Scenario: Annullamento export
- **WHEN** l'utente annulla un export in corso
- **THEN** nella destinazione non resta alcun file parziale
