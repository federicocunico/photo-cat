# Spec Delta

## ADDED Requirements

### Requirement: Valutazione delle foto
L'utente SHALL poter assegnare a ogni foto una valutazione da 0 (nessuna) a 5 stelle dall'Inspector, dal menu contestuale "Valutazione ▸", in bulk e da tastiera con Alt+1–5 e Alt+0 sulla selezione corrente. La valutazione SHALL essere salvata in libreria, annullabile, mostrata sulle miniature quando è maggiore di 0, e MUST NOT modificare i file originali.

#### Scenario: Valutazione da tastiera
- **WHEN** l'utente seleziona 3 foto in Libreria e preme Alt+5
- **THEN** le 3 foto hanno 5 stelle e le miniature lo mostrano senza ricaricare la vista

#### Scenario: Azzeramento
- **WHEN** una foto ha 3 stelle e l'utente preme Alt+0
- **THEN** la foto non ha più valutazione e la miniatura non mostra stelle

#### Scenario: Tasti numerici senza Alt
- **WHEN** il set di tag attivo ha "Paesaggi" in posizione 2 e l'utente preme 2 su una foto selezionata
- **THEN** la foto riceve il tag "Paesaggi" e la sua valutazione non cambia

#### Scenario: Originale intatto
- **WHEN** l'utente assegna 4 stelle a una foto JPEG
- **THEN** il file sul disco ha lo stesso contenuto e la stessa data di modifica di prima

### Requirement: Caratteristica di colore
L'utente SHALL poter impostare a mano su una o più foto una caratteristica di colore tra "Bianco e nero", "Colore", "Colore virato" e "Seppia", oppure rimuoverla, dal menu contestuale "Colore ▸" e dall'Inspector. Il sistema MUST NOT assegnare la caratteristica automaticamente né modificare i pixel o i file originali. Una foto senza caratteristica SHALL risultare "Non indicato".

#### Scenario: Scansioni virate
- **WHEN** l'utente seleziona 12 stampe scansionate dai colori virati al rosso e sceglie "Colore ▸ Colore virato"
- **THEN** le 12 foto hanno caratteristica "Colore virato" e l'Inspector la mostra

#### Scenario: Rimozione
- **WHEN** una foto è "Seppia" e l'utente sceglie "Colore ▸ Non indicato"
- **THEN** la foto non ha più caratteristica di colore

#### Scenario: Nessuna assegnazione automatica
- **WHEN** viene importata o rianalizzata una foto in bianco e nero
- **THEN** la sua caratteristica di colore resta "Non indicato" finché l'utente non la imposta

### Requirement: Filtri per valutazione e colore
La libreria e i cataloghi SHALL poter essere filtrati per valutazione minima o esatta e per caratteristica di colore, incluso "Non indicato". Le regole dei cataloghi smart SHALL offrire i campi "Valutazione" (confronti "almeno", "uguale a", "al massimo") e "Colore". I conteggi dei filtri SHALL seguire le stesse regole degli altri filtri.

#### Scenario: Valutazione minima
- **WHEN** l'utente filtra "Valutazione: almeno ★★★★"
- **THEN** sono mostrate solo le foto con 4 o 5 stelle

#### Scenario: Catalogo smart delle foto storiche in bianco e nero
- **WHEN** l'utente crea un catalogo smart "Colore è Bianco e nero e Data di scatto prima del 1970"
- **THEN** il catalogo contiene le foto con quella caratteristica e data, e si aggiorna quando l'utente imposta la caratteristica su altre foto

#### Scenario: Foto non indicate
- **WHEN** l'utente filtra "Colore: Non indicato"
- **THEN** sono mostrate solo le foto senza caratteristica di colore

## MODIFIED Requirements

### Requirement: Ricerca e filtri
Il sistema SHALL permettere di filtrare la libreria o un catalogo per sorgente, cartella (con o senza sottocartelle), tipo soggetto, interno/esterno, persone, luogo, intervallo di date, formato, tag, valutazione e caratteristica di colore, e di cercare per testo su nome file, percorso, luogo, persone e tag.

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

#### Scenario: Valutazione e colore combinati
- **WHEN** l'utente seleziona i filtri "Valutazione: almeno ★★★" e "Colore: Seppia"
- **THEN** vengono mostrate solo le foto seppia con almeno 3 stelle
