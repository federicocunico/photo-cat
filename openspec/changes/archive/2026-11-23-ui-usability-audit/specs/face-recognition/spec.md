# Spec Delta

## ADDED Requirements

### Requirement: Comandi di correzione dei volti
Le correzioni SHALL usare due soli verbi: "Sposta in…" per volti selezionati e "Unisci a…" per gruppi o persone. Le azioni sui volti SHALL comparire in una barra che sostituisce l'intestazione dei volti solo quando c'è almeno un volto selezionato: "Sposta in…", "Non è questa persona" e un menu "Altro" con "Nuovo gruppo con questi volti", "Usa come copertina" (un solo volto) e "Rimuovi volti…".

#### Scenario: Volti selezionati
- **WHEN** l'utente seleziona tre volti di un gruppo
- **THEN** al posto dell'intestazione compare "3 selezionati" con "Sposta in…", "Non è questa persona", "Altro" e "Deseleziona", su una riga

#### Scenario: Copertina dal volto
- **WHEN** l'utente seleziona un solo volto e apre "Altro"
- **THEN** il menu contiene "Usa come copertina", e il volto diventa la copertina del gruppo

#### Scenario: Unire un gruppo
- **WHEN** l'utente sceglie "Unisci a…" dalla scheda di un gruppo senza nome e sceglie "Gioia"
- **THEN** tutti i volti del gruppo passano a Gioia e un avviso dice "Gruppo unito a Gioia" con "Annulla operazione"

### Requirement: Selettore della destinazione
"Sposta in…" e "Unisci a…" SHALL aprire lo stesso selettore con ricerca, che elenca prima le persone con nome e poi i gruppi senza nome. Ogni voce SHALL mostrare un volto di copertina, il nome e il numero di foto. Il selettore SHALL permettere di creare una persona nuova con il nome scritto nella ricerca.

#### Scenario: Gruppi senza nome distinguibili
- **WHEN** l'utente apre "Unisci a…" e ci sono sei gruppi senza nome
- **THEN** ogni gruppo senza nome compare con il proprio volto di copertina e il numero di foto, così che le voci si distinguono a colpo d'occhio

#### Scenario: Persona nuova dal selettore
- **WHEN** l'utente scrive "Marta" nel selettore e nessuna persona ha quel nome
- **THEN** il selettore propone "Nuova persona «Marta»", che crea la persona e vi sposta i volti

### Requirement: Suggerimento di identità nella scheda
Quando un gruppo senza nome somiglia a una persona o a un altro gruppo, la scheda SHALL mostrare in cima la domanda "È {nome}?", oppure "È la stessa persona?" se l'altro gruppo non ha nome, con i due volti di copertina affiancati e i comandi "Sì" e "No". La somiglianza SHALL essere indicata in parole (alta, media), senza valori numerici. Nell'elenco il gruppo SHALL riportare "forse {nome}".

#### Scenario: Conferma del suggerimento
- **WHEN** l'utente apre un gruppo che somiglia a Gioia e sceglie "Sì"
- **THEN** il gruppo viene unito a Gioia, si apre il gruppo successivo dell'elenco e l'avviso offre "Annulla operazione"

#### Scenario: Rifiuto del suggerimento
- **WHEN** l'utente sceglie "No" alla domanda "È Gioia?"
- **THEN** il suggerimento non viene più proposto per quei due elementi, come con "Persone diverse"
