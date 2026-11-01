# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
