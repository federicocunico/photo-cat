# Spec Delta

## MODIFIED Requirements

### Requirement: Layout a pannelli
L'interfaccia SHALL presentare una barra superiore, una navigazione dei moduli, un pannello laterale sinistro (in Libreria: cataloghi, filtri e, in fondo, cartelle), un'area centrale (griglia o lista), un pannello destro di ispezione e una barra di stato. I pannelli laterali SHALL essere ridimensionabili e richiudibili, anche da tastiera (Ctrl+Maiusc+B nasconde e mostra entrambi i pannelli laterali, Ctrl+B la navigazione dei moduli, Ctrl+I l'ispettore; Tab resta riservato allo spostamento del focus). La navigazione dei moduli SHALL potersi ridurre a sole icone con descrizione al passaggio del mouse, e SHALL ridursi automaticamente a icone sotto i 1.008 px di larghezza della finestra. Le sezioni del pannello sinistro SHALL potersi richiudere singolarmente. Lo stato del layout SHALL essere ricordato.

#### Scenario: Pannello richiuso
- **WHEN** l'utente chiude il pannello di ispezione e riavvia l'app
- **THEN** il pannello risulta ancora chiuso

#### Scenario: Navigazione a icone
- **WHEN** l'utente preme Ctrl+B
- **THEN** la navigazione dei moduli mostra solo le icone, ognuna con nome e scorciatoia al passaggio del mouse, e la griglia guadagna lo spazio liberato

#### Scenario: Finestra stretta
- **WHEN** la finestra viene ridotta a 900 px di larghezza
- **THEN** la navigazione passa a sole icone senza cambiare la preferenza salvata per le finestre larghe

#### Scenario: Concentrazione sulla griglia
- **WHEN** l'utente preme Ctrl+Maiusc+B in Libreria senza un campo di testo attivo
- **THEN** pannello sinistro e ispettore si nascondono, e un nuovo Ctrl+Maiusc+B li ripristina com'erano

#### Scenario: Ordine del pannello sinistro in Libreria
- **WHEN** l'utente apre la Libreria
- **THEN** il pannello sinistro mostra dall'alto la sezione dei cataloghi, poi i filtri e infine le cartelle

### Requirement: Menu contestuale della griglia
Il clic destro su una miniatura SHALL aprire un menu contestuale che agisce sulla selezione corrente (o sulla sola miniatura se non selezionata) con almeno: "Apri con ▸" (applicazioni esterne configurate e app predefinita del sistema), "Mostra nella cartella", "Imposta luogo…", "Aggiungi a catalogo ▸" e le azioni bulk più comuni; il menu SHALL essere accessibile da tastiera. "Aggiungi a catalogo ▸" SHALL mostrare un elenco scorrevole con ricerca dei cataloghi manuali dell'utente e "Nuovo catalogo…".

#### Scenario: Clic destro su miniatura non selezionata
- **WHEN** sono selezionate 3 foto e l'utente fa clic destro su una quarta foto non selezionata
- **THEN** la selezione diventa la sola quarta foto e il menu agisce su di essa

#### Scenario: Clic destro sulla selezione
- **WHEN** sono selezionate 20 foto e l'utente fa clic destro su una di esse
- **THEN** il menu indica che agisce su 20 foto

#### Scenario: Apertura da tastiera
- **WHEN** una miniatura ha il focus e l'utente preme il tasto Menu o Maiusc+F10
- **THEN** si apre il menu contestuale navigabile con le frecce

#### Scenario: Aggiunta a un catalogo dal menu
- **WHEN** sono selezionate 20 foto, l'utente apre il menu contestuale, sceglie "Aggiungi a catalogo ▸", scrive "ma" e sceglie "Mare"
- **THEN** le 20 foto vengono aggiunte a "Mare" con un avviso con "Annulla operazione"

#### Scenario: Molti cataloghi
- **WHEN** l'utente ha 60 cataloghi manuali e apre "Aggiungi a catalogo ▸"
- **THEN** l'elenco mostra tutti i cataloghi manuali in ordine alfabetico, scorre entro l'altezza del menu e i cataloghi smart e di sistema non compaiono

#### Scenario: Nuovo catalogo dal menu
- **WHEN** l'utente sceglie "Aggiungi a catalogo ▸" › "Nuovo catalogo…" e scrive "Compleanno"
- **THEN** viene creato il catalogo manuale "Compleanno" con le foto su cui agisce il menu
