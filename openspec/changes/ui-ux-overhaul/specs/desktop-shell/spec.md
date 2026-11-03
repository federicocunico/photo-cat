# Spec Delta

> Le basi dei requisiti modificati sono i testi che risulteranno dopo l'archiviazione di `loading-feedback` ("Indicatori globali") e di `people-faces-workspace` ("Moduli di navigazione"). Se quei testi cambiano prima dell'integrazione, questa delta va riallineata (vedi `tasks.md`, gruppo 0).

## MODIFIED Requirements

### Requirement: Layout a pannelli
L'interfaccia SHALL presentare una barra superiore, una navigazione dei moduli, un pannello laterale sinistro (cartelle, cataloghi, filtri), un'area centrale (griglia o lista), un pannello destro di ispezione e una barra di stato. I pannelli laterali SHALL essere ridimensionabili e richiudibili, anche da tastiera (Ctrl+Maiusc+B nasconde e mostra entrambi i pannelli laterali, Ctrl+B la navigazione dei moduli, Ctrl+I l'ispettore; Tab resta riservato allo spostamento del focus). La navigazione dei moduli SHALL potersi ridurre a sole icone con descrizione al passaggio del mouse, e SHALL ridursi automaticamente a icone sotto i 1.008 px di larghezza della finestra. Le sezioni del pannello sinistro SHALL potersi richiudere singolarmente. Lo stato del layout SHALL essere ricordato.

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

### Requirement: Moduli di navigazione
L'applicazione SHALL organizzare la navigazione in gruppi con un'etichetta:
- **Sfoglia**: Libreria, Sequenza;
- **Organizza**: Cataloghi, Persone, Tag;
- **Importa ed esporta**: Importa, Esporta.

In fondo alla navigazione SHALL stare **Attività** e **Impostazioni**.

Modelli AI e Diagnostica SHALL essere sezioni di Impostazioni e non voci di primo livello. SHALL restare raggiungibili direttamente da Attività, dalla barra di stato, dagli avvisi e dalla palette dei comandi. Persone e identità SHALL essere gestite in un unico modulo Persone.

Ogni voce di primo livello SHALL avere una scorciatoia (Ctrl+1…Ctrl+7 nell'ordine dei gruppi). Un modulo SHALL poter essere aperto su un elemento specifico, ad esempio una persona, un volto, una sezione di Impostazioni o un'attività, da altri moduli. La navigazione SHALL essere estendibile con nuovi moduli.

Un modulo salvato come attivo nelle preferenze che non è più una voce di primo livello SHALL aprire la sezione corrispondente.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria ad Attività e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

#### Scenario: Gruppi della navigazione
- **WHEN** l'utente apre l'app
- **THEN** la navigazione mostra i gruppi Sfoglia, Organizza e Importa ed esporta con le loro voci, e in fondo Attività e Impostazioni

#### Scenario: Scorciatoia di modulo
- **WHEN** l'utente preme Ctrl+3
- **THEN** si apre il modulo Cataloghi

#### Scenario: Identità nel modulo Persone
- **WHEN** l'utente cerca nella navigazione il modulo Identità
- **THEN** non lo trova come voce separata, e le identità sono gestite nella sezione "Persone" del modulo Persone

#### Scenario: Preferenza del modulo non più esistente
- **WHEN** l'ultimo modulo aperto salvato nelle preferenze era "Identità", "Modelli AI" o "Diagnostica"
- **THEN** all'avvio si apre rispettivamente Persone sulla sezione "Persone", Impostazioni sulla sezione "Modelli AI e sistema" o Impostazioni sulla sezione "Diagnostica"

#### Scenario: Diagnostica da un errore
- **WHEN** l'utente fa clic sul conteggio degli errori nella barra di stato
- **THEN** si apre Impostazioni › Diagnostica filtrata sugli errori

### Requirement: Indicatori globali
La barra superiore SHALL mostrare sempre un indicatore delle attività in corso con la percentuale complessiva. La barra di stato SHALL mostrare:
- il numero di elementi e di selezionati;
- lo stato dei modelli;
- il consumo di RAM;
- il numero di errori recenti;

con accesso diretto alle rispettive schermate.

L'indicatore delle attività SHALL animarsi solo quando ci sono attività in corso, e SHALL indicare in parole che cosa sta facendo (ad esempio "Importazione · 42%"), mai un identificatore interno. La barra superiore SHALL mostrare un indicatore distinto quando richieste dell'interfaccia al motore sono in corso da più di 300 ms. Quando la memoria è sotto pressione, l'indicatore della RAM SHALL segnalarlo anche con un'icona e un testo, non solo con il colore.

#### Scenario: Job in corso da altro modulo
- **WHEN** è in corso un'analisi e l'utente è in Libreria
- **THEN** la barra superiore mostra "Analisi AI" con la percentuale, e un clic porta al modulo Attività su quell'attività

#### Scenario: Nessun job attivo
- **WHEN** non ci sono attività in corso
- **THEN** l'indicatore è fermo e mostra "Nessuna attività"

#### Scenario: Richiesta lenta
- **WHEN** l'utente applica un filtro il cui calcolo richiede 2 secondi
- **THEN** dopo 300 ms compare nella barra superiore l'indicatore di caricamento, che scompare all'arrivo dei risultati

#### Scenario: Memoria sotto pressione
- **WHEN** il governatore delle risorse segnala pressione di memoria
- **THEN** l'indicatore della RAM nella barra di stato mostra un'icona di avviso e il testo "Memoria quasi esaurita", con accesso al modulo Attività

### Requirement: Pannello di ispezione
Selezionando un file, il pannello di ispezione SHALL mostrare, in quest'ordine:
- anteprima e nome;
- data, luogo, persone e tag;
- cataloghi e foto simili;
- catalogazioni AI con confidenza;
- caratteristiche tecniche e percorso.

Le sezioni SHALL potersi richiudere, e lo stato SHALL essere ricordato. I campi correggibili, compresi luogo, tipo soggetto e le altre categorie, SHALL essere modificabili direttamente sul campo, senza un modulo separato di modifica. Con più foto selezionate il pannello SHALL mostrare i valori comuni e permettere di modificarli per tutte.

#### Scenario: Ispezione file
- **WHEN** l'utente seleziona una foto
- **THEN** il pannello mostra tutti i dati del file e consente di correggere il tipo soggetto facendo clic sul valore

#### Scenario: Correzione del luogo
- **WHEN** l'utente modifica il luogo di una foto da "D7000" a "Giappone" nel pannello di ispezione
- **THEN** il luogo diventa "Giappone" con fonte "manuale" e il facet "Luogo" della libreria si aggiorna

#### Scenario: Suggerimenti del luogo
- **WHEN** l'utente inizia a digitare il luogo
- **THEN** il pannello suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline

#### Scenario: Selezione multipla
- **WHEN** sono selezionate 12 foto con luoghi diversi e l'utente imposta il luogo "Roma" nel pannello
- **THEN** il pannello mostra "Valori diversi" prima della modifica, e dopo la modifica tutte le 12 foto hanno luogo "Roma", con la possibilità di annullare l'operazione

#### Scenario: Sezione richiusa
- **WHEN** l'utente richiude la sezione "Dati tecnici" e seleziona un'altra foto
- **THEN** la sezione resta richiusa

### Requirement: Stile visivo
L'interfaccia SHALL usare:
- tre temi, "Come il sistema" (predefinito), scuro e chiaro;
- contrasto del testo di almeno 4,5:1, e di almeno 3:1 per bordi dei controlli, icone e indicatore del focus;
- un solo colore d'accento;
- una scala tipografica fissa e una scala di spaziature, raggi ed elevazioni definita come token;
- un unico insieme di icone.

Le animazioni SHALL durare al massimo 200 ms e SHALL essere ridotte quando il sistema chiede movimento ridotto. Ogni pagina di modulo SHALL avere un'intestazione con titolo uguale al nome della voce di navigazione.

#### Scenario: Cambio tema
- **WHEN** l'utente seleziona il tema chiaro
- **THEN** l'intera interfaccia passa al tema chiaro senza riavvio

#### Scenario: Tema del sistema
- **WHEN** il tema è "Come il sistema" e il sistema operativo passa al tema scuro
- **THEN** l'app passa al tema scuro senza intervento dell'utente

#### Scenario: Titolo coerente
- **WHEN** l'utente apre la voce di navigazione "Importa"
- **THEN** la pagina ha come titolo "Importa"

### Requirement: Primo avvio guidato
Al primo avvio l'app SHALL guidare l'utente in tre passi:
1. benvenuto, con lingua e tema;
2. scelta della prima cartella di foto, con selettore di cartelle e trascinamento;
3. preparazione delle funzioni AI. Mostra il riepilogo del sistema in frasi comprensibili, ad esempio "Il computer ha 16 GB di memoria: profilo consigliato Bilanciato", e avvia il download dei modelli in background.

Ogni passo SHALL poter essere saltato. La verifica del sistema SHALL avvenire comunque in background. Al termine l'app SHALL aprire la Libreria, che mostra le foto man mano che vengono importate e la lista "Primi passi". L'introduzione SHALL poter essere rivista dal menu Aiuto.

#### Scenario: Download rimandato
- **WHEN** l'utente salta il passo delle funzioni AI al primo avvio
- **THEN** l'app si apre con le funzioni AI disattivate e un avviso che permette di avviare il download

#### Scenario: Fine dell'introduzione
- **WHEN** l'utente sceglie la cartella "D:\Foto" e completa l'introduzione
- **THEN** si apre la Libreria con le foto che compaiono man mano che l'importazione procede, e l'indicatore delle attività mostra l'avanzamento

#### Scenario: Rivedere l'introduzione
- **WHEN** l'utente sceglie Aiuto › "Rivedi l'introduzione"
- **THEN** l'introduzione si riapre senza cambiare libreria né impostazioni finché l'utente non conferma un passo

### Requirement: Impostazioni
Le impostazioni SHALL essere organizzate in sezioni, con una sotto-navigazione e una ricerca:
- **Generale**: tema, lingua, azione del doppio clic, percorso dei dati dell'app;
- **Libreria e importazione**: nomi di cartella da ignorare per il luogo, cartelle e formati predefiniti;
- **Riconoscimento AI**: attivazione del riconoscimento volti, sensibilità per gruppo e categoria, persone e volti, somiglianza tra volti e tra foto, tassonomia;
- **Modelli AI e sistema**: verifica del sistema, modelli, profilo risorse e limite RAM, provider di esecuzione;
- **Applicazioni esterne**;
- **Diagnostica**: livello di log, errori, log, pacchetto diagnostico;
- **Avanzate**: valori numerici delle soglie, eliminazione dei dati dei volti.

Ogni impostazione SHALL esistere in un solo punto.

Le preferenze SHALL essere salvate appena modificate, con conferma visibile accanto al campo. Le impostazioni che ricalcolano dati della libreria (soglie, tassonomia, percorso dei dati) SHALL essere applicate con un comando "Applica" e un'anteprima dell'effetto.

Le soglie SHALL essere presentate come sensibilità, su una scala da "Più risultati" a "Più precisi", e il valore numerico SHALL essere visibile in "Avanzate".

#### Scenario: Modifica profilo
- **WHEN** l'utente cambia il profilo risorse da "Bilanciato" a "Leggero"
- **THEN** l'impostazione è salvata subito e i job successivi usano il nuovo budget di memoria

#### Scenario: Soglie di gruppo preservate
- **WHEN** l'utente ha impostato soglie diverse per i gruppi della tassonomia e modifica un'altra impostazione
- **THEN** le soglie dei gruppi restano quelle impostate dall'utente

#### Scenario: Ricerca di un'impostazione
- **WHEN** l'utente scrive "log" nella ricerca delle impostazioni
- **THEN** compare la sezione Diagnostica con il campo "Livello di log" evidenziato

#### Scenario: Soglia con anteprima
- **WHEN** l'utente sposta la sensibilità del rilevamento persone verso "Più precisi"
- **THEN** la pagina mostra quante foto cambierebbero e applica la modifica solo con "Applica"

#### Scenario: Nessun duplicato
- **WHEN** l'utente cerca il provider di esecuzione
- **THEN** lo trova solo in "Modelli AI e sistema"

## ADDED Requirements

### Requirement: Barra delle azioni sulla selezione
Quando in Libreria, in un catalogo o in Sequenza è selezionata almeno una foto, la barra degli strumenti della vista SHALL essere sostituita da una barra delle azioni. La barra contiene:
- il numero di foto selezionate;
- "Deseleziona" (Esc);
- comandi diretti per Tag, catalogo (Aggiungi o Escludi), Persona, Luogo, Apri con ed Esporta;
- un menu "Altro" con le azioni restanti.

Senza selezione la barra MUST NOT essere visibile.

#### Scenario: Selezione di foto
- **WHEN** l'utente seleziona 3 foto in Libreria
- **THEN** al posto della barra degli strumenti compare "3 selezionate" con i comandi diretti, e premendo Esc la selezione si svuota e torna la barra degli strumenti

#### Scenario: Azione che cambia modulo
- **WHEN** l'utente sceglie "Esporta" con 40 foto selezionate
- **THEN** si apre il modulo Esporta con ambito "Selezione (40 foto)" già impostato

### Requirement: Selettori delle azioni sulla selezione
Ogni comando della barra delle azioni che richiede un valore SHALL aprire accanto al comando un selettore con ricerca, utilizzabile da tastiera. Quando il tipo di valore lo consente, il selettore SHALL permettere anche di creare un valore nuovo.

#### Scenario: Aggiunta di un tag
- **WHEN** l'utente fa clic su "Tag" nella barra delle azioni, scrive "mare" e preme Invio
- **THEN** il tag "mare" è aggiunto alle foto selezionate e compare un avviso con "Annulla operazione"

### Requirement: Visualizzatore delle foto
In Libreria, nei cataloghi e in Sequenza, Spazio o Invio su una miniatura con il focus SHALL aprire la foto in un visualizzatore interno. Il doppio clic SHALL eseguire l'azione configurata in Impostazioni (vedi `external-apps`, "Azione del doppio clic").

#### Scenario: Doppio clic configurato
- **WHEN** l'azione del doppio clic è "app predefinita del sistema" e l'utente fa doppio clic su una miniatura
- **THEN** la foto si apre nell'app predefinita del sistema

### Requirement: Comandi del visualizzatore
Il visualizzatore delle foto SHALL offrire:
- le frecce per scorrere le foto nell'ordine della vista;
- i tasti 1–9 e 0 per i tag del set attivo;
- I per le informazioni;
- Esc o Spazio per tornare alla griglia sulla stessa foto.

#### Scenario: Revisione rapida
- **WHEN** l'utente preme Spazio su una foto, poi → tre volte e 2
- **THEN** il visualizzatore mostra la quarta foto della vista, le assegna il tag del tasto 2 e, alla chiusura con Esc, la griglia ha il focus sulla quarta foto

### Requirement: Navigazione da tastiera nella griglia
Le griglie di Libreria, cataloghi, Sequenza e Persone SHALL essere navigabili con:
- frecce, Home/Fine e Pagina su/giù per spostare il focus;
- Maiusc con una freccia per estendere la selezione;
- Ctrl+Spazio per aggiungere o togliere la foto con il focus dalla selezione;
- Ctrl+A per selezionare tutte le foto della vista.

La miniatura con il focus SHALL essere sempre visibile e distinta da quelle selezionate.

#### Scenario: Selezione da tastiera
- **WHEN** l'utente porta il focus su una miniatura e preme Maiusc+→ due volte
- **THEN** sono selezionate tre foto consecutive e il focus è sulla terza

#### Scenario: Seleziona tutto
- **WHEN** la vista ha un filtro con 8.000 foto e l'utente preme Ctrl+A
- **THEN** sono selezionate tutte le 8.000 foto del filtro, senza caricarle tutte nell'interfaccia

### Requirement: Contesto della vista in Libreria
La Libreria SHALL mostrare in un'unica barra:
- dove si trova l'utente, come percorso, ad esempio "Tutte le foto › Catalogo Viaggio Toscana › Escluse";
- il numero di foto della vista;
- i filtri attivi come chip rimovibili;
- un menu "Ordina";
- un menu "Vista", con dimensione delle miniature e raggruppamento delle foto simili.

I filtri, compreso "Solo foto con simili", SHALL stare nel pannello Filtri e SHALL generare un chip quando sono attivi.

#### Scenario: Catalogo aperto
- **WHEN** l'utente apre il catalogo "Viaggio Toscana" dalla barra laterale
- **THEN** la barra mostra "Tutte le foto › Viaggio Toscana · 312 foto" e il comando "Escluse (6)", e facendo clic su "Tutte le foto" si torna alla libreria completa

#### Scenario: Filtro delle foto simili
- **WHEN** l'utente attiva "Solo foto con simili" nel pannello Filtri
- **THEN** nella barra compare il chip "Con foto simili" che lo rimuove con un clic
