# bulk-operations Specification

## Purpose
Permette di agire in un colpo solo su molti file della libreria o di un catalogo (selezioni manuali, risultati di filtri o interi cataloghi) per taggare, catalogare, correggere, rianalizzare o rimuovere.

## Requirements

### Requirement: Selezione multipla
L'utente SHALL poter selezionare più file con clic, Ctrl/Shift+clic, selezione rettangolare, "seleziona tutto" sui risultati del filtro corrente e inversione della selezione; la UI SHALL mostrare sempre il numero di elementi selezionati.

#### Scenario: Seleziona tutto i risultati
- **WHEN** un filtro restituisce 2.340 file e l'utente sceglie "Seleziona tutto"
- **THEN** risultano selezionati 2.340 file, inclusi quelli non ancora visualizzati nella griglia

### Requirement: Azioni bulk disponibili
Su una selezione l'utente SHALL poter: aggiungere/rimuovere tag, aggiungere/rimuovere da cataloghi manuali, impostare o correggere un campo di catalogazione, impostare il luogo o ripristinare il luogo automatico, assegnare una persona, rianalizzare con AI, esportare, organizzare in cartelle e rimuovere dalla libreria.

#### Scenario: Tag in bulk
- **WHEN** l'utente aggiunge il tag "Matrimonio" a 150 file selezionati
- **THEN** tutti i 150 file hanno il tag "Matrimonio"

#### Scenario: Correzione campo in bulk
- **WHEN** l'utente imposta "Locazione = Interno: Teatro" su 40 file
- **THEN** i 40 file hanno quella locazione marcata come correzione manuale

#### Scenario: Luogo in bulk
- **WHEN** l'utente seleziona le 300 foto della cartella `Viaggio Giappone\D7000` e imposta il luogo "Giappone"
- **THEN** le 300 foto hanno luogo "Giappone" con fonte "manuale" e una rianalisi successiva non lo modifica

#### Scenario: Ripristino del luogo automatico
- **WHEN** l'utente sceglie "Ripristina luogo automatico" su 10 foto con luogo manuale
- **THEN** le 10 foto tornano al luogo dedotto da GPS o directory, con la relativa fonte

#### Scenario: Annulla luogo
- **WHEN** l'utente imposta un luogo su 50 foto e poi sceglie "Annulla"
- **THEN** le 50 foto tornano al luogo e alla fonte che avevano prima

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

### Requirement: Rinomina di un luogo
L'utente SHALL poter rinominare un luogo: tutte le foto della libreria con quel nome di luogo ricevono il nuovo nome come correzione manuale. Il sistema SHALL indicare il numero di foto coinvolte prima della conferma e l'operazione SHALL essere annullabile; rinominare verso un luogo già esistente unisce i due luoghi.

#### Scenario: Rinomina da facet
- **WHEN** il luogo "D7000" ha 300 foto e l'utente lo rinomina in "Giappone"
- **THEN** dopo la conferma che indica 300 foto, il luogo "D7000" non compare più e "Giappone" include quelle 300 foto

#### Scenario: Unione con luogo esistente
- **WHEN** esistono "Giappone" con 120 foto e "D7000" con 300 foto e l'utente rinomina "D7000" in "Giappone"
- **THEN** "Giappone" conta 420 foto

#### Scenario: Annulla rinomina
- **WHEN** l'utente rinomina un luogo e poi sceglie "Annulla"
- **THEN** le foto coinvolte tornano al nome e alla fonte di luogo precedenti

### Requirement: Esclusività dei set nelle azioni bulk
Quando un'azione bulk o un tasto rapido aggiunge a una selezione un tag che appartiene a un set esclusivo, il sistema SHALL rimuovere dagli stessi file gli altri tag di quel set; l'operazione SHALL essere annullabile come un'unica azione.

#### Scenario: Tag in bulk con set esclusivo
- **WHEN** 40 file selezionati hanno "Paesaggi" del set esclusivo "Viaggio USA 2026" e l'utente aggiunge "Città" in bulk
- **THEN** i 40 file hanno "Città" e non hanno più "Paesaggi"

#### Scenario: Annulla
- **WHEN** l'utente sceglie "Annulla" subito dopo
- **THEN** i 40 file tornano ad avere "Paesaggi" e non hanno "Città"

### Requirement: Tagging da tastiera sulla selezione
In Libreria e in Sequenza i tasti 1–9 e 0 del set attivo SHALL agire sull'intera selezione corrente, inclusi i file selezionati ma non visibili, e il risultato SHALL comparire senza ricaricare la vista.

#### Scenario: Selezione estesa
- **WHEN** l'utente seleziona tutti i 2.340 risultati di un filtro e preme 2
- **THEN** i 2.340 file ricevono il tag in posizione 2 del set attivo
