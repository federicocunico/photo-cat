# Spec Delta

## Purpose

Offre all'utente un pannello di controllo dell'AI in esecuzione sulla sua libreria: mostra lo stato reale delle analisi, permette rianalisi mirate, rende esplorabili i risultati con grafici interattivi e consente di ripulirli in blocco.

## ADDED Requirements

### Requirement: Modulo AI
L'applicazione SHALL offrire un modulo "AI" con le sezioni Panoramica, Rianalisi, Risultati, Pulizia e Simili. Il modulo SHALL ricordare l'ultima sezione aperta e SHALL poter essere aperto su una sezione specifica da altri moduli e dalla palette dei comandi.

#### Scenario: Apertura del modulo
- **WHEN** l'utente apre il modulo AI per la prima volta
- **THEN** viene mostrata la sezione Panoramica

#### Scenario: Apertura da palette
- **WHEN** l'utente cerca "Rianalisi" nella palette dei comandi e conferma
- **THEN** si apre il modulo AI sulla sezione Rianalisi

### Requirement: Copertura delle analisi per fase
La Panoramica SHALL mostrare, per ciascuna fase (classificazione, persone, volti, gruppi di simili, impronte), il numero di foto analizzate con la versione corrente, da aggiornare perché analizzate con una versione precedente, in coda per un job attivo e mai analizzate. La somma SHALL corrispondere alle foto analizzabili della libreria, escluse quelle dei cataloghi di sistema.

#### Scenario: Libreria parzialmente analizzata
- **WHEN** la libreria ha 10.000 foto e la classificazione è stata eseguita su 7.000, di cui 500 con un modello precedente, senza job attivi
- **THEN** la Panoramica mostra per la classificazione 6.500 analizzate, 500 da aggiornare, 0 in coda e 3.000 mai analizzate

#### Scenario: Foto analizzata senza etichette
- **WHEN** una foto è stata classificata ma nessuna voce ha superato la soglia
- **THEN** la foto è contata tra le analizzate

### Requirement: Aggiornamento dal vivo
Durante un job AI la Panoramica SHALL aggiornare conteggi, avanzamento dei job attivi, modelli caricati e memoria in uso senza azioni dell'utente, almeno ogni 2 secondi e senza rallentare l'analisi in corso.

#### Scenario: Analisi in corso
- **WHEN** è in corso l'analisi di 2.000 foto e l'utente tiene aperta la Panoramica
- **THEN** il numero delle foto analizzate cresce durante il job e il job compare con la sua fase e la percentuale

### Requirement: Dalla Panoramica alla Libreria
Ogni conteggio della Panoramica SHALL essere attivabile. I conteggi "analizzate", "da aggiornare" e "mai analizzate" SHALL aprire la Libreria filtrata sulle foto corrispondenti. Il conteggio "in coda" dipende dai job attivi e non da uno stato delle foto, quindi SHALL aprire Attività sul job AI attivo.

#### Scenario: Foto mai analizzate
- **WHEN** l'utente fa clic sul conteggio "mai analizzate" della fase volti
- **THEN** si apre la Libreria con il filtro "Volti: mai analizzate" e il conteggio dei risultati è uguale a quello mostrato

#### Scenario: Foto in coda
- **WHEN** è in corso un'analisi e l'utente fa clic sul conteggio "in coda" della classificazione
- **THEN** si apre Attività sul job di analisi in corso

### Requirement: Rianalisi per ambito
La sezione Rianalisi SHALL permettere di scegliere l'ambito (tutta la libreria, uno o più cataloghi, la selezione corrente della Libreria, i risultati filtrati correnti della Libreria), le fasi da includere e la modalità: "solo mancanti o non aggiornate" oppure "forza". Prima dell'avvio SHALL mostrare quante foto verranno elaborate per fase.

#### Scenario: Due cataloghi, solo mancanti
- **WHEN** l'utente sceglie i cataloghi "Vacanze" e "Matrimonio", la fase classificazione e la modalità "solo mancanti o non aggiornate"
- **THEN** l'anteprima indica solo le foto dei due cataloghi non ancora classificate con la versione corrente, e l'avvio crea un'attività che elabora solo quelle

#### Scenario: Forza su una selezione
- **WHEN** l'utente ha selezionato 40 foto in Libreria, apre Rianalisi con ambito "selezione corrente", fasi persone e volti, modalità "forza"
- **THEN** le 40 foto vengono rianalizzate per persone e volti anche se già aggiornate, e le altre fasi non vengono toccate

#### Scenario: Ambito vuoto
- **WHEN** l'ambito scelto non contiene foto da elaborare
- **THEN** il pulsante di avvio è disattivato e l'anteprima indica 0 foto

### Requirement: Correzioni preservate dalla rianalisi
Ogni rianalisi, anche in modalità "forza", MUST preservare le correzioni manuali: etichette manuali, etichette rifiutate, volti assegnati a mano, correzioni e foto principali dei gruppi di simili.

#### Scenario: Rianalisi forzata dopo correzione
- **WHEN** l'utente ha impostato a mano il soggetto "Opera d'arte" su una foto e poi rianalizza il suo catalogo in modalità "forza"
- **THEN** il soggetto della foto resta "Opera d'arte"

### Requirement: Grafici dei risultati
La sezione Risultati SHALL mostrare sull'ambito scelto (libreria o catalogo):
- la distribuzione delle voci di un gruppo della tassonomia;
- l'istogramma delle confidenze di una voce, con la soglia corrente;
- la copertura dell'analisi per mese di scatto.

I grafici SHALL avere etichette leggibili, valori al passaggio del puntatore e uno stato vuoto esplicito.

#### Scenario: Distribuzione dei soggetti
- **WHEN** l'utente sceglie il gruppo "Soggetto" sull'intera libreria
- **THEN** il grafico mostra una barra per voce, con il numero di foto, ordinate dalla più frequente

#### Scenario: Ambito senza risultati
- **WHEN** l'ambito scelto non ha foto analizzate
- **THEN** i grafici mostrano un messaggio che invita ad avviare la rianalisi, con un collegamento alla sezione Rianalisi

### Requirement: Interazione con i grafici
Fare clic su una barra della distribuzione SHALL aprire la Libreria filtrata su quella voce e sull'ambito. Nell'istogramma delle confidenze l'utente SHALL poter selezionare un intervallo di confidenza e aprire quelle foto in Libreria o passarle alla Pulizia.

#### Scenario: Intervallo di confidenza
- **WHEN** l'utente seleziona l'intervallo 0,25–0,35 nell'istogramma della voce "Natura" e sceglie "Mostra in Libreria"
- **THEN** la Libreria mostra le foto con l'etichetta automatica "Natura" e confidenza in quell'intervallo

### Requirement: Pulizia delle etichette automatiche
La sezione Pulizia SHALL permettere di togliere un'etichetta automatica da tutte le foto di un ambito, eventualmente solo sotto una confidenza indicata. Prima di confermare SHALL mostrare il numero di foto interessate. L'operazione SHALL rendere le etichette rifiutate, SHALL essere annullabile e MUST NOT toccare le etichette manuali.

#### Scenario: Rimozione sotto soglia
- **WHEN** l'utente toglie "Natura" dalle foto del catalogo "Vacanze" con confidenza sotto 0,35 e conferma
- **THEN** quelle foto non hanno più "Natura", le altre foto con "Natura" restano invariate e compare un avviso con "Annulla operazione"

#### Scenario: Annullamento
- **WHEN** l'utente sceglie "Annulla operazione" subito dopo la rimozione
- **THEN** le etichette tornano come prima e non sono più rifiutate

### Requirement: Stato dei gruppi di simili
La sezione Simili SHALL mostrare il numero di gruppi, le foto raggruppate, le foto in attesa di analisi dei gruppi e la distribuzione delle dimensioni dei gruppi. SHALL permettere di avviare l'analisi dei gruppi in attesa e di aprire in Libreria le foto con simili.

#### Scenario: Foto in attesa
- **WHEN** ci sono 300 foto in attesa di analisi dei gruppi e l'utente sceglie "Avvia"
- **THEN** viene accodata l'analisi dei gruppi e il numero di foto in attesa diminuisce durante l'esecuzione

#### Scenario: Gruppi grandi
- **WHEN** l'utente fa clic sulla barra "più di 10 foto" della distribuzione delle dimensioni
- **THEN** si apre la Libreria con la vista raggruppata e solo le foto dei gruppi con più di 10 foto

### Requirement: Prestazioni del modulo AI
Con una libreria di 100.000 foto la Panoramica SHALL comparire entro 1 secondo, e ciascun grafico dei Risultati e ciascuna anteprima della Rianalisi o della Pulizia SHALL comparire entro 1 secondo dalla scelta dei parametri.

#### Scenario: Libreria grande
- **WHEN** l'utente apre la Panoramica con 100.000 foto in libreria
- **THEN** i conteggi di tutte le fasi compaiono entro 1 secondo
