# desktop-shell Specification

## Purpose
Definisce la struttura dell'applicazione desktop: layout a pannelli in stile professionale ma semplice, navigazione tra moduli, indicatori globali di avanzamento e risorse, impostazioni e procedura di primo avvio.

## Requirements

### Requirement: Layout a pannelli
L'interfaccia SHALL presentare una barra superiore, una navigazione dei moduli, un pannello laterale sinistro (sorgenti, cataloghi, filtri), un'area centrale (griglia o lista), un pannello destro di ispezione e una barra di stato; i pannelli laterali SHALL essere ridimensionabili e richiudibili e lo stato del layout SHALL essere ricordato.

#### Scenario: Pannello richiuso
- **WHEN** l'utente chiude il pannello di ispezione e riavvia l'app
- **THEN** il pannello risulta ancora chiuso

### Requirement: Moduli di navigazione
L'applicazione SHALL offrire i moduli Libreria, Cataloghi, Persone, Importa, Job, Esporta e organizza, Modelli AI, Diagnostica e Impostazioni, e la navigazione SHALL essere estendibile con nuovi moduli.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria a Job e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

### Requirement: Griglia di miniature performante
La griglia SHALL visualizzare librerie di almeno 100.000 elementi con scorrimento fluido, caricando le miniature in modo progressivo e con dimensione delle miniature regolabile.

#### Scenario: Scorrimento rapido
- **WHEN** l'utente scorre rapidamente una libreria da 100.000 foto
- **THEN** l'interfaccia resta reattiva e mostra segnaposto finché le miniature non sono caricate

### Requirement: Pannello di ispezione
Selezionando un file, il pannello di ispezione SHALL mostrare anteprima, percorso, nome, caratteristiche tecniche, date, luogo, catalogazioni AI con confidenza, persone, tag e cataloghi, permettendo la modifica dei campi correggibili, incluso il luogo.

#### Scenario: Ispezione file
- **WHEN** l'utente seleziona una foto
- **THEN** il pannello mostra tutti i dati del file e consente di correggere il tipo soggetto

#### Scenario: Correzione del luogo
- **WHEN** l'utente modifica il luogo di una foto da "D7000" a "Giappone" nel pannello di ispezione
- **THEN** il luogo diventa "Giappone" con fonte "manuale" e il facet "Luogo" della libreria si aggiorna

#### Scenario: Suggerimenti del luogo
- **WHEN** l'utente inizia a digitare il luogo
- **THEN** il pannello suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline

### Requirement: Indicatori globali
La barra superiore SHALL mostrare sempre un indicatore dei job attivi con percentuale complessiva e uno di consumo RAM, e la barra di stato SHALL mostrare numero di elementi, selezionati, stato dei modelli e numero di errori recenti, con accesso diretto alle rispettive schermate.

#### Scenario: Job in corso da altro modulo
- **WHEN** è in corso un'analisi e l'utente è in Libreria
- **THEN** la barra superiore mostra la percentuale dell'analisi e un clic porta alla schermata Job

### Requirement: Stile visivo
L'interfaccia SHALL usare un tema scuro predefinito con contrasto testo accessibile (almeno 4,5:1), un solo colore d'accento e una gerarchia tipografica chiara, e SHALL offrire anche un tema chiaro selezionabile.

#### Scenario: Cambio tema
- **WHEN** l'utente seleziona il tema chiaro
- **THEN** l'intera interfaccia passa al tema chiaro senza riavvio

### Requirement: Primo avvio guidato
Al primo avvio l'app SHALL guidare l'utente attraverso: benvenuto, verifica del sistema (RAM, CPU, acceleratori, profilo consigliato), download dei modelli con progresso e aggiunta della prima sorgente; ogni passo tranne la verifica del sistema SHALL poter essere rimandato.

#### Scenario: Download rimandato
- **WHEN** l'utente salta il download dei modelli al primo avvio
- **THEN** l'app si apre con le funzioni AI disattivate e un avviso che permette di avviare il download

### Requirement: Impostazioni
L'utente SHALL poter configurare: profilo risorse e limite RAM, provider di esecuzione, soglie dell'automatismo AI (CLIP per gruppo e categoria, rilevamento persone, rilevamento volti, somiglianza dei volti), tassonomia, attivazione del riconoscimento volti, nomi di cartella da ignorare per il luogo, applicazioni esterne e azione del doppio clic, livello di log, tema, lingua e percorso dei dati dell'app.

#### Scenario: Modifica profilo
- **WHEN** l'utente cambia il profilo risorse da "Bilanciato" a "Leggero"
- **THEN** i job successivi usano il nuovo budget di memoria

#### Scenario: Soglie di gruppo preservate
- **WHEN** l'utente ha impostato soglie diverse per i gruppi della tassonomia e modifica un'altra impostazione
- **THEN** le soglie dei gruppi restano quelle impostate dall'utente

### Requirement: Localizzazione
Tutti i testi dell'interfaccia SHALL provenire da risorse traducibili; la prima versione SHALL essere disponibile in italiano e in inglese.

#### Scenario: Cambio lingua
- **WHEN** l'utente imposta la lingua inglese
- **THEN** l'interfaccia è mostrata in inglese

### Requirement: Pubblicazione dell'installer Windows
La pipeline di distribuzione SHALL compilare l'eseguibile/setup Windows e, su un tag di versione corrispondente alla versione dell'app, SHALL creare automaticamente una GitHub Release con installer EXE e checksum SHA-256. I test completi SHALL essere eseguiti prevalentemente in locale; i normali push SHALL usare una build Windows con cache senza pubblicare release.

#### Scenario: Tag di versione
- **WHEN** viene pubblicato un tag `v<versione>` valido
- **THEN** la pipeline crea una release della stessa versione con installer EXE scaricabile e relativo checksum, senza fotografie né dati personali dell'utente

### Requirement: Esecuzione su Linux
L'applicazione SHALL avviarsi e funzionare su Linux x86_64 con un desktop grafico (Wayland o X11), sia in sviluppo sia dalla build distribuita, con le stesse funzioni di Windows salvo quelle dichiarate specifiche di una piattaforma.

#### Scenario: Avvio in sviluppo
- **WHEN** su Linux, con le dipendenze di sistema documentate installate, lo sviluppatore esegue l'avvio locale in debug
- **THEN** la finestra principale si apre e diventa interattiva senza variabili d'ambiente aggiuntive

#### Scenario: Avvio dall'archivio
- **WHEN** l'utente estrae l'archivio Linux di una release e avvia l'eseguibile
- **THEN** l'app si apre, trova le proprie risorse (modelli, runtime, dati di riferimento) e usa la cartella dati utente standard di Linux

#### Scenario: Installazione del pacchetto deb
- **WHEN** l'utente installa il pacchetto deb di una release su Ubuntu o Debian
- **THEN** le dipendenze di sistema necessarie vengono installate dal gestore pacchetti, photo-cat compare nel menu delle applicazioni e si avvia trovando le proprie risorse

### Requirement: Verifica Linux nella pipeline
Ogni push e pull request SHALL compilare e verificare l'app anche su Linux, con controllo di formattazione, analisi statica, test Rust e build di release, oltre alla build Windows già prevista.

#### Scenario: Regressione solo su Linux
- **WHEN** una modifica compila su Windows ma non su Linux
- **THEN** la pipeline fallisce sul job Linux

### Requirement: Pacchetti Linux nelle release
Su un tag di versione valido, la release SHALL includere, oltre all'installer Windows, un pacchetto deb per Linux x64 e un archivio Linux x64 con l'eseguibile e le risorse necessarie, ciascuno con il relativo checksum SHA-256, senza fotografie né dati personali.

#### Scenario: Tag di versione
- **WHEN** viene pubblicato un tag `v<versione>` valido
- **THEN** la release contiene `photo-cat_<versione>_amd64.deb` e `photo-cat-<versione>-linux-x64.tar.gz`, ciascuno con un file `.sha256` verificabile con `sha256sum -c`

### Requirement: Menu contestuale della griglia
Il clic destro su una miniatura SHALL aprire un menu contestuale che agisce sulla selezione corrente (o sulla sola miniatura se non selezionata) con almeno: "Apri con ▸" (applicazioni esterne configurate e app predefinita del sistema), "Mostra nella cartella", "Imposta luogo…" e le azioni bulk più comuni; il menu SHALL essere accessibile da tastiera.

#### Scenario: Clic destro su miniatura non selezionata
- **WHEN** sono selezionate 3 foto e l'utente fa clic destro su una quarta foto non selezionata
- **THEN** la selezione diventa la sola quarta foto e il menu agisce su di essa

#### Scenario: Clic destro sulla selezione
- **WHEN** sono selezionate 20 foto e l'utente fa clic destro su una di esse
- **THEN** il menu indica che agisce su 20 foto

#### Scenario: Apertura da tastiera
- **WHEN** una miniatura ha il focus e l'utente preme il tasto Menu o Maiusc+F10
- **THEN** si apre il menu contestuale navigabile con le frecce

### Requirement: Pannelli flottanti
L'app SHALL poter mostrare contenuti in pannelli flottanti non modali sopra l'interfaccia: trascinabili dalla barra del titolo, ridimensionabili, chiudibili con un pulsante o con Esc quando hanno il focus, sempre interamente raggiungibili dentro la finestra. Mentre un pannello è aperto l'utente SHALL poter continuare a usare il resto dell'interfaccia. Posizione e dimensione SHALL essere ricordate tra un'apertura e l'altra e tra i riavvii.

#### Scenario: Elenco luoghi in un pannello flottante
- **WHEN** l'utente sceglie "Apri in finestra" sulla lista Luoghi
- **THEN** si apre un pannello flottante con l'elenco completo dei luoghi, e l'utente può selezionare foto nella griglia senza chiuderlo

#### Scenario: Aggiornamento col filtro
- **WHEN** il pannello dei luoghi è aperto e l'utente attiva il filtro "Anno 2023" dalla barra laterale
- **THEN** il pannello mostra i luoghi e i conteggi delle sole foto del 2023

#### Scenario: Più pannelli
- **WHEN** l'utente apre il pannello dei luoghi e quello dei tag
- **THEN** i due pannelli sono visibili insieme e quello cliccato per ultimo è in primo piano

#### Scenario: Posizione ricordata
- **WHEN** l'utente sposta e ridimensiona il pannello dei luoghi, lo chiude e lo riapre dopo un riavvio
- **THEN** il pannello ricompare con la stessa posizione e dimensione

#### Scenario: Finestra rimpicciolita
- **WHEN** la finestra dell'app viene ridotta e un pannello aperto resterebbe in parte fuori
- **THEN** il pannello viene riposizionato e, se serve, ridotto per restare interamente visibile

#### Scenario: Uso da tastiera
- **WHEN** il pannello ha il focus e l'utente preme Esc
- **THEN** il pannello si chiude e il focus torna al comando che lo aveva aperto
