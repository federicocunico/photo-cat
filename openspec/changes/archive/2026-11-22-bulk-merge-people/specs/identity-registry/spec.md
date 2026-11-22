# Spec Delta

## ADDED Requirements

### Requirement: Unione di più identità
Nella sezione "Persone" del modulo Persone l'utente SHALL poter selezionare più identità (clic, Ctrl/Maiusc+clic, Ctrl+A, Esc), filtrarle per numero massimo di foto nella libreria aperta e unirle con "Unisci in…" in un'identità di destinazione scelta con ricerca tra tutte le identità. L'unione SHALL avvenire in un'unica operazione, dopo una conferma che indica quante identità, foto e volti di riferimento verranno uniti.

#### Scenario: Molte identità piccole
- **WHEN** l'utente filtra "con al massimo 2 foto", seleziona tutte le 340 identità risultanti e sceglie "Unisci in…" › "Sconosciuti"
- **THEN** dopo la conferma esiste solo "Sconosciuti", che ha i loro alias, volti di riferimento e foto, e le 340 identità non esistono più

#### Scenario: Persone della libreria collegate
- **WHEN** tre delle identità unite sono collegate a persone della libreria aperta
- **THEN** i volti di quelle persone risultano della persona collegata alla destinazione, oppure, se la destinazione non è collegata, la prima persona diventa la sua e le altre vi vengono unite

#### Scenario: Destinazione tra le selezionate
- **WHEN** l'utente seleziona "G. Bianchi", "Giulia B." e "Giulia Bianchi" e sceglie come destinazione "Giulia Bianchi"
- **THEN** le altre due vengono unite in "Giulia Bianchi", che non viene eliminata

#### Scenario: Errore a metà
- **WHEN** l'unione di 50 identità fallisce alla trentesima
- **THEN** nessuna delle 50 identità è stata modificata e l'utente vede il motivo dell'errore
