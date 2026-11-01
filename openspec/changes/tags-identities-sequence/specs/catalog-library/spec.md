# Spec Delta

## MODIFIED Requirements

### Requirement: Tag utente
L'utente SHALL poter assegnare e rimuovere tag liberi ai file; i tag utente SHALL essere distinti dalle catalogazioni automatiche e non essere mai sovrascritti da un'analisi. Ogni tag SHALL essere un'entità con nome univoco (senza distinzione di maiuscole) e colore, così che rinominarlo o unirlo a un altro mantenga le assegnazioni ai file.

#### Scenario: Rianalisi non tocca i tag
- **WHEN** un file con tag utente viene rianalizzato
- **THEN** i suoi tag utente restano invariati

#### Scenario: Stesso tag con maiuscole diverse
- **WHEN** l'utente aggiunge il tag "matrimonio" e in libreria esiste già "Matrimonio"
- **THEN** il file riceve il tag esistente "Matrimonio" e non ne viene creato uno nuovo

## ADDED Requirements

### Requirement: Ordinamento per istante reale
La libreria e i cataloghi SHALL poter essere ordinati per istante reale di scatto (dopo le correzioni di fuso e scostamento) e il filtro per intervallo di date SHALL usare l'ora locale corretta di ciascuna foto.

#### Scenario: Ordinamento dopo una correzione
- **WHEN** l'utente applica uno scostamento di +2 minuti a un gruppo di sorgente e ordina la libreria per data di scatto
- **THEN** le foto di quel gruppo occupano la posizione corrispondente all'istante corretto
