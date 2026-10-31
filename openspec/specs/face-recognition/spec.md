# face-recognition Specification

## Purpose
Rileva i volti nelle foto, li raggruppa per somiglianza in persone e permette all'utente di dare un nome alle persone e correggere i raggruppamenti, mantenendo i dati biometrici solo in locale.

## Requirements

### Requirement: Rilevamento volti
Il sistema SHALL rilevare i volti presenti in ogni foto analizzata, memorizzandone posizione e qualità; i volti sotto una dimensione o qualità minima configurabile SHALL essere ignorati per il riconoscimento.

#### Scenario: Volti rilevati
- **WHEN** viene analizzata una foto con 3 volti ben visibili
- **THEN** la foto ha 3 volti associati con le rispettive aree

#### Scenario: Volto troppo piccolo
- **WHEN** un volto è sotto la dimensione minima
- **THEN** il volto non viene usato per il raggruppamento

### Requirement: Raggruppamento in persone
Il sistema SHALL raggruppare automaticamente i volti simili in cluster di persone e mostrare per ciascuna il numero di foto; i cluster senza nome SHALL apparire come "Senza nome".

#### Scenario: Stessa persona in più foto
- **WHEN** la stessa persona compare in 20 foto
- **THEN** i suoi volti sono raggruppati in un'unica persona con conteggio 20

### Requirement: Identificazione delle persone
L'utente SHALL poter assegnare un nome a una persona; i nuovi volti sufficientemente simili a una persona con nome SHALL essere associati automaticamente a quella persona.

#### Scenario: Nome assegnato e propagato
- **WHEN** l'utente chiama "Giulia" un cluster e poi importa nuove foto in cui Giulia compare
- **THEN** le nuove foto riportano "Persone: Giulia"

### Requirement: Correzione dei raggruppamenti
L'utente SHALL poter unire due persone, separare volti da una persona, indicare "non è questa persona" e rimuovere un volto; le correzioni MUST essere rispettate nei raggruppamenti successivi.

#### Scenario: Volto rifiutato
- **WHEN** l'utente indica che un volto non appartiene a "Marco"
- **THEN** quel volto non viene più associato a "Marco" anche dopo un nuovo raggruppamento

#### Scenario: Unione persone
- **WHEN** l'utente unisce "Senza nome 12" con "Giulia"
- **THEN** tutti i volti risultano di "Giulia" e "Senza nome 12" non esiste più

### Requirement: Privacy dei dati biometrici
I dati dei volti (ritagli ed embedding) MUST restare esclusivamente sul computer dell'utente; l'utente SHALL poter disattivare il riconoscimento volti e cancellare tutti i dati dei volti in un'unica azione.

#### Scenario: Cancellazione dati volti
- **WHEN** l'utente sceglie "Elimina tutti i dati dei volti"
- **THEN** volti, embedding e persone vengono eliminati e il riconoscimento resta disattivato finché l'utente non lo riattiva

#### Scenario: Riconoscimento disattivato
- **WHEN** il riconoscimento volti è disattivato
- **THEN** l'analisi non calcola embedding facciali ma continua a contare le persone presenti
