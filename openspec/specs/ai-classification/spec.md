# ai-classification Specification

## Purpose
Assegna automaticamente e in locale le catalogazioni semantiche alle foto (tipo soggetto, interno/esterno e ambiente, presenza e numero di persone, luogo) tramite modelli neurali eseguiti sul computer dell'utente.

## Requirements

### Requirement: Esecuzione esclusivamente locale
Tutta l'analisi AI MUST essere eseguita sul computer dell'utente; nessuna immagine, miniatura, embedding o risultato SHALL essere inviato in rete.

#### Scenario: Analisi offline
- **WHEN** i modelli sono già scaricati e il computer è offline
- **THEN** l'analisi AI viene completata normalmente

### Requirement: Tassonomia configurabile
Il sistema SHALL fornire una tassonomia predefinita di catalogazioni (tipo soggetto, locazione interno/esterno con sottocategorie, ambiente) e l'utente SHALL poter aggiungere, rinominare o disattivare voci con etichette testuali senza riaddestrare modelli. L'utente SHALL poter creare anche gruppi nuovi e voci prive di descrizione per il riconoscimento automatico ("voci manuali"), che l'analisi non assegna mai ma che l'utente può assegnare, filtrare e usare nei cataloghi smart.

#### Scenario: Nuova categoria
- **WHEN** l'utente aggiunge la voce "Barche" al tipo soggetto e rianalizza una selezione
- **THEN** le foto con barche possono ricevere la catalogazione "Barche"

#### Scenario: Tassonomia predefinita
- **WHEN** l'utente non ha modificato la tassonomia
- **THEN** sono disponibili almeno: tipo soggetto (ritratto, gruppo, natura, paesaggio, ambiente urbano, monumento, opera d'arte, animale, fiori, alberi, cibo, oggetto), locazione (interno: stanza, teatro, palazzetto, chiesa, museo; esterno urbano: via, piazza; esterno natura: mare, montagna, lago, fiume, bosco, campagna)

#### Scenario: Voce manuale
- **WHEN** l'utente aggiunge al tipo soggetto la voce "Matrimonio Anna" senza descrizione e rianalizza la libreria
- **THEN** nessuna foto riceve "Matrimonio Anna" dall'analisi, e le foto a cui l'utente l'ha assegnata la mantengono

#### Scenario: Gruppo nuovo solo manuale
- **WHEN** l'utente crea il gruppo "Evento" con le voci "Compleanno" e "Laurea", senza descrizioni
- **THEN** il gruppo compare nell'Inspector, nella barra bulk, nei filtri della libreria e nell'editor dei cataloghi smart, e l'analisi automatica lo ignora

#### Scenario: Voce resa automatica
- **WHEN** l'utente aggiunge alla voce manuale "Barche" la descrizione "a photo of boats"
- **THEN** la voce diventa automatica e il sistema propone di rianalizzare i file per assegnarla

#### Scenario: Eliminazione di una voce usata
- **WHEN** l'utente elimina la voce "Laurea" assegnata a 25 foto
- **THEN** il sistema chiede conferma indicando 25 foto e, dopo la conferma, le 25 foto non hanno più quella catalogazione

### Requirement: Classificazione zero-shot con confidenza
Per ogni foto analizzata il sistema SHALL assegnare per ciascun gruppo della tassonomia le voci più probabili con un punteggio di confidenza da 0 a 1, conservando solo quelle sopra la soglia applicabile: la soglia della singola categoria se impostata, altrimenti quella del suo gruppo.

#### Scenario: Paesaggio montano
- **WHEN** viene analizzata la foto di un paesaggio montano
- **THEN** la foto riceve "Esterno natura" e "Montagna" con le relative confidenze e le voci sotto soglia non sono assegnate

#### Scenario: Soglia per categoria
- **WHEN** il gruppo "Ambiente" ha soglia 0,25, la categoria "Stanza" ha soglia 0,60 e una foto ottiene 0,40 per "Stanza"
- **THEN** la foto non riceve "Stanza", mentre riceve le altre voci di "Ambiente" con punteggio almeno 0,25

#### Scenario: Categoria senza soglia propria
- **WHEN** una categoria non ha una soglia impostata
- **THEN** per quella categoria si usa la soglia del gruppo

### Requirement: Rilevamento e conteggio persone
Il sistema SHALL rilevare se nella foto sono presenti persone e quante, memorizzando il conteggio e le aree rilevate; sono contate solo le rilevazioni con confidenza pari o superiore alla soglia di rilevamento persone configurata.

#### Scenario: Foto di gruppo
- **WHEN** viene analizzata una foto con 4 persone
- **THEN** la foto risulta "Persone: presenti" con conteggio 4

#### Scenario: Nessuna persona
- **WHEN** viene analizzata una foto senza persone
- **THEN** la foto risulta "Persone: assenti" con conteggio 0

#### Scenario: Soglia alzata
- **WHEN** una foto ha rilevazioni di persona con confidenza 0,9, 0,7 e 0,45 e l'utente porta la soglia a 0,6
- **THEN** la foto risulta con conteggio 2

### Requirement: Geolocalizzazione
Il sistema SHALL determinare il luogo dalle coordinate GPS EXIF quando presenti (con nome località da un dataset geografico offline) e altrimenti dal nome delle directory del percorso, indicando sempre la fonte del luogo. Nella deduzione da directory il sistema SHALL ignorare i nomi generici, i nomi che corrispondono a marca o modello della fotocamera della foto, i nomi di dispositivo comuni e i nomi di cartella indicati dall'utente, usando la directory superiore.

#### Scenario: GPS presente
- **WHEN** una foto contiene coordinate GPS di Firenze
- **THEN** il luogo è "Firenze" con fonte "GPS"

#### Scenario: Luogo da directory
- **WHEN** una foto senza GPS si trova in `D:\Foto\2023\Toscana\Siena`
- **THEN** il luogo proposto è "Siena" con fonte "directory"

#### Scenario: Sottocartella del dispositivo
- **WHEN** una foto senza GPS scattata con una Nikon D7000 si trova in `D:\Foto\Viaggio Giappone\D7000`
- **THEN** il luogo proposto è "Viaggio Giappone" con fonte "directory"

#### Scenario: Sottocartella del telefono
- **WHEN** una foto senza GPS si trova in `D:\Foto\Viaggio Giappone\Telefono`
- **THEN** il luogo proposto è "Viaggio Giappone" con fonte "directory"

#### Scenario: Nome di cartella ignorato dall'utente
- **WHEN** l'utente ha aggiunto "Selezionate" ai nomi di cartella da ignorare e una foto senza GPS si trova in `D:\Foto\Sardegna\Selezionate`
- **THEN** il luogo proposto è "Sardegna" con fonte "directory"

### Requirement: Correzione manuale prevale
L'utente SHALL poter correggere qualsiasi catalogazione automatica; le correzioni manuali MUST prevalere sulle analisi successive.

#### Scenario: Rianalisi dopo correzione
- **WHEN** l'utente cambia il tipo soggetto da "Natura" a "Opera d'arte" e poi la foto viene rianalizzata
- **THEN** il tipo soggetto resta "Opera d'arte"

### Requirement: Versionamento delle analisi
Ogni risultato AI SHALL riportare la versione del modello e della tassonomia che l'ha prodotto, così che il sistema possa proporre la rianalisi dei soli file prodotti con versioni precedenti. Le modifiche alla tassonomia che non cambiano l'analisi automatica (voci o gruppi manuali, nomi visualizzati) MUST NOT rendere i file da rianalizzare.

#### Scenario: Modello aggiornato
- **WHEN** viene installata una nuova versione del modello di classificazione
- **THEN** il sistema indica quanti file sono stati analizzati con la versione precedente e permette di rianalizzarli

#### Scenario: Voce manuale aggiunta
- **WHEN** la libreria è interamente analizzata e l'utente aggiunge una voce manuale
- **THEN** il numero di file da rianalizzare resta zero

#### Scenario: Rinomina del nome visualizzato
- **WHEN** l'utente rinomina la voce "Fiori" in "Fiori e piante" senza cambiarne la descrizione
- **THEN** il numero di file da rianalizzare resta zero e le foto con quella voce mostrano il nuovo nome

#### Scenario: Prompt modificato
- **WHEN** l'utente modifica la descrizione di una voce automatica
- **THEN** il sistema indica i file analizzati con la versione precedente e permette di rianalizzarli

### Requirement: Analisi riprendibile
L'analisi AI SHALL essere idempotente per file: un'analisi interrotta SHALL poter riprendere elaborando solo i file non ancora completati.

#### Scenario: Chiusura durante l'analisi
- **WHEN** l'app viene chiusa con l'analisi al 60%
- **THEN** alla ripresa vengono elaborati solo i file rimanenti

### Requirement: Soglie dell'automatismo configurabili
Il sistema SHALL esporre nelle impostazioni, con valori predefiniti ripristinabili, le soglie che governano l'automatismo: soglia CLIP per gruppo e per categoria, soglia di rilevamento persone, soglia di rilevamento volti, dimensione minima dei volti e soglia di somiglianza per il raggruppamento dei volti. Valori fuori dall'intervallo 0–1 MUST essere rifiutati.

#### Scenario: Ripristino dei predefiniti
- **WHEN** l'utente ha modificato più soglie e sceglie "Ripristina predefiniti"
- **THEN** tutte le soglie tornano ai valori predefiniti dell'applicazione

#### Scenario: Valore non valido
- **WHEN** l'utente inserisce 1,5 come soglia di rilevamento persone
- **THEN** il valore viene rifiutato e la soglia precedente resta in vigore

### Requirement: Anteprima dell'effetto delle soglie
Prima di salvare nuove soglie il sistema SHALL mostrare quante foto della libreria cambierebbero catalogazioni o conteggio persone e quante persone cambierebbero raggruppamento, calcolandolo dai risultati già salvati senza eseguire i modelli.

#### Scenario: Anteprima di una soglia CLIP
- **WHEN** l'utente porta la soglia della categoria "Stanza" da 0,25 a 0,60
- **THEN** il sistema mostra il numero di foto che perderebbero "Stanza" prima della conferma

### Requirement: Applicazione delle soglie senza rianalisi
Al salvataggio di nuove soglie il sistema SHALL ricalcolare catalogazioni CLIP, conteggi delle persone e raggruppamenti dai risultati già salvati, come job con progresso e senza eseguire di nuovo i modelli, rispettando le correzioni manuali. Se una soglia scende sotto il minimo con cui i risultati sono stati salvati, il sistema SHALL indicarlo e proporre la rianalisi dei file interessati.

#### Scenario: Soglia alzata
- **WHEN** l'utente alza la soglia del gruppo "Tipo soggetto" e salva
- **THEN** viene avviato un job di rivalutazione che aggiorna le catalogazioni di tutta la libreria senza caricare il modello immagine

#### Scenario: Correzione manuale preservata
- **WHEN** una foto ha "Tipo soggetto = Opera d'arte" come correzione manuale e le soglie vengono cambiate
- **THEN** il tipo soggetto della foto resta "Opera d'arte"

#### Scenario: Soglia sotto il minimo salvato
- **WHEN** l'utente porta la soglia di rilevamento volti sotto il minimo con cui sono stati salvati i volti
- **THEN** il sistema avvisa che i nuovi volti emergeranno solo con una rianalisi e propone di avviarla

### Requirement: Creazione di voci durante la correzione
Nei selettori dei valori di catalogazione usati per correggere una foto o una selezione, l'utente SHALL poter creare una nuova voce del gruppo scrivendone solo il nome; la voce viene creata come voce manuale e assegnata subito come correzione manuale. Un nome già esistente nel gruppo, senza distinzione di maiuscole e accenti, SHALL riusare la voce esistente invece di crearne un doppione.

#### Scenario: Nuova voce dall'Inspector
- **WHEN** nell'Inspector l'utente sceglie il gruppo "Tipo soggetto", poi "Nuova voce…", scrive "Barche" e conferma
- **THEN** "Barche" viene aggiunta al tipo soggetto e la foto ha "Tipo soggetto = Barche" come correzione manuale

#### Scenario: Nuova voce in bulk
- **WHEN** l'utente seleziona 60 foto e, nell'azione "Imposta catalogazione", crea la voce "Gita scolastica"
- **THEN** la voce viene creata una sola volta e le 60 foto la ricevono come correzione manuale, con possibilità di annullare l'assegnazione

#### Scenario: Nome già esistente
- **WHEN** l'utente crea nel tipo soggetto la voce "fiori" e il gruppo contiene già "Fiori"
- **THEN** non viene creato un doppione e viene assegnata la voce "Fiori"

#### Scenario: Nome vuoto
- **WHEN** l'utente conferma "Nuova voce…" senza scrivere un nome
- **THEN** nessuna voce viene creata e il campo segnala che il nome è obbligatorio
