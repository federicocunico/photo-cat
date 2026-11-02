# Spec Delta

## MODIFIED Requirements

### Requirement: Indicatori globali
La barra superiore SHALL mostrare sempre un indicatore dei job attivi con percentuale complessiva e uno di consumo RAM, e la barra di stato SHALL mostrare numero di elementi, selezionati, stato dei modelli e numero di errori recenti, con accesso diretto alle rispettive schermate. L'indicatore dei job SHALL animarsi solo quando ci sono job attivi, e la barra superiore SHALL mostrare un indicatore distinto quando richieste dell'interfaccia al motore sono in corso da più di 300 ms.

#### Scenario: Job in corso da altro modulo
- **WHEN** è in corso un'analisi e l'utente è in Libreria
- **THEN** la barra superiore mostra la percentuale dell'analisi e un clic porta alla schermata Job

#### Scenario: Nessun job attivo
- **WHEN** non ci sono job attivi
- **THEN** l'indicatore dei job è fermo e non mostra animazioni

#### Scenario: Richiesta lenta
- **WHEN** l'utente applica un filtro il cui calcolo richiede 2 secondi
- **THEN** dopo 300 ms compare nella barra superiore l'indicatore di caricamento, che scompare all'arrivo dei risultati

## ADDED Requirements

### Requirement: Stati di caricamento
Ogni elenco, griglia o pannello che mostra dati SHALL distinguere tre stati: in caricamento (indicatore o segnaposto), vuoto (messaggio di stato vuoto) ed errore (messaggio con "Riprova"). Lo stato vuoto MUST NOT essere mostrato prima che i dati siano arrivati. Quando un dato già visibile si sta aggiornando, il pannello SHALL segnalarlo senza nascondere il contenuto.

#### Scenario: Elenco delle sorgenti in caricamento
- **WHEN** l'utente apre la pagina Import e l'elenco delle sorgenti non è ancora arrivato
- **THEN** la pagina mostra un segnaposto di caricamento e non il messaggio "nessuna sorgente"

#### Scenario: Elenco vuoto
- **WHEN** l'elenco delle persone è arrivato ed è vuoto
- **THEN** la pagina Persone mostra lo stato vuoto con l'indicazione di come ottenere persone

#### Scenario: Errore di caricamento
- **WHEN** il caricamento dei cataloghi fallisce
- **THEN** la pagina mostra il messaggio d'errore e "Riprova", che ripete la richiesta

#### Scenario: Primo caricamento della griglia
- **WHEN** l'utente apre la Libreria con 100.000 foto
- **THEN** la griglia mostra segnaposto delle miniature e il conteggio "in calcolo…" finché il totale non è noto

#### Scenario: Aggiornamento in background
- **WHEN** la griglia mostra dei risultati e l'utente cambia filtro
- **THEN** i risultati precedenti restano visibili attenuati con una barra di avanzamento sottile finché arrivano i nuovi

### Requirement: Azioni in corso
Un comando che avvia un'operazione SHALL mostrare che è in corso (indicatore nel comando) e MUST NOT poter essere avviato di nuovo finché l'operazione non termina. Le operazioni che durano più di un secondo SHALL concludersi con un avviso non modale che ne riporta l'esito.

#### Scenario: Doppio clic su Salva
- **WHEN** l'utente fa doppio clic su "Salva" nell'editor di un'identità
- **THEN** il salvataggio viene inviato una sola volta e il pulsante mostra lo stato "in corso" fino all'esito

#### Scenario: Esito di un'operazione lenta
- **WHEN** l'utente unisce due tag con 5.000 foto e l'operazione dura 3 secondi
- **THEN** al termine compare un avviso con l'esito, che si chiude da solo

### Requirement: Schermata di avvio
All'avvio l'app SHALL mostrare subito una finestra con una schermata di avvio che indica la fase in corso (apertura del catalogo, backup e migrazione, caricamento del runtime AI). La Libreria o l'onboarding SHALL comparire solo quando la configurazione è stata letta. Un errore bloccante SHALL essere mostrato nella schermata con il percorso dei log.

#### Scenario: Migrazione del catalogo
- **WHEN** l'app si avvia dopo un aggiornamento che migra un catalogo grande
- **THEN** la finestra compare subito con "Backup e aggiornamento del catalogo…" invece di restare invisibile

#### Scenario: Primo avvio
- **WHEN** l'onboarding non è ancora stato completato
- **THEN** dopo la schermata di avvio compare direttamente l'onboarding, senza mostrare prima la Libreria

#### Scenario: Errore all'avvio
- **WHEN** il catalogo non si apre perché il disco dati non è disponibile
- **THEN** la schermata di avvio mostra l'errore, il percorso dei log e i comandi per riprovare o scegliere un'altra cartella dati
