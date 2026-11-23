# Spec Delta

## Purpose

Dà a photo-cat una barra dei menu da app desktop (File, Modifica, Vista, Vai, Strumenti, Aiuto) che rende visibili tutti i comandi globali e le loro scorciatoie, con gli stessi nomi della palette dei comandi.

## ADDED Requirements

### Requirement: Barra dei menu
La barra superiore SHALL contenere, dopo il nome dell'app, una barra con i menu File, Modifica, Vista, Vai, Strumenti e Aiuto. La barra SHALL essere visibile in tutti i moduli e SHALL sostituire il menu a icona "?" della barra superiore.

#### Scenario: Menu sempre presenti
- **WHEN** l'utente apre un qualsiasi modulo
- **THEN** la barra superiore mostra i menu File, Modifica, Vista, Vai, Strumenti e Aiuto, e non mostra più il pulsante "?"

### Requirement: Voci dei menu dal registro dei comandi
Ogni voce di menu SHALL essere un comando del registro dei comandi, con lo stesso nome e la stessa scorciatoia mostrati dalla palette e dal foglio delle scorciatoie. La scorciatoia SHALL comparire accanto alla voce. Una voce che non si può usare SHALL restare visibile, disattivata, con il motivo al passaggio del mouse o al focus.

#### Scenario: Stessa scorciatoia ovunque
- **WHEN** l'utente apre il menu Vista
- **THEN** la voce "Inspector" mostra "Ctrl+I", lo stesso tasto indicato dalla palette per lo stesso comando

#### Scenario: Comando non disponibile
- **WHEN** l'utente apre il menu Modifica senza foto selezionate
- **THEN** "Sposta nel Cestino" è disattivata e spiega che bisogna prima selezionare delle foto

### Requirement: Menu File e Modifica
Il menu File SHALL contenere almeno: Importa una cartella…, Esporta…, Apri una cartella di RAW…, Apri un progetto FujiView…, Mostra la cartella dei dati, Impostazioni, Esci. Il menu Modifica SHALL contenere almeno: Annulla operazione, Seleziona tutto, Inverti selezione, Deseleziona, tag del set attivo, valutazione, Aggiungi a un catalogo…, Sposta nel Cestino.

#### Scenario: Importare dal menu
- **WHEN** l'utente sceglie File › "Importa una cartella…"
- **THEN** si apre il selettore di cartelle di Importa, come con il pulsante della pagina Importa

#### Scenario: Selezione dal menu
- **WHEN** l'utente ha il focus nella griglia dei volti di Persone e sceglie Modifica › "Seleziona tutto"
- **THEN** si selezionano tutti i volti del gruppo aperto, come con Ctrl+A

### Requirement: Menu Vista, Vai e Strumenti
Il menu Vista SHALL contenere almeno: navigazione compatta, pannelli laterali, Inspector, dimensione delle miniature, Raggruppa foto simili, Mappa delle foto, tema, schermo intero. Il menu Vai SHALL elencare ogni modulo, Attività e la palette dei comandi. Il menu Strumenti SHALL contenere almeno: Rianalizza le foto…, Metti da parte i duplicati, Controlla la qualità, Modelli AI, Diagnostica e registri. Il menu Aiuto è descritto in "Menu Aiuto".

#### Scenario: Aprire un modulo dal menu
- **WHEN** l'utente sceglie Vai › Persone
- **THEN** si apre il modulo Persone, e la voce mostra la stessa scorciatoia Ctrl+N della navigazione

### Requirement: Barra dei menu da tastiera
Alt o F10 SHALL portare il focus sul primo menu. Le frecce SHALL spostarsi tra i menu e tra le voci, Invio SHALL eseguire la voce ed Esc SHALL chiudere il menu riportando il focus dov'era. Ogni menu SHALL avere una lettera di accesso sottolineata quando il focus è sulla barra.

#### Scenario: Menu da tastiera
- **WHEN** l'utente preme Alt, poi la freccia destra due volte e Invio
- **THEN** si apre il menu Vista con il focus sulla prima voce

#### Scenario: Esc
- **WHEN** il menu Vista è aperto e l'utente preme Esc due volte
- **THEN** il menu si chiude, la barra perde il focus e il focus torna all'elemento che lo aveva prima

### Requirement: Menu su macOS
Su macOS gli stessi menu SHALL comparire nella barra dei menu del sistema, preceduti dal menu dell'app con "Informazioni su photo-cat", "Impostazioni…" (⌘,) ed "Esci da photo-cat" (⌘Q). Su macOS la barra superiore della finestra MUST NOT duplicare i menu.

#### Scenario: Informazioni su macOS
- **WHEN** su macOS l'utente sceglie photo-cat › "Informazioni su photo-cat"
- **THEN** si apre la stessa finestra Informazioni delle altre piattaforme
