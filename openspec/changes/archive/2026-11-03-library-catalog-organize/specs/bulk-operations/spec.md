# Spec Delta

## MODIFIED Requirements

### Requirement: Selezione multipla
L'utente SHALL poter selezionare più file con clic, Ctrl/Shift+clic, selezione rettangolare, "seleziona tutto" sui risultati del filtro corrente e inversione della selezione; la UI SHALL mostrare sempre il numero di elementi selezionati. "Seleziona tutto" e l'inversione SHALL funzionare con 100.000 risultati senza caricare nell'interfaccia l'elenco di tutti gli elementi, e le azioni bulk SHALL applicarsi esattamente agli elementi selezionati. Shift+clic SHALL selezionare l'intervallo dall'ancora, cioè l'elemento dell'ultimo clic semplice o Ctrl+clic; Shift+clic successivi MUST NOT spostare l'ancora.

#### Scenario: Seleziona tutto i risultati
- **WHEN** un filtro restituisce 2.340 file e l'utente sceglie "Seleziona tutto"
- **THEN** risultano selezionati 2.340 file, inclusi quelli non ancora visualizzati nella griglia

#### Scenario: Seleziona tutto su una libreria grande
- **WHEN** la libreria contiene 100.000 foto e l'utente sceglie "Seleziona tutto", poi deseleziona 3 foto con Ctrl+clic e aggiunge un tag
- **THEN** il conteggio indica 99.997 selezionati, l'interfaccia resta reattiva e il tag viene aggiunto esattamente alle 99.997 foto

#### Scenario: Risultati cambiati dopo la selezione
- **WHEN** dopo "Seleziona tutto" su un filtro un import aggiunge nuove foto che soddisfano il filtro
- **THEN** l'azione bulk si applica solo alle foto che erano nei risultati al momento della selezione

#### Scenario: Shift+clic ripetuti
- **WHEN** in una cartella con le foto 1…10 l'utente fa clic sulla foto 2, poi Shift+clic sulla 5, poi Shift+clic sulla 8
- **THEN** risultano selezionate le foto da 2 a 8

#### Scenario: Intervallo che si restringe
- **WHEN** dopo clic sulla foto 2 e Shift+clic sulla 8 l'utente fa Shift+clic sulla 4
- **THEN** risultano selezionate le foto da 2 a 4

#### Scenario: Ctrl+Shift+clic
- **WHEN** l'utente fa clic sulla foto 2, Ctrl+clic sulla 6 e poi Ctrl+Shift+clic sulla 9
- **THEN** risultano selezionate la foto 2 e le foto da 6 a 9

#### Scenario: Volti in Persone
- **WHEN** nella griglia dei volti l'utente fa clic sul primo volto, poi Shift+clic sul terzo e Shift+clic sul sesto
- **THEN** risultano selezionati i volti dal primo al sesto
