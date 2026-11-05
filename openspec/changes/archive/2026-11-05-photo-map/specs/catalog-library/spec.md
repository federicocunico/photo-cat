# Spec Delta

## ADDED Requirements

### Requirement: Filtro per area geografica
La Libreria SHALL poter filtrare le foto per area geografica (rettangolo di latitudine e longitudine), includendo le foto con coordinate GPS nell'area e, se richiesto, le foto con un luogo approssimato collocato nell'area. Il filtro SHALL comparire tra i filtri attivi con il luogo prevalente come nome e SHALL essere rimovibile come gli altri.

#### Scenario: Filtro dalla mappa
- **WHEN** l'utente apre dalla mappa il gruppo di Firenze
- **THEN** tra i filtri attivi compare "Area: Firenze" e la griglia mostra solo le foto di quell'area

#### Scenario: Rimozione del filtro
- **WHEN** l'utente rimuove il filtro "Area: Firenze"
- **THEN** la griglia torna a mostrare tutte le foto dell'ambito precedente
