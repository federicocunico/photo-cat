# Spec Delta

## Purpose

Rende esplicite le informazioni sull'app installata (versione, build, sistema, autori, licenza e licenze dei componenti di terze parti), consultabili dalla finestra Informazioni e da Impostazioni, e facili da copiare in una segnalazione.

## ADDED Requirements

### Requirement: Finestra Informazioni
Aiuto › "Informazioni su photo-cat" SHALL aprire una finestra con due schede, "Informazioni" e "Licenze di terze parti". La scheda "Informazioni" SHALL mostrare nome e versione dell'app, la build, il sistema operativo con l'architettura, la versione del motore della finestra, l'acceleratore usato dall'analisi AI e il percorso dei dati dell'app con un comando per aprirlo.

#### Scenario: Versione esplicita
- **WHEN** l'utente apre Aiuto › "Informazioni su photo-cat"
- **THEN** la finestra mostra "photo-cat 0.5.1" e, alla voce Build, il commit abbreviato, la data della build e la configurazione (ad esempio "release")

#### Scenario: Aprire la cartella dei dati
- **WHEN** l'utente sceglie "Apri" accanto al percorso dei dati
- **THEN** si apre la cartella dei dati nel gestore di file del sistema

### Requirement: Dati della build
I dati della build SHALL essere fissati al momento della compilazione: commit abbreviato, data in formato della lingua dell'interfaccia e configurazione. Se il commit non è disponibile, ad esempio per una build da un archivio dei sorgenti, la voce SHALL indicarlo in parole invece di restare vuota.

#### Scenario: Build senza repository git
- **WHEN** l'app è compilata da un archivio dei sorgenti senza cartella git
- **THEN** la voce Build mostra "commit non disponibile" con data e configurazione

### Requirement: Autori e licenza del progetto
La scheda "Informazioni" SHALL indicare gli autori, il collegamento al codice sorgente e la licenza del progetto, letti dai metadati del pacchetto e non scritti a mano nell'interfaccia.

#### Scenario: Licenza del progetto
- **WHEN** l'utente apre la scheda "Informazioni"
- **THEN** vede gli autori "photo-cat contributors" e la licenza "MIT oppure Apache-2.0, a scelta", come nei metadati del pacchetto

### Requirement: Licenze di terze parti
La scheda "Licenze di terze parti" SHALL elencare ogni componente distribuito con l'app (modelli AI e dati, librerie native, componenti dell'interfaccia, componenti Rust) con nome, versione e licenza, raggruppati per tipo e con una ricerca. Per ogni componente SHALL essere possibile leggere il testo completo della licenza. Per i componenti con obbligo di sorgenti SHALL essere indicato dove si trovano nell'installazione.

#### Scenario: Ricerca di una licenza
- **WHEN** l'utente scrive "LGPL" nella ricerca delle licenze
- **THEN** compaiono libheif e libde265, ciascuno con il testo della licenza e l'indicazione della cartella delle sorgenti corrispondenti

#### Scenario: Elenco completo
- **WHEN** viene preparata una build di rilascio
- **THEN** l'elenco delle licenze comprende tutte le dipendenze Rust e dell'interfaccia incluse nella build, e la build fallisce se una dipendenza non ha una licenza riconosciuta

### Requirement: Copia delle informazioni
La finestra Informazioni SHALL offrire "Copia informazioni", che mette negli appunti in testo semplice versione, build, sistema, motore della finestra e acceleratore. Il percorso dei dati MUST NOT essere copiato se l'utente non lo chiede esplicitamente. Lo stesso comando SHALL essere disponibile come Aiuto › "Copia le informazioni di sistema" e nella palette dei comandi.

#### Scenario: Informazioni per una segnalazione
- **WHEN** l'utente sceglie "Copia informazioni" e incolla il testo in un messaggio
- **THEN** il testo contiene versione, build, sistema, motore della finestra e acceleratore, uno per riga, senza il percorso dei dati

### Requirement: Informazioni in Impostazioni
Impostazioni SHALL avere in fondo una sezione "Informazioni" con lo stesso contenuto della finestra Informazioni.

#### Scenario: Versione dalle impostazioni
- **WHEN** l'utente apre Impostazioni › Informazioni
- **THEN** vede versione e build e può aprire le licenze di terze parti senza passare dal menu Aiuto
