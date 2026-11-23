# app-updates Specification

## Purpose
TBD - created by archiving change app-updates. Update Purpose after archive.

## Requirements

### Requirement: Controllo delle nuove versioni
L'app SHALL controllare se esiste una versione più recente leggendo l'ultima release pubblicata del progetto su GitHub (escluse bozze e pre-release). Il controllo SHALL avvenire all'avvio, dopo che la libreria è pronta, se l'impostazione "Controlla gli aggiornamenti all'avvio" è attiva (predefinita), e su richiesta da Aiuto › "Controlla aggiornamenti…" e da Impostazioni › Informazioni. La richiesta MUST NOT contenere dati dell'utente o della libreria: solo l'User-Agent "photo-cat/<versione>". Un errore di rete durante il controllo automatico MUST NOT mostrare avvisi; durante il controllo su richiesta SHALL essere spiegato.

#### Scenario: Versione più recente disponibile
- **WHEN** l'app in uso è la 0.5.3 e l'ultima release pubblicata è la v0.5.4
- **THEN** compare la notifica "È disponibile photo-cat 0.5.4 (in uso 0.5.3)" con "Novità", "Aggiorna" e il comando per chiuderla

#### Scenario: Già aggiornata
- **WHEN** l'utente sceglie "Controlla aggiornamenti…" e l'ultima release è quella in uso
- **THEN** l'app dice "photo-cat 0.5.3 è la versione più recente" e non mostra la notifica

#### Scenario: Senza connessione
- **WHEN** il controllo automatico all'avvio non raggiunge GitHub
- **THEN** non compare alcun avviso e l'app funziona come sempre

### Requirement: Notifica che si può chiudere
La notifica di una nuova versione SHALL potersi chiudere. Chiusa, non SHALL ricomparire fino al prossimo avvio dell'app o al controllo su richiesta. "Novità" SHALL aprire la pagina della release nel browser.

#### Scenario: Notifica chiusa
- **WHEN** l'utente chiude la notifica della 0.5.4
- **THEN** la notifica sparisce e non ricompare durante la sessione, anche se un controllo automatico la ritrova

### Requirement: Download dell'aggiornamento in background
Su Windows "Aggiorna" SHALL scaricare in background l'installer della release ("photo-cat_<versione>_x64-setup.exe") nella cartella `updates` dei dati dell'app, mentre l'utente continua a lavorare. L'avanzamento SHALL essere visibile con una barra determinata (percentuale e megabyte) nella notifica e nella barra superiore. Il download SHALL riprendere dal punto raggiunto se la connessione cade, SHALL potersi interrompere e SHALL essere verificato con lo SHA-256 pubblicato nella release prima di essere considerato pronto. Un file che non corrisponde MUST NOT essere usato.

#### Scenario: Download con avanzamento
- **WHEN** l'utente sceglie "Aggiorna" sulla notifica della 0.5.4
- **THEN** la notifica e la barra superiore mostrano "Download dell'aggiornamento 42% · 199 di 473 MB" mentre l'utente continua a usare la Libreria

#### Scenario: File non integro
- **WHEN** lo SHA-256 dell'installer scaricato non corrisponde a quello della release
- **THEN** il file viene eliminato e la notifica dice che il download non è valido, con "Riprova"

#### Scenario: Aggiornamento già scaricato
- **WHEN** l'installer della 0.5.4, già scaricato e verificato, è nella cartella `updates` al riavvio dell'app
- **THEN** l'app non lo scarica di nuovo e mostra subito il badge "Aggiornamento pronto"

### Requirement: Installazione dell'aggiornamento
A download completato SHALL comparire nella barra superiore il badge "Aggiornamento pronto", che chiede se aggiornare ora. "Aggiorna ora" SHALL avviare l'installer scaricato e chiudere photo-cat in modo regolare, così che l'utente prosegua con l'installazione. "Più tardi" SHALL lasciare il badge, che resta disponibile.

#### Scenario: Aggiorna ora
- **WHEN** l'utente sceglie il badge "Aggiornamento pronto" e poi "Aggiorna ora"
- **THEN** parte l'installer di photo-cat 0.5.4 e la finestra di photo-cat si chiude; catalogo e impostazioni restano

#### Scenario: Più tardi
- **WHEN** l'utente sceglie "Più tardi"
- **THEN** il badge resta nella barra superiore e l'installazione si può avviare in un secondo momento

### Requirement: Aggiornamento su macOS e Linux
Su macOS e Linux la notifica SHALL comparire allo stesso modo, ma "Aggiorna" SHALL aprire la pagina della release nel browser, perché DMG e pacchetti si installano con gli strumenti del sistema. L'app MUST NOT scaricare né avviare installer su queste piattaforme.

#### Scenario: Linux
- **WHEN** su Linux compare la notifica della 0.5.4 e l'utente sceglie "Aggiorna"
- **THEN** si apre la pagina della release v0.5.4 nel browser

### Requirement: Pulizia degli installer
All'avvio l'app SHALL eliminare dalla cartella `updates` gli installer e i download parziali di versioni uguali o precedenti a quella in uso.

#### Scenario: Dopo l'aggiornamento
- **WHEN** photo-cat 0.5.4 si avvia per la prima volta dopo l'installazione
- **THEN** l'installer della 0.5.4 viene eliminato dalla cartella `updates`
