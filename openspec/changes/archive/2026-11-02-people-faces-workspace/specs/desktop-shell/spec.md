# Spec Delta

## MODIFIED Requirements

### Requirement: Moduli di navigazione
L'applicazione SHALL offrire i moduli Libreria, Cataloghi, Persone, Importa, Job, Esporta e organizza, Modelli AI, Diagnostica e Impostazioni, e la navigazione SHALL essere estendibile con nuovi moduli. Persone e identità SHALL essere gestite in un unico modulo Persone. Un modulo SHALL poter essere aperto su un elemento specifico (ad esempio una persona o un volto) da altri moduli.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria a Job e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

#### Scenario: Identità nel modulo Persone
- **WHEN** l'utente cerca nella navigazione il modulo Identità
- **THEN** non lo trova come voce separata e le identità sono gestite nella sezione "Persone" del modulo Persone

#### Scenario: Preferenza del modulo non più esistente
- **WHEN** l'ultimo modulo aperto salvato nelle preferenze era "Identità"
- **THEN** all'avvio si apre il modulo Persone sulla sezione "Persone"

## ADDED Requirements

### Requirement: Revisione delle persone
Il modulo Persone SHALL presentare un elenco diviso nelle sezioni "Da rivedere", "Persone" e "Ignorati" con conteggi e ricerca, i volti dell'elemento scelto in una griglia con selezione multipla, e una scheda con "Chi è?", unione e ignora. L'elenco e la griglia SHALL restare fluidi con 100.000 volti, caricando i dati a pagine, e SHALL aggiornarsi quando cambiano i dati dei volti senza ricaricamenti periodici. Tutte le azioni SHALL essere disponibili da tastiera.

#### Scenario: Revisione da tastiera
- **WHEN** l'utente scorre "Da rivedere" con le frecce, preme N, scrive un nome e conferma
- **THEN** il gruppo diventa una persona con quel nome e il focus passa al gruppo successivo da rivedere

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 volti in 3.000 gruppi
- **THEN** l'elenco e la griglia dei volti si aprono e scorrono senza blocchi percepibili

#### Scenario: Aggiornamento dopo l'analisi
- **WHEN** termina un'analisi dei volti mentre il modulo Persone è aperto
- **THEN** elenco e conteggi si aggiornano senza perdere la selezione e la posizione di scorrimento
