# face-recognition Specification

## Purpose
Rileva i volti nelle foto, li raggruppa per somiglianza in persone e permette all'utente di dare un nome alle persone e correggere i raggruppamenti, mantenendo i dati biometrici solo in locale.

## Requirements

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
L'utente SHALL poter assegnare a una persona un'identità dell'archivio dell'app, scegliendone una esistente o creandone una nuova; i nuovi volti sufficientemente simili a una persona collegata o ai volti di riferimento di un'identità SHALL essere associati automaticamente a quella identità, con la soglia di somiglianza dell'identità.

#### Scenario: Nome assegnato e propagato
- **WHEN** l'utente collega un cluster all'identità "Giulia" e poi importa nuove foto in cui Giulia compare
- **THEN** le nuove foto riportano "Persone: Giulia"

#### Scenario: Volto sotto soglia
- **WHEN** un nuovo volto ha somiglianza con "Giulia" inferiore alla soglia dell'identità (o, se non impostata, a quella configurata)
- **THEN** il volto non viene associato automaticamente a "Giulia"

#### Scenario: Riconoscimento in una libreria nuova
- **WHEN** l'identità "Giulia" ha volti di riferimento e l'utente analizza una libreria nuova in cui Giulia compare
- **THEN** i volti di Giulia vengono raggruppati in una persona collegata all'identità "Giulia" senza che l'utente la nomini

#### Scenario: Suggerimento in "Chi è?"
- **WHEN** l'utente scrive "Giu" nel campo "Chi è?" di un gruppo senza nome
- **THEN** il sistema propone le identità il cui nome o alias contiene "Giu", ordinate per somiglianza con i volti del gruppo

### Requirement: Correzione dei raggruppamenti
L'utente SHALL poter unire due persone, separare volti da una persona, indicare "non è questa persona" e rimuovere un volto; le correzioni MUST essere rispettate nei raggruppamenti successivi.

#### Scenario: Volto rifiutato
- **WHEN** l'utente indica che un volto non appartiene a "Marco"
- **THEN** quel volto non viene più associato a "Marco" anche dopo un nuovo raggruppamento

#### Scenario: Unione persone
- **WHEN** l'utente unisce "Senza nome 12" con "Giulia"
- **THEN** tutti i volti risultano di "Giulia" e "Senza nome 12" non esiste più

### Requirement: Privacy dei dati biometrici
I dati dei volti (ritagli ed embedding, inclusi i volti di riferimento delle identità) MUST restare esclusivamente sul computer dell'utente; l'utente SHALL poter disattivare il riconoscimento volti e cancellare tutti i dati dei volti in un'unica azione.

#### Scenario: Cancellazione dati volti
- **WHEN** l'utente sceglie "Elimina tutti i dati dei volti"
- **THEN** volti, embedding, persone e volti di riferimento delle identità vengono eliminati, le identità restano con nome, alias e gruppo, e il riconoscimento resta disattivato finché l'utente non lo riattiva

#### Scenario: Riconoscimento disattivato
- **WHEN** il riconoscimento volti è disattivato
- **THEN** l'analisi non calcola embedding facciali ma continua a contare le persone presenti
