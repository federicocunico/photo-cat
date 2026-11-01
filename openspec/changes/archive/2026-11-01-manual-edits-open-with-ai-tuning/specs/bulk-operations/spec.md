# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
