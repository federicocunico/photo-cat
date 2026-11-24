# Spec Delta

## ADDED Requirements

### Requirement: Copertina delle persone
Nell'elenco di Persone › Persone e nella scheda della persona, ogni persona con volti nella libreria aperta SHALL comparire con il suo volto di copertina: quello scelto dall'utente oppure, se non l'ha scelto, il volto migliore, con la stessa regola dei gruppi da rivedere. Le iniziali SHALL comparire solo per le persone senza volti nella libreria aperta.

#### Scenario: Copertina automatica
- **WHEN** l'utente apre Persone › Persone e Gioia ha 26 foto nella libreria, senza una copertina scelta
- **THEN** Gioia compare nell'elenco e nella scheda con il suo volto migliore, non con la "G"

#### Scenario: Copertina scelta
- **WHEN** l'utente sceglie "Usa come copertina" su un volto di Gioia
- **THEN** nell'elenco, nella scheda e nel selettore "Sposta in…" Gioia compare con quel volto, e l'avviso offre "Annulla operazione"

#### Scenario: Persona senza volti nella libreria
- **WHEN** una persona ha solo volti di riferimento e nessun volto nella libreria aperta
- **THEN** nell'elenco compare con le iniziali

## MODIFIED Requirements

### Requirement: Comandi di correzione dei volti
Le correzioni SHALL usare due soli verbi: "Sposta in…" per volti selezionati e "Unisci a…" per gruppi o persone. Le azioni sui volti SHALL comparire in una barra che sostituisce l'intestazione dei volti solo quando c'è almeno un volto selezionato: "Sposta in…", "Non è questa persona" e un menu "Altro" con "Nuovo gruppo con questi volti", "Usa come copertina" (un solo volto) e "Rimuovi volti…". Il menu contestuale di un solo volto SHALL contenere anche "Usa come copertina".

#### Scenario: Volti selezionati
- **WHEN** l'utente seleziona tre volti di un gruppo
- **THEN** al posto dell'intestazione compare "3 selezionati" con "Sposta in…", "Non è questa persona", "Altro" e "Deseleziona", su una riga

#### Scenario: Copertina dal volto
- **WHEN** l'utente seleziona un solo volto e apre "Altro"
- **THEN** il menu contiene "Usa come copertina", e il volto diventa la copertina del gruppo

#### Scenario: Copertina dal menu contestuale
- **WHEN** l'utente fa clic destro su un volto di un gruppo o di una persona
- **THEN** il menu contiene "Usa come copertina", e il volto diventa la copertina

#### Scenario: Unire un gruppo
- **WHEN** l'utente sceglie "Unisci a…" dalla scheda di un gruppo senza nome e sceglie "Gioia"
- **THEN** tutti i volti del gruppo passano a Gioia e un avviso dice "Gruppo unito a Gioia" con "Annulla operazione"
