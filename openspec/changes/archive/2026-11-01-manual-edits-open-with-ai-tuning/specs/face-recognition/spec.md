# Spec Delta

## MODIFIED Requirements

### Requirement: Rilevamento volti
Il sistema SHALL rilevare i volti presenti in ogni foto analizzata, memorizzandone posizione e qualità; i volti sotto una dimensione o qualità minima configurabile SHALL essere ignorati per il riconoscimento. La qualità minima è la soglia di confidenza del rilevamento volti impostata dall'utente.

#### Scenario: Volti rilevati
- **WHEN** viene analizzata una foto con 3 volti ben visibili
- **THEN** la foto ha 3 volti associati con le rispettive aree

#### Scenario: Volto troppo piccolo
- **WHEN** un volto è sotto la dimensione minima
- **THEN** il volto non viene usato per il raggruppamento

#### Scenario: Soglia di rilevamento alzata
- **WHEN** una foto ha volti con confidenza 0,95 e 0,85 e l'utente porta la soglia di rilevamento volti a 0,9
- **THEN** solo il volto con confidenza 0,95 viene usato per il raggruppamento

### Requirement: Raggruppamento in persone
Il sistema SHALL raggruppare automaticamente i volti simili in cluster di persone e mostrare per ciascuna il numero di foto; i cluster senza nome SHALL apparire come "Senza nome". Due volti SHALL essere considerati della stessa persona solo se la loro somiglianza raggiunge la soglia di somiglianza configurata.

#### Scenario: Stessa persona in più foto
- **WHEN** la stessa persona compare in 20 foto
- **THEN** i suoi volti sono raggruppati in un'unica persona con conteggio 20

#### Scenario: Soglia più severa
- **WHEN** l'utente alza la soglia di somiglianza e salva
- **THEN** il raggruppamento viene ricalcolato dai volti salvati e i cluster con volti poco simili si separano, mentre le persone con nome e le correzioni manuali restano invariate

### Requirement: Identificazione delle persone
L'utente SHALL poter assegnare un nome a una persona; i nuovi volti la cui somiglianza con una persona con nome raggiunge la soglia di somiglianza configurata SHALL essere associati automaticamente a quella persona.

#### Scenario: Nome assegnato e propagato
- **WHEN** l'utente chiama "Giulia" un cluster e poi importa nuove foto in cui Giulia compare
- **THEN** le nuove foto riportano "Persone: Giulia"

#### Scenario: Volto sotto soglia
- **WHEN** un nuovo volto ha somiglianza con "Giulia" inferiore alla soglia configurata
- **THEN** il volto non viene associato automaticamente a "Giulia"
