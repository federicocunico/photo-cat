# camera-connection Specification

## Purpose
Permette a photo-cat di trovare una fotocamera Fujifilm collegata via USB in modalità USB RAW CONV./BACKUP RESTORE e di dialogare con essa in modo affidabile su Windows, senza driver aggiuntivi e senza X RAW Studio.

## Requirements

### Requirement: Rilevamento della camera
Su Windows il sistema SHALL elencare le fotocamere Fujifilm collegate mostrando modello, firmware e se sono in una modalità che consente la conversione RAW. Il rilevamento MUST funzionare col driver Windows predefinito, senza installare driver o strumenti di terze parti.

#### Scenario: X-T5 in modalità USB RAW CONV
- **WHEN** una X-T5 impostata su USB RAW CONV./BACKUP RESTORE è collegata e accesa
- **THEN** il sistema mostra "X-T5" con la versione firmware e lo stato "pronta per la conversione"

#### Scenario: Camera in modalità non compatibile
- **WHEN** la camera è collegata in una modalità USB diversa (es. lettore di schede, webcam)
- **THEN** il sistema la segnala come non pronta e indica di impostare USB RAW CONV./BACKUP RESTORE nel menu della camera

#### Scenario: Nessuna camera
- **WHEN** nessuna camera Fujifilm è collegata o la camera è spenta o in standby
- **THEN** il sistema mostra "nessuna fotocamera rilevata" e permette di ripetere la ricerca senza riavviare l'app

### Requirement: Disponibilità per piattaforma
Su Linux e macOS lo sviluppo dalla fotocamera MUST NOT essere offerto come azione eseguibile. L'interfaccia SHALL indicare che è disponibile solo su Windows, e il resto della revisione RAW SHALL funzionare normalmente.

#### Scenario: Revisione su Linux
- **WHEN** l'utente apre Varianti RAW (Fujifilm X-T5) su Linux
- **THEN** la voce "Sviluppa dalla fotocamera" è disattivata con il motivo "Disponibile solo su Windows" e confronto, valutazione ed export funzionano

### Requirement: Sessione esclusiva
Il sistema SHALL aprire una sola sessione alla volta con la camera e chiuderla al termine o all'annullamento. Se la camera è già in uso da un altro programma, il sistema MUST segnalarlo invece di fallire in modo silenzioso.

#### Scenario: X RAW Studio aperto
- **WHEN** X RAW Studio sta già usando la camera e l'utente avvia uno sviluppo
- **THEN** il sistema mostra che la camera è occupata e chiede di chiudere l'altro programma

#### Scenario: Sessione rimasta aperta
- **WHEN** una sessione precedente non è stata chiusa correttamente (es. crash dell'app)
- **THEN** il sistema la chiude e ne riapre una nuova automaticamente

### Requirement: Gestione della disconnessione e degli errori della camera
Il sistema SHALL riconoscere disconnessione, timeout e stato "occupato" della camera, interrompere l'operazione in corso senza lasciare file parziali e riportare un messaggio comprensibile con l'azione suggerita.

#### Scenario: Cavo scollegato durante uno sviluppo
- **WHEN** la camera viene scollegata mentre uno sviluppo è in corso
- **THEN** l'attività si mette in pausa con stato "fotocamera scollegata", nessun file parziale resta nelle cartelle di output e l'attività può essere ripresa dopo il ricollegamento

#### Scenario: Camera bloccata
- **WHEN** la camera smette di rispondere oltre il timeout configurato
- **THEN** il sistema interrompe l'attività e suggerisce di spegnere e riaccendere la camera

### Requirement: Diagnostica della fotocamera
Il sistema SHALL offrire una verifica della fotocamera con l'esito di ogni controllo (driver, modalità USB, sessione, lettura dei banchi, conversione di prova) e un suggerimento per ogni controllo non superato. Lo sviluppo SHALL poter partire solo quando nessun controllo è fallito.

#### Scenario: Modalità USB sbagliata
- **WHEN** l'utente avvia la verifica con la camera in modalità lettore di schede
- **THEN** il controllo "Modalità USB" risulta non superato con l'indicazione del menu da cambiare, e "Avvia sviluppo" è disattivato
