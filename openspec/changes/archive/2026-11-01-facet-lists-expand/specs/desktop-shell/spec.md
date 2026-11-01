# Spec Delta

## ADDED Requirements

### Requirement: Pannelli flottanti
L'app SHALL poter mostrare contenuti in pannelli flottanti non modali sopra l'interfaccia: trascinabili dalla barra del titolo, ridimensionabili, chiudibili con un pulsante o con Esc quando hanno il focus, sempre interamente raggiungibili dentro la finestra. Mentre un pannello è aperto l'utente SHALL poter continuare a usare il resto dell'interfaccia. Posizione e dimensione SHALL essere ricordate tra un'apertura e l'altra e tra i riavvii.

#### Scenario: Elenco luoghi in un pannello flottante
- **WHEN** l'utente sceglie "Apri in finestra" sulla lista Luoghi
- **THEN** si apre un pannello flottante con l'elenco completo dei luoghi, e l'utente può selezionare foto nella griglia senza chiuderlo

#### Scenario: Aggiornamento col filtro
- **WHEN** il pannello dei luoghi è aperto e l'utente attiva il filtro "Anno 2023" dalla barra laterale
- **THEN** il pannello mostra i luoghi e i conteggi delle sole foto del 2023

#### Scenario: Più pannelli
- **WHEN** l'utente apre il pannello dei luoghi e quello dei tag
- **THEN** i due pannelli sono visibili insieme e quello cliccato per ultimo è in primo piano

#### Scenario: Posizione ricordata
- **WHEN** l'utente sposta e ridimensiona il pannello dei luoghi, lo chiude e lo riapre dopo un riavvio
- **THEN** il pannello ricompare con la stessa posizione e dimensione

#### Scenario: Finestra rimpicciolita
- **WHEN** la finestra dell'app viene ridotta e un pannello aperto resterebbe in parte fuori
- **THEN** il pannello viene riposizionato e, se serve, ridotto per restare interamente visibile

#### Scenario: Uso da tastiera
- **WHEN** il pannello ha il focus e l'utente preme Esc
- **THEN** il pannello si chiude e il focus torna al comando che lo aveva aperto
