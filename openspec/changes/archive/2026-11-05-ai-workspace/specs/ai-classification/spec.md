# Spec Delta

## ADDED Requirements

### Requirement: Etichette automatiche rifiutate
Quando l'utente toglie un'etichetta automatica da una foto, il sistema SHALL ricordare il rifiuto, e le analisi, le rianalisi e la riapplicazione delle soglie successive MUST NOT riassegnare quella voce a quella foto. Il rifiuto SHALL decadere se l'utente assegna a mano la stessa voce o annulla l'operazione. Le altre voci dello stesso gruppo SHALL continuare a essere assegnate automaticamente.

#### Scenario: Rianalisi dopo il rifiuto
- **WHEN** l'utente toglie l'etichetta automatica "Natura" da una foto e poi la foto viene rianalizzata in modalità "forza"
- **THEN** la foto non ha "Natura" ma riceve le altre voci del gruppo che superano la soglia

#### Scenario: Soglie abbassate
- **WHEN** l'utente abbassa la soglia della voce "Natura" e applica le soglie senza rianalisi
- **THEN** le foto in cui "Natura" è stata rifiutata non la ricevono

#### Scenario: Assegnazione manuale dopo il rifiuto
- **WHEN** l'utente assegna a mano "Natura" a una foto da cui l'aveva tolta
- **THEN** la foto ha "Natura" come etichetta manuale e il rifiuto non è più attivo
