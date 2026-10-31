# Spec Delta

## Purpose

Definisce la struttura dell'applicazione desktop: layout a pannelli in stile professionale ma semplice, navigazione tra moduli, indicatori globali di avanzamento e risorse, impostazioni e procedura di primo avvio.

## ADDED Requirements

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
Selezionando un file, il pannello di ispezione SHALL mostrare anteprima, percorso, nome, caratteristiche tecniche, date, luogo, catalogazioni AI con confidenza, persone, tag e cataloghi, permettendo la modifica dei campi correggibili.

#### Scenario: Ispezione file
- **WHEN** l'utente seleziona una foto
- **THEN** il pannello mostra tutti i dati del file e consente di correggere il tipo soggetto

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
L'utente SHALL poter configurare: profilo risorse e limite RAM, provider di esecuzione, soglie di confidenza, tassonomia, attivazione del riconoscimento volti, livello di log, tema, lingua e percorso dei dati dell'app.

#### Scenario: Modifica profilo
- **WHEN** l'utente cambia il profilo risorse da "Bilanciato" a "Leggero"
- **THEN** i job successivi usano il nuovo budget di memoria

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
