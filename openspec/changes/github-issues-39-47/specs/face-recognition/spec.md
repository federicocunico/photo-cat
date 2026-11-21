# Spec Delta

## ADDED Requirements

### Requirement: Dal volto alla foto
Il clic destro su un volto nella griglia di Persone SHALL aprire un menu che agisce sulla selezione corrente (o sul solo volto se non selezionato) con "Apri la foto", "Mostra in Libreria", "Apri con ▸" e "Mostra nella cartella", oltre alle azioni sui volti già disponibili. Il doppio clic e Invio su un volto SHALL aprire la foto nel visore. Il menu SHALL essere accessibile da tastiera.

#### Scenario: Apri la foto
- **WHEN** l'utente fa clic destro su un volto di "Giulia" e sceglie "Apri la foto"
- **THEN** si apre nel visore la foto da cui è preso il volto, con il riquadro del volto evidenziato

#### Scenario: Mostra in Libreria
- **WHEN** l'utente sceglie "Mostra in Libreria" su un volto
- **THEN** si apre la Libreria nella cartella della foto, con la foto selezionata e portata in vista

#### Scenario: Apri con un editor esterno su più volti
- **WHEN** l'utente seleziona 4 volti di 3 foto diverse e sceglie "Apri con ▸ Photoshop"
- **THEN** le 3 foto, ciascuna una sola volta, vengono passate a Photoshop

#### Scenario: Doppio clic
- **WHEN** l'utente fa doppio clic su un volto
- **THEN** la foto si apre nel visore e chiudendolo si torna alla stessa posizione della griglia dei volti

#### Scenario: Foto mancante sul disco
- **WHEN** la foto del volto è segnata mancante e l'utente sceglie "Apri con ▸"
- **THEN** compare un avviso che il file non è disponibile, con il percorso, e nessuna applicazione viene avviata

### Requirement: Ordinamento dei volti per data
La griglia dei volti di una persona o di un gruppo SHALL poter essere ordinata per qualità (predefinito), per data di scatto dalla più vecchia o per data di scatto dalla più recente. Negli ordinamenti per data SHALL mostrare un'intestazione per anno con il numero di volti, e i volti senza data SHALL comparire in fondo sotto "Senza data". La scelta SHALL essere ricordata.

#### Scenario: Persona per anno
- **WHEN** l'utente apre "Nonno Piero" con volti dal 1958 al 2004 e sceglie "Data: dalla più vecchia"
- **THEN** i volti sono in ordine cronologico con le intestazioni "1958", "1959", … e il numero di volti di ciascun anno

#### Scenario: Date impostate a mano
- **WHEN** una scansione ha data manuale "1962" e l'ordinamento è per data
- **THEN** il suo volto compare sotto l'intestazione "1962"

#### Scenario: Scelta ricordata
- **WHEN** l'utente sceglie "Data: dalla più recente", chiude l'app e apre un'altra persona
- **THEN** i volti sono ordinati dalla data più recente

#### Scenario: Persona con molti volti
- **WHEN** la persona ha 3.000 volti e l'utente cambia ordinamento
- **THEN** la griglia si aggiorna caricando i dati a pagine senza blocchi percepibili, e la selezione esistente resta valida
