# Spec Delta

## MODIFIED Requirements

### Requirement: Sorgenti di import
L'utente SHALL poter aggiungere una o più cartelle sorgente, con opzioni di scansione ricorsiva, formati inclusi e pattern da ignorare; le sorgenti SHALL essere persistite e rimovibili. Aggiornamento e rimozione di una sorgente SHALL essere disponibili sia dalla pagina Import sia dalla barra laterale della Libreria.

#### Scenario: Aggiunta sorgente ricorsiva
- **WHEN** l'utente aggiunge `D:\Foto` con scansione ricorsiva attiva
- **THEN** vengono considerati i file supportati in `D:\Foto` e in tutte le sottocartelle, esclusi quelli che corrispondono ai pattern ignorati

#### Scenario: Rimozione sorgente
- **WHEN** l'utente rimuove una sorgente
- **THEN** i file di quella sorgente vengono rimossi dalla libreria senza toccare i file sul disco

#### Scenario: Rimozione dalla Libreria
- **WHEN** l'utente apre il menu di una sorgente nella barra laterale della Libreria e sceglie "Rimuovi dalla libreria…"
- **THEN** viene mostrata la stessa conferma con riepilogo della pagina Import

### Requirement: Scansione incrementale
Una nuova scansione di una sorgente SHALL rielaborare solo i file nuovi o modificati (dimensione o data di modifica diverse) e marcare come mancanti i file scomparsi. I file il cui contenuto è cambiato SHALL essere rianalizzati, conservando correzioni manuali, tag e cataloghi.

#### Scenario: Rescan senza modifiche
- **WHEN** una sorgente già importata viene riscansionata senza cambiamenti sul disco
- **THEN** nessun file viene rianalizzato

#### Scenario: File spostato rilevato
- **WHEN** un file viene spostato in un'altra cartella della stessa sorgente
- **THEN** il sistema lo riconosce tramite impronta del contenuto e conserva le sue catalogazioni e i cataloghi di appartenenza

#### Scenario: Foto ritoccata
- **WHEN** l'utente ritaglia una foto con un altro programma, ne ha corretto a mano il tipo soggetto, e aggiorna la sorgente
- **THEN** la foto ha una nuova miniatura e una nuova analisi automatica, e il tipo soggetto corretto a mano, i tag e i cataloghi restano invariati

#### Scenario: File mancante tornato presente
- **WHEN** un file marcato "mancante" ricompare nello stesso percorso e la sorgente viene aggiornata
- **THEN** il file torna disponibile con tutte le sue catalogazioni

## ADDED Requirements

### Requirement: Rimozione completa di una sorgente
Prima di rimuovere una sorgente il sistema SHALL mostrare un riepilogo: numero di foto, foto con correzioni manuali o tag, cataloghi manuali che perderanno foto e persone con nome che resteranno senza volti. Dopo la conferma il sistema SHALL annullare i job in corso su quella sorgente e cancellare anche miniature, embedding e dati dei volti delle sue foto, MUST NOT toccare i file sul disco, e SHALL eliminare le persone senza nome rimaste prive di volti.

#### Scenario: Importazione fatta per errore
- **WHEN** l'utente ha importato per errore `D:\Download` con 1.200 foto e sceglie "Rimuovi dalla libreria…"
- **THEN** la conferma indica 1.200 foto e, dopo la conferma, le foto non compaiono più in libreria, le loro miniature sono cancellate dalla cache e i file in `D:\Download` sono intatti

#### Scenario: Rimozione durante l'import
- **WHEN** l'import di una sorgente è in corso e l'utente la rimuove
- **THEN** l'import viene annullato prima della rimozione e nessuna foto di quella sorgente ricompare in libreria al termine

#### Scenario: Persone coinvolte
- **WHEN** la persona con nome "Giulia" compare solo in foto della sorgente da rimuovere
- **THEN** il riepilogo indica che "Giulia" resterà senza foto, e dopo la rimozione "Giulia" resta tra le persone con nome con 0 foto

#### Scenario: Persone senza nome
- **WHEN** un cluster "Senza nome" contiene solo volti della sorgente rimossa
- **THEN** dopo la rimozione quel cluster non esiste più

### Requirement: Riepilogo dell'aggiornamento
Al termine dell'aggiornamento di una sorgente il sistema SHALL mostrare quanti file sono nuovi, modificati, spostati o rinominati, diventati mancanti e tornati presenti, con accesso diretto alla libreria filtrata su ciascun gruppo.

#### Scenario: Cartella riorganizzata
- **WHEN** l'utente ha aggiunto 30 foto, spostato 12 foto in un'altra sottocartella e cancellato 5 foto, e poi aggiorna la sorgente
- **THEN** il riepilogo indica 30 nuove, 12 spostate e 5 mancanti, e un clic su "5 mancanti" mostra in libreria quei 5 file

#### Scenario: Nessuna modifica
- **WHEN** l'utente aggiorna una sorgente senza cambiamenti
- **THEN** il riepilogo indica che non ci sono modifiche

### Requirement: Aggiornamento di una cartella
L'utente SHALL poter aggiornare una sola sottocartella di una sorgente; l'aggiornamento SHALL considerare solo i file in quella cartella e nelle sue sottocartelle e marcare come mancanti solo i file al suo interno.

#### Scenario: Sottocartella modificata
- **WHEN** l'utente aggiunge foto in `D:\Foto\2024\Mare` e sceglie "Aggiorna cartella…" su quella cartella
- **THEN** vengono importate le nuove foto di `Mare` e nessun file fuori da `Mare` viene marcato mancante o rielaborato
