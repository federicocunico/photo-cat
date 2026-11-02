# Spec Delta

## MODIFIED Requirements

### Requirement: Selezione multipla
L'utente SHALL poter selezionare più file con clic, Ctrl/Shift+clic, selezione rettangolare, "seleziona tutto" sui risultati del filtro corrente e inversione della selezione; la UI SHALL mostrare sempre il numero di elementi selezionati. "Seleziona tutto" e l'inversione SHALL funzionare con 100.000 risultati senza caricare nell'interfaccia l'elenco di tutti gli elementi, e le azioni bulk SHALL applicarsi esattamente agli elementi selezionati.

#### Scenario: Seleziona tutto i risultati
- **WHEN** un filtro restituisce 2.340 file e l'utente sceglie "Seleziona tutto"
- **THEN** risultano selezionati 2.340 file, inclusi quelli non ancora visualizzati nella griglia

#### Scenario: Seleziona tutto su una libreria grande
- **WHEN** la libreria contiene 100.000 foto e l'utente sceglie "Seleziona tutto", poi deseleziona 3 foto con Ctrl+clic e aggiunge un tag
- **THEN** il conteggio indica 99.997 selezionati, l'interfaccia resta reattiva e il tag viene aggiunto esattamente alle 99.997 foto

#### Scenario: Risultati cambiati dopo la selezione
- **WHEN** dopo "Seleziona tutto" su un filtro un import aggiunge nuove foto che soddisfano il filtro
- **THEN** l'azione bulk si applica solo alle foto che erano nei risultati al momento della selezione
