# variant-generation Specification

## Purpose
Sviluppa su richiesta, con la fotocamera collegata, i RAF di una sessione di revisione in tutte le varianti JPEG richieste. Scrive una sottocartella per preset nel layout che la revisione RAW legge direttamente.

## Requirements

### Requirement: Scatti da sviluppare
Dalla sessione di revisione l'utente SHALL poter sviluppare tutti gli scatti RAF della cartella oppure solo quelli selezionati. Prima dell'avvio il sistema SHALL mostrare:
- quanti RAF verranno sviluppati;
- quante conversioni risultano in totale (RAF × preset) e quante sono già presenti;
- lo spazio stimato rispetto a quello libero;
- il tempo stimato.

#### Scenario: Cartella di uno shooting
- **WHEN** la cartella contiene 160 RAF e 160 JPG e l'utente sceglie 7 preset per tutti gli scatti
- **THEN** il sistema indica 160 RAF e 1.120 conversioni, e i JPG presenti accanto ai RAF vengono ignorati

#### Scenario: Solo gli scatti selezionati
- **WHEN** l'utente seleziona 5 scatti e sceglie "Sviluppa selezionati" con 3 preset
- **THEN** il sistema indica 5 RAF e 15 conversioni e sviluppa solo quelli

### Requirement: Layout di output
Il sistema SHALL scrivere ogni variante in `<cartella di output>/<cartella del preset>/<nome RAF>.jpg`. La cartella di output predefinita è `<cartella aperta>/FujiView varianti` e l'utente SHALL poterla cambiare. Al termine la sessione di revisione SHALL mostrare le nuove varianti senza riaprire la cartella.

#### Scenario: Output pronto per il confronto
- **WHEN** lo sviluppo termina con successo
- **THEN** ogni cartella preset contiene un JPEG per ogni RAF sviluppato con lo stesso nome base (es. `DSCF9131.jpg`) e i pannelli dei nuovi preset compaiono nella revisione

### Requirement: Protezione dei file originali
Il sistema MUST NOT modificare, spostare o cancellare i RAF né alcun file fuori dalle cartelle di output dei preset. Ogni JPEG SHALL essere scritto in un file temporaneo e rinominato solo a scrittura completata.

#### Scenario: Sviluppo interrotto
- **WHEN** l'app viene chiusa a metà di uno sviluppo
- **THEN** nelle cartelle preset non restano JPEG parziali e i RAF sorgente sono identici a prima (stessa dimensione e hash)

### Requirement: Ripresa e idempotenza
Una variante già presente e completa SHALL essere saltata. Rilanciare lo stesso sviluppo MUST produrre solo le varianti mancanti.

#### Scenario: Ripresa dopo un'interruzione
- **WHEN** uno sviluppo da 1.120 conversioni si interrompe dopo 400 e viene rilanciato
- **THEN** il sistema esegue solo le 720 mancanti e il riepilogo le indica come "saltate: 400"

#### Scenario: Risviluppare forzatamente
- **WHEN** l'utente attiva "Sovrascrivi varianti esistenti"
- **THEN** tutte le varianti selezionate vengono rigenerate

### Requirement: Sviluppo come attività
Lo sviluppo SHALL essere un'attività di photo-cat, visibile in Attività e nella barra di stato. Mostra RAF e preset correnti, conteggio di completate e totali, tempo residuo stimato ed errori. L'utente SHALL poter mettere in pausa, riprendere e annullare, e l'annullamento MUST terminare entro la conversione in corso. Lo sviluppo SHALL proseguire anche se l'utente cambia modulo.

#### Scenario: Annullamento
- **WHEN** l'utente preme "Annulla" durante lo sviluppo
- **THEN** la conversione in corso termina o viene scartata, nessun file parziale resta e la sessione con la camera viene chiusa

#### Scenario: Cambio di modulo
- **WHEN** l'utente avvia lo sviluppo e passa alla Libreria
- **THEN** lo sviluppo continua e la barra di stato ne mostra l'avanzamento

### Requirement: Errori per singolo file
Un RAF che la camera rifiuta SHALL essere segnalato e saltato senza fermare lo sviluppo; il riepilogo finale MUST elencare file e motivo, e gli errori SHALL essere registrati nel registro errori.

#### Scenario: RAF di un altro corpo macchina
- **WHEN** la cartella contiene un RAF scattato con una camera diversa da quella collegata
- **THEN** quel RAF viene saltato con il motivo "RAF non scattato da questa fotocamera" e gli altri vengono sviluppati

### Requirement: Controllo dello spazio su disco
Prima di avviare lo sviluppo il sistema SHALL stimare lo spazio necessario e MUST rifiutare l'avvio se lo spazio libero sul disco di output non basta lasciando un margine di sicurezza. Durante lo sviluppo MUST mettersi in pausa se lo spazio scende sotto il margine.

#### Scenario: Disco quasi pieno
- **WHEN** lo spazio stimato supera lo spazio libero meno il margine
- **THEN** lo sviluppo non parte e il sistema mostra spazio richiesto e disponibile
