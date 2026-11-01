# Spec Delta

## ADDED Requirements

### Requirement: Elenco completo dei valori dei filtri
Ogni lista di filtro della libreria (catalogazioni AI per gruppo, luoghi, anni, formati, tag) SHALL mostrare in forma compatta le voci con più foto, il numero totale di voci e un comando per vederle tutte. Ogni voce esistente nei risultati SHALL essere raggiungibile e selezionabile, senza limiti fissi al numero di voci, con ricerca testuale e ordinamento per numero di foto o alfabetico.

#### Scenario: Molte località
- **WHEN** la libreria contiene foto in 340 luoghi diversi
- **THEN** la lista Luoghi mostra le voci principali e "Mostra tutti (340)", e dall'elenco completo è possibile selezionare il luogo con meno foto

#### Scenario: Ricerca nell'elenco
- **WHEN** l'utente apre l'elenco completo dei luoghi e digita "sie"
- **THEN** compaiono solo i luoghi il cui nome contiene "sie" (senza distinzione di maiuscole e accenti), con i rispettivi conteggi

#### Scenario: Ordinamento alfabetico
- **WHEN** l'utente sceglie l'ordinamento alfabetico nell'elenco completo dei tag
- **THEN** i tag sono elencati in ordine alfabetico invece che per numero di foto

#### Scenario: Voce selezionata sempre visibile
- **WHEN** l'utente ha selezionato un luogo che non è tra le voci principali e richiude l'elenco completo
- **THEN** il luogo selezionato resta visibile e spuntato nella lista compatta

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 foto e 5.000 tag e l'utente apre l'elenco completo dei tag
- **THEN** le prime voci compaiono entro 1 secondo e lo scorrimento dell'elenco resta fluido

### Requirement: Conteggi dei filtri indipendenti dalla propria selezione
Le voci e i conteggi di una lista di filtro SHALL essere calcolati applicando tutti gli altri filtri attivi ma non la selezione della lista stessa, così che le altre voci della stessa lista restino visibili e selezionabili.

#### Scenario: Selezione di più luoghi
- **WHEN** l'utente seleziona il luogo "Siena" e la lista Luoghi conteneva anche "Firenze" con 80 foto
- **THEN** "Firenze" resta nella lista con 80 foto e può essere aggiunto alla selezione

#### Scenario: Altri filtri applicati
- **WHEN** è attivo il filtro "Esterno" e l'utente guarda la lista Luoghi
- **THEN** i conteggi dei luoghi considerano solo le foto esterne
