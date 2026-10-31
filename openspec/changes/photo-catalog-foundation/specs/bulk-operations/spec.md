# Spec Delta

## Purpose

Permette di agire in un colpo solo su molti file della libreria o di un catalogo (selezioni manuali, risultati di filtri o interi cataloghi) per taggare, catalogare, correggere, rianalizzare o rimuovere.

## ADDED Requirements

### Requirement: Selezione multipla
L'utente SHALL poter selezionare più file con clic, Ctrl/Shift+clic, selezione rettangolare, "seleziona tutto" sui risultati del filtro corrente e inversione della selezione; la UI SHALL mostrare sempre il numero di elementi selezionati.

#### Scenario: Seleziona tutto i risultati
- **WHEN** un filtro restituisce 2.340 file e l'utente sceglie "Seleziona tutto"
- **THEN** risultano selezionati 2.340 file, inclusi quelli non ancora visualizzati nella griglia

### Requirement: Azioni bulk disponibili
Su una selezione l'utente SHALL poter: aggiungere/rimuovere tag, aggiungere/rimuovere da cataloghi manuali, impostare o correggere un campo di catalogazione, assegnare una persona, rianalizzare con AI, esportare, organizzare in cartelle e rimuovere dalla libreria.

#### Scenario: Tag in bulk
- **WHEN** l'utente aggiunge il tag "Matrimonio" a 150 file selezionati
- **THEN** tutti i 150 file hanno il tag "Matrimonio"

#### Scenario: Correzione campo in bulk
- **WHEN** l'utente imposta "Locazione = Interno: Teatro" su 40 file
- **THEN** i 40 file hanno quella locazione marcata come correzione manuale

### Requirement: Esecuzione come job per selezioni grandi
Le azioni bulk su più di una soglia di elementi o che richiedono elaborazione SHALL essere eseguite come job con progresso, senza bloccare l'interfaccia.

#### Scenario: Rianalisi di 5.000 file
- **WHEN** l'utente avvia la rianalisi di 5.000 file
- **THEN** viene creato un job con progresso visibile e l'utente può continuare a navigare la libreria

### Requirement: Conferma per azioni distruttive
Le azioni bulk che rimuovono dati (rimozione dalla libreria, rimozione da catalogo, cancellazione dati volti) SHALL richiedere conferma indicando il numero di file coinvolti, e MUST NOT cancellare mai i file originali dal disco.

#### Scenario: Rimozione dalla libreria
- **WHEN** l'utente rimuove 30 file dalla libreria
- **THEN** il sistema chiede conferma indicando 30 file e, dopo conferma, i file restano sul disco

### Requirement: Annullamento delle modifiche ai metadati
Le azioni bulk che modificano tag, cataloghi o catalogazioni SHALL poter essere annullate con "Annulla" almeno per l'ultima operazione eseguita.

#### Scenario: Annulla tag
- **WHEN** l'utente aggiunge un tag a 200 file e poi sceglie "Annulla"
- **THEN** il tag viene rimosso dai 200 file e gli altri dati restano invariati

### Requirement: Esito delle azioni bulk
Al termine di un'azione bulk il sistema SHALL riportare quanti elementi sono stati elaborati con successo e quanti sono falliti, con accesso agli errori.

#### Scenario: Esito parziale
- **WHEN** un'azione su 100 file fallisce per 2 file
- **THEN** l'esito mostra 98 riusciti e 2 falliti con collegamento ai dettagli in diagnostica
