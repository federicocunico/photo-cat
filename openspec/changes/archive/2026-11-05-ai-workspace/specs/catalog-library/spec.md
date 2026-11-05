# Spec Delta

## ADDED Requirements

### Requirement: Filtro per stato dell'analisi
La Libreria SHALL poter filtrare le foto per stato di una fase di analisi (classificazione, persone, volti, gruppi di simili, impronte): analizzate con la versione corrente, da aggiornare o mai analizzate. Il filtro SHALL comparire tra i filtri attivi con un nome leggibile e SHALL essere rimovibile come gli altri. Conteggi e "Seleziona tutto" SHALL rispettarlo.

#### Scenario: Foto mai classificate
- **WHEN** è attivo il filtro "Classificazione: mai analizzate"
- **THEN** la griglia mostra solo le foto senza classificazione e "Seleziona tutto" seleziona esattamente quelle foto

#### Scenario: Combinazione con un catalogo
- **WHEN** l'utente apre il catalogo "Vacanze" con il filtro "Volti: da aggiornare"
- **THEN** la griglia mostra solo le foto del catalogo analizzate per i volti con una versione precedente
