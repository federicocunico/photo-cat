# Spec Delta

## ADDED Requirements

### Requirement: Assegnazione in blocco dei gruppi
In "Da rivedere" l'utente SHALL poter filtrare i gruppi per numero di foto ("1 foto", "fino a 2 foto", "fino a N foto") e selezionarli tutti; in "Singoli" SHALL poter selezionare tutti i volti. Con "Assegna a…" SHALL poter spostare i volti selezionati in una persona o identità scelta con ricerca, anche non selezionata, oppure in una nuova persona con nome, in un'unica operazione con un solo "Annulla", anche con centinaia di gruppi.

#### Scenario: Gruppi con una foto assegnati a una persona
- **WHEN** l'utente filtra "1 foto" in "Da rivedere", preme Ctrl+A sui 420 gruppi e sceglie "Assegna a…" › "Nonno Piero"
- **THEN** tutti i volti dei 420 gruppi risultano di "Nonno Piero", i 420 gruppi non esistono più e un avviso offre "Annulla"

#### Scenario: Annulla
- **WHEN** dopo l'assegnazione di 420 gruppi l'utente sceglie "Annulla"
- **THEN** i 420 gruppi tornano com'erano, con i loro volti, e "Nonno Piero" ha di nuovo solo i suoi volti

#### Scenario: Identità non ancora collegata
- **WHEN** l'utente assegna 12 gruppi all'identità "Zia Anna", che non ha ancora una persona in questa libreria
- **THEN** i 12 gruppi diventano un'unica persona collegata a "Zia Anna"

#### Scenario: Nuova persona
- **WHEN** l'utente assegna 5 gruppi a "Nuova persona…" e scrive "Vicino di casa"
- **THEN** i 5 gruppi diventano un'unica persona "Vicino di casa"

#### Scenario: Volti singoli
- **WHEN** in "Singoli" l'utente preme Ctrl+A sui 230 volti e sceglie "Assegna a…" › "Sconosciuti"
- **THEN** i 230 volti risultano di "Sconosciuti" e un avviso offre "Annulla"

#### Scenario: Correzioni rispettate
- **WHEN** un volto di un gruppo assegnato era stato rifiutato per la persona di destinazione
- **THEN** quel volto non viene assegnato alla destinazione e resta in un gruppo da rivedere
