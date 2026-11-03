# in-app-guidance

## ADDED Requirements

### Requirement: Primi passi
Dopo il primo avvio la Libreria SHALL mostrare una lista "Primi passi" con:
- importare una cartella;
- scaricare i modelli AI;
- dare un nome a una persona;
- creare un catalogo;
- usare un set di tag.

Ogni voce SHALL avere un comando che porta al punto giusto e SHALL risultare completata quando l'utente fa quell'azione, anche per un'altra strada.

#### Scenario: Passo completato altrove
- **WHEN** l'utente crea un catalogo dalla barra delle azioni della Libreria
- **THEN** la voce "Crea un catalogo" della lista risulta completata

### Requirement: Persistenza dei primi passi
La lista "Primi passi" SHALL sparire quando tutte le voci sono completate o quando l'utente la nasconde, e SHALL poter essere riaperta dal menu Aiuto. Il suo stato SHALL sopravvivere a riavvii e reinstallazioni.

#### Scenario: Lista nascosta
- **WHEN** l'utente nasconde la lista e riavvia l'app
- **THEN** la lista non compare, e Aiuto › "Primi passi" la riapre

### Requirement: Suggerimenti al primo uso
La prima volta che l'utente apre Persone, Cataloghi, Tag o Sequenza, il modulo SHALL mostrare un suggerimento breve e non modale, con "Scopri di più" e "Ho capito". I suggerimenti MUST NOT bloccare l'interfaccia né comparire più di una volta per modulo, salvo reimpostazione dal menu Aiuto. L'app MUST NOT presentare tour a schede obbligatori.

#### Scenario: Primo ingresso in Tag
- **WHEN** l'utente apre Tag per la prima volta
- **THEN** compare accanto all'elenco dei set il suggerimento "Un set assegna i tasti 1–9: in Libreria premi un numero per taggare le foto selezionate", che si chiude con "Ho capito" e non ricompare

#### Scenario: Reimpostare i suggerimenti
- **WHEN** l'utente sceglie Aiuto › "Mostra di nuovo i suggerimenti"
- **THEN** i suggerimenti ricompaiono al successivo ingresso in ciascun modulo

### Requirement: Menu Aiuto
La barra superiore SHALL offrire un menu Aiuto con:
- Scorciatoie da tastiera;
- Rivedi l'introduzione;
- Primi passi;
- Mostra di nuovo i suggerimenti;
- Guida;
- Segnala un problema;
- Informazioni su photo-cat (versione e percorso dei dati).

#### Scenario: Segnalare un problema
- **WHEN** l'utente sceglie Aiuto › "Segnala un problema"
- **THEN** si apre Impostazioni › Diagnostica con il comando di esportazione del pacchetto diagnostico in evidenza
