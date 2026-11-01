# Spec Delta

## MODIFIED Requirements

### Requirement: Ricerca e filtri
Il sistema SHALL permettere di filtrare la libreria o un catalogo per sorgente, cartella (con o senza sottocartelle), tipo soggetto, interno/esterno, persone, luogo, intervallo di date, formato e tag, e di cercare per testo su nome file, percorso, luogo, persone e tag.

#### Scenario: Filtri combinati
- **WHEN** l'utente seleziona i filtri "Esterno" e "Persona: Giulia"
- **THEN** vengono mostrati solo i file esterni in cui è presente Giulia

#### Scenario: Risposta su librerie grandi
- **WHEN** la libreria contiene 100.000 file e l'utente applica un filtro
- **THEN** il primo blocco di risultati viene mostrato entro 1 secondo su hardware consumer

#### Scenario: Filtro per cartella con sottocartelle
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone` includendo le sottocartelle
- **THEN** vengono mostrati i file di `Viaggio Giappone` e di tutte le sue sottocartelle (`D7000`, `Telefono`, …)

#### Scenario: Filtro per cartella esatta
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone\D7000` senza sottocartelle
- **THEN** vengono mostrati solo i file contenuti direttamente in quella cartella

## ADDED Requirements

### Requirement: Albero delle cartelle
Il sistema SHALL mostrare, sotto ogni sorgente, l'albero delle cartelle che contengono file della libreria con il numero di file per cartella (incluse le sottocartelle); l'albero SHALL caricarsi un livello alla volta per restare reattivo su librerie grandi.

#### Scenario: Espansione di una sorgente
- **WHEN** l'utente espande la sorgente `D:\Foto` che contiene `Viaggio Giappone` con 3 sottocartelle
- **THEN** compare `Viaggio Giappone` con il conteggio totale e, espandendola, le 3 sottocartelle con i rispettivi conteggi

#### Scenario: Seleziona tutto in una cartella
- **WHEN** l'utente sceglie una cartella nell'albero e poi "Seleziona tutto"
- **THEN** risultano selezionati tutti i file di quella cartella, pronti per un'azione bulk
