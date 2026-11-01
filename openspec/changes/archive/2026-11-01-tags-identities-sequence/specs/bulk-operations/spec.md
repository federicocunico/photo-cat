# Spec Delta

## ADDED Requirements

### Requirement: Esclusività dei set nelle azioni bulk
Quando un'azione bulk o un tasto rapido aggiunge a una selezione un tag che appartiene a un set esclusivo, il sistema SHALL rimuovere dagli stessi file gli altri tag di quel set; l'operazione SHALL essere annullabile come un'unica azione.

#### Scenario: Tag in bulk con set esclusivo
- **WHEN** 40 file selezionati hanno "Paesaggi" del set esclusivo "Viaggio USA 2026" e l'utente aggiunge "Città" in bulk
- **THEN** i 40 file hanno "Città" e non hanno più "Paesaggi"

#### Scenario: Annulla
- **WHEN** l'utente sceglie "Annulla" subito dopo
- **THEN** i 40 file tornano ad avere "Paesaggi" e non hanno "Città"

### Requirement: Tagging da tastiera sulla selezione
In Libreria e in Sequenza i tasti 1–9 e 0 del set attivo SHALL agire sull'intera selezione corrente, inclusi i file selezionati ma non visibili, e il risultato SHALL comparire senza ricaricare la vista.

#### Scenario: Selezione estesa
- **WHEN** l'utente seleziona tutti i 2.340 risultati di un filtro e preme 2
- **THEN** i 2.340 file ricevono il tag in posizione 2 del set attivo
