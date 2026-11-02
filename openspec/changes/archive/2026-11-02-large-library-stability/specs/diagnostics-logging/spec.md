# Spec Delta

## MODIFIED Requirements

### Requirement: Cattura dei crash
Il sistema SHALL registrare nei log gli errori fatali e i panic prima della chiusura e, al riavvio successivo, segnalare all'utente che la sessione precedente si è chiusa in modo anomalo. Questo vale anche per la terminazione del processo dell'interfaccia (WebView), per gli abort nativi e per le chiusure forzate del sistema: una sessione non conclusa regolarmente SHALL essere riconosciuta al riavvio anche quando non è stato possibile scrivere nulla al momento del crash.

#### Scenario: Panic
- **WHEN** un componente va in panic
- **THEN** il log contiene messaggio e posizione del panic e al riavvio l'utente vede l'avviso con collegamento alla diagnostica

#### Scenario: Interfaccia terminata
- **WHEN** il processo della WebView termina in modo anomalo mentre l'app è aperta
- **THEN** il log registra l'evento con il motivo fornito dal sistema, la memoria disponibile in quel momento e i job attivi, e l'app prova a ricaricare l'interfaccia

#### Scenario: Chiusura forzata senza log
- **WHEN** l'app viene terminata dal sistema senza poter scrivere nulla
- **THEN** al riavvio l'utente vede l'avviso che la sessione precedente si è chiusa in modo anomalo, con l'ora dell'ultima attività registrata

#### Scenario: Dump nel pacchetto diagnostico
- **WHEN** dopo un crash della WebView l'utente crea il pacchetto diagnostico
- **THEN** il pacchetto include i dump di crash della WebView generati dopo l'ultimo avvio regolare e un riepilogo della memoria registrata prima del crash

## ADDED Requirements

### Requirement: Errori ripetuti raggruppati
Durante un job, gli errori per file con lo stesso codice e la stessa fase SHALL essere raggruppati: l'interfaccia SHALL ricevere aggiornamenti aggregati a intervalli e la diagnostica SHALL mostrare un riepilogo per codice con il numero di file e l'elenco consultabile dei file coinvolti. Ogni errore SHALL comunque restare registrato singolarmente nel registro errori.

#### Scenario: Migliaia di file non decodificabili
- **WHEN** un import di 100.000 file incontra 1.240 file non decodificabili
- **THEN** l'interfaccia resta reattiva, la diagnostica mostra "DECODE_FAILED — 1.240 file" nella fase miniature, e aprendo la voce si vedono i file con il motivo di ciascuno

#### Scenario: Errori diversi
- **WHEN** nello stesso job compaiono errori `DECODE_FAILED` e `UNSUPPORTED_FORMAT`
- **THEN** la diagnostica li mostra come due righe distinte con i rispettivi conteggi
