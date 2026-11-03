# Spec Delta

## MODIFIED Requirements

### Requirement: Gestione delle identità
L'utente SHALL poter creare, modificare, unire ed eliminare identità dalla sezione "Persone" del modulo Persone, con filtro per gruppo e ricerca su nome e alias.

#### Scenario: Ricerca per alias
- **WHEN** l'utente cerca "Nonna Rita"
- **THEN** compare l'identità "Rita Rossi" che ha quell'alias

#### Scenario: Unione di identità
- **WHEN** l'utente unisce "G. Bianchi" in "Giulia Bianchi"
- **THEN** volti di riferimento, alias e collegamenti di "G. Bianchi" passano a "Giulia Bianchi" e "G. Bianchi" non esiste più

#### Scenario: Identità senza foto nella libreria
- **WHEN** un'identità non ha volti nella libreria aperta
- **THEN** la sezione "Persone" la mostra con l'indicazione "nessuna foto in questa libreria", e resta modificabile ed esportabile

### Requirement: Collegamento con le persone della libreria
In ogni libreria un gruppo di volti SHALL poter essere collegato a un'identità tramite "Chi è?" nella scheda del gruppo; il collegamento trasforma il gruppo in persona, il nome mostrato è quello dell'identità e la sezione "Persone" indica per ogni identità il numero di foto nella libreria aperta.

#### Scenario: Collegamento
- **WHEN** l'utente collega il gruppo "Gruppo di 23 volti" all'identità "Marco Russo"
- **THEN** il gruppo esce da "Da rivedere", appare come "Marco Russo" nella sezione "Persone" e l'identità mostra il numero di foto del gruppo

#### Scenario: Collegamento a un'identità già collegata
- **WHEN** l'utente collega un secondo gruppo a "Marco Russo", che ha già un gruppo collegato in questa libreria
- **THEN** i due gruppi vengono uniti nella persona "Marco Russo"
