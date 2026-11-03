# Spec Delta

## MODIFIED Requirements

### Requirement: Azioni bulk disponibili
Su una selezione l'utente SHALL poter: aggiungere/rimuovere tag, aggiungere a cataloghi manuali, escludere da un catalogo (manuale o smart) e reinserire nel catalogo, impostare o correggere un campo di catalogazione, impostare il luogo o ripristinare il luogo automatico, assegnare una persona, rianalizzare con AI, esportare, organizzare in cartelle e rimuovere dalla libreria.

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

#### Scenario: Esclusione in bulk da un catalogo smart
- **WHEN** l'utente seleziona 120 foto del catalogo smart "Paesaggi" e sceglie "Escludi dal catalogo"
- **THEN** le 120 foto non compaiono più in "Paesaggi", compaiono tra le sue escluse, e "Annulla" le riporta nel catalogo
