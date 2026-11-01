# Spec Delta

## MODIFIED Requirements

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

### Requirement: Privacy dei dati biometrici
I dati dei volti (ritagli ed embedding, inclusi i volti di riferimento delle identità) MUST restare esclusivamente sul computer dell'utente; l'utente SHALL poter disattivare il riconoscimento volti e cancellare tutti i dati dei volti in un'unica azione.

#### Scenario: Cancellazione dati volti
- **WHEN** l'utente sceglie "Elimina tutti i dati dei volti"
- **THEN** volti, embedding, persone e volti di riferimento delle identità vengono eliminati, le identità restano con nome, alias e gruppo, e il riconoscimento resta disattivato finché l'utente non lo riattiva

#### Scenario: Riconoscimento disattivato
- **WHEN** il riconoscimento volti è disattivato
- **THEN** l'analisi non calcola embedding facciali ma continua a contare le persone presenti
