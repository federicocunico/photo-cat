# in-app-guidance Specification

## Purpose
Aiutare chi usa l'app senza un manuale: primo avvio in pochi passi, lista dei primi passi, suggerimenti al primo uso dei moduli e menu Aiuto.

## Requirements

### Requirement: Primi passi
Dopo il primo avvio la Libreria SHALL mostrare una lista "Primi passi" con:
- importare una cartella;
- scaricare i modelli AI;
- dare un nome a una persona;
- creare un catalogo;
- usare un set di tag.

Ogni voce SHALL avere un comando che porta al punto giusto e SHALL risultare completata quando l'utente fa quell'azione, anche per un'altra strada. I cataloghi di sistema (Cestino, Duplicati, Qualità insufficiente) MUST NOT completare la voce "Crea un catalogo". La lista SHALL avere il comando di chiusura in alto a destra.

#### Scenario: Passo completato altrove
- **WHEN** l'utente crea un catalogo dalla barra delle azioni della Libreria
- **THEN** la voce "Crea un catalogo" della lista risulta completata

#### Scenario: Solo cataloghi di sistema
- **WHEN** la libreria ha solo i cataloghi di sistema
- **THEN** la voce "Crea un catalogo" non risulta completata

### Requirement: Persistenza dei primi passi
La lista "Primi passi" SHALL sparire quando tutte le voci sono completate o quando l'utente la nasconde, e SHALL poter essere riaperta dal menu Aiuto. Il suo stato SHALL sopravvivere a riavvii e reinstallazioni.

#### Scenario: Lista nascosta
- **WHEN** l'utente nasconde la lista e riavvia l'app
- **THEN** la lista non compare, e Aiuto › "Primi passi" la riapre

### Requirement: Suggerimenti al primo uso
La prima volta che l'utente apre Persone, Cataloghi, Tag o Sequenza, il modulo SHALL mostrare un suggerimento breve e non modale, con "Scopri di più" e "Ho capito". I suggerimenti MUST NOT bloccare l'interfaccia, MUST NOT coprire comandi o il contenuto principale del modulo, e MUST NOT comparire più di una volta per modulo, salvo reimpostazione dal menu Aiuto. L'app MUST NOT presentare tour a schede obbligatori.

#### Scenario: Primo ingresso in Tag
- **WHEN** l'utente apre Tag per la prima volta
- **THEN** compare accanto all'elenco dei set il suggerimento "Un set assegna i tasti 1–9: in Libreria premi un numero per taggare le foto selezionate", che si chiude con "Ho capito" e non ricompare

#### Scenario: Reimpostare i suggerimenti
- **WHEN** l'utente sceglie Aiuto › "Mostra di nuovo i suggerimenti"
- **THEN** i suggerimenti ricompaiono al successivo ingresso in ciascun modulo

#### Scenario: Comandi liberi
- **WHEN** compare il suggerimento al primo ingresso in Sequenza
- **THEN** tutti i comandi della barra di Sequenza restano visibili e utilizzabili

### Requirement: Menu Aiuto
La barra dei menu SHALL offrire un menu Aiuto con:
- Guida;
- Scorciatoie da tastiera;
- Primi passi;
- Rivedi l'introduzione;
- Mostra di nuovo i suggerimenti;
- Segnala un problema;
- Copia le informazioni di sistema;
- Informazioni su photo-cat (versione, build, sistema, autori e licenze).

#### Scenario: Segnalare un problema
- **WHEN** l'utente sceglie Aiuto › "Segnala un problema"
- **THEN** si apre Impostazioni › Diagnostica con il comando di esportazione del pacchetto diagnostico in evidenza

#### Scenario: Informazioni dal menu
- **WHEN** l'utente sceglie Aiuto › "Informazioni su photo-cat"
- **THEN** si apre la finestra Informazioni con versione e build
