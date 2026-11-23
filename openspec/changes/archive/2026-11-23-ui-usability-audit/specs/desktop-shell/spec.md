# Spec Delta

## MODIFIED Requirements

### Requirement: Moduli di navigazione
L'applicazione SHALL organizzare la navigazione in gruppi con un'etichetta, senza gruppi di una sola voce:
- **Sfoglia**: Libreria, Sequenza;
- **Organizza**: Cataloghi, Persone, Tag, Analisi AI (il modulo AI);
- **Importa ed esporta**: Importa, Esporta, Varianti RAW.

In fondo alla navigazione SHALL stare **Attività** e **Impostazioni**. La voce del modulo di revisione RAW SHALL chiamarsi "Varianti RAW"; la fotocamera supportata (Fujifilm X-T5) SHALL essere indicata nel sottotitolo della pagina.

Modelli AI e Diagnostica SHALL essere sezioni di Impostazioni e non voci di primo livello. SHALL restare raggiungibili direttamente da Attività, dalla barra di stato, dagli avvisi, dalla palette dei comandi e dalla barra dei menu. Persone e identità SHALL essere gestite in un unico modulo Persone. Il modulo AI riguarda lo stato e il controllo delle analisi; le soglie e i modelli restano in Impostazioni.

Ogni voce di primo livello SHALL avere una scorciatoia (Ctrl+1…Ctrl+9 nell'ordine dei gruppi). L'ordine delle voci e le scorciatoie MUST NOT cambiare rispetto alla versione 0.5. Un modulo SHALL poter essere aperto su un elemento specifico, ad esempio una persona, un volto, una sezione di Impostazioni, una sezione del modulo AI, un'attività o una sessione di revisione RAW, da altri moduli. La navigazione SHALL essere estendibile con nuovi moduli.

Un modulo salvato come attivo nelle preferenze che non è più una voce di primo livello SHALL aprire la sezione corrispondente.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria ad Attività e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

#### Scenario: Gruppi della navigazione
- **WHEN** l'utente apre l'app
- **THEN** la navigazione mostra i gruppi Sfoglia, Organizza e Importa ed esporta con le loro voci, e in fondo Attività e Impostazioni; nessun gruppo ha una sola voce

#### Scenario: Scorciatoia di modulo
- **WHEN** l'utente preme Ctrl+3
- **THEN** si apre il modulo Cataloghi

#### Scenario: Scorciatoia del modulo AI
- **WHEN** l'utente preme Ctrl+6
- **THEN** si apre il modulo Analisi AI

#### Scenario: Scorciatoia della revisione RAW
- **WHEN** l'utente preme Ctrl+9
- **THEN** si apre il modulo Varianti RAW sull'ultima sessione aperta, oppure sulla scelta della cartella se non ce ne sono

#### Scenario: Nome della voce RAW
- **WHEN** la navigazione è larga 200 px
- **THEN** la voce "Varianti RAW" sta su una riga, e la pagina indica nel sottotitolo che il modulo è per la Fujifilm X-T5

#### Scenario: Identità nel modulo Persone
- **WHEN** l'utente cerca nella navigazione il modulo Identità
- **THEN** non lo trova come voce separata, e le identità sono gestite nella sezione "Persone" del modulo Persone

#### Scenario: Preferenza del modulo non più esistente
- **WHEN** l'ultimo modulo aperto salvato nelle preferenze era "Identità", "Modelli AI" o "Diagnostica"
- **THEN** all'avvio si apre rispettivamente Persone sulla sezione "Persone", Impostazioni sulla sezione "Modelli AI e sistema" o Impostazioni sulla sezione "Diagnostica"

#### Scenario: Diagnostica da un errore
- **WHEN** l'utente fa clic sul conteggio degli errori nella barra di stato
- **THEN** si apre Impostazioni › Diagnostica filtrata sugli errori

### Requirement: Stile visivo
L'interfaccia SHALL usare:
- tre temi, "Come il sistema" (predefinito), scuro e chiaro;
- contrasto del testo di almeno 4,5:1, e di almeno 3:1 per bordi dei controlli, icone e indicatore del focus;
- un solo colore d'accento;
- una scala tipografica fissa di quattro corpi (12, 14, 16 e 20 px), con un solo stile per le intestazioni di sezione, e una scala di spaziature, raggi ed elevazioni definita come token;
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

#### Scenario: Scala tipografica
- **WHEN** si misurano i corpi del testo visibili in un qualsiasi modulo
- **THEN** compaiono solo 12, 14, 16 e 20 px, e tutte le intestazioni di sezione hanno lo stesso corpo e lo stesso peso

### Requirement: Impostazioni
Le impostazioni SHALL essere organizzate in sezioni, con una sotto-navigazione e una ricerca:
- **Generale**: tema, lingua, azione del doppio clic, percorso dei dati dell'app;
- **Libreria e importazione**: nomi di cartella da ignorare per il luogo, cartelle e formati predefiniti;
- **Riconoscimento AI**: attivazione del riconoscimento volti, sensibilità per gruppo e categoria, persone e volti, somiglianza tra volti e tra foto, tassonomia;
- **Modelli AI e sistema**: verifica del sistema, modelli, profilo risorse e limite RAM, provider di esecuzione;
- **Applicazioni esterne**;
- **Diagnostica**: livello di log, errori, log, pacchetto diagnostico;
- **Avanzate**: valori numerici delle soglie, cache delle anteprime di Varianti RAW, eliminazione dei dati dei volti;
- **Informazioni**: versione, build, sistema, autori, licenze.

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

#### Scenario: Cache delle anteprime RAW
- **WHEN** l'utente cerca "cache" nelle impostazioni
- **THEN** trova la cache delle anteprime di Varianti RAW in "Avanzate" e non in "Generale"

### Requirement: Revisione delle persone
Il modulo Persone SHALL mostrare "Da rivedere", "Persone" e "Ignorati" come schede con conteggio su una riga nell'intestazione della pagina. Sotto ci sono un elenco con ricerca e un menu "Ordina e filtra", i volti dell'elemento aperto con selezione multipla, e una scheda con "Chi è?", unione e ignora. Elenco e griglia SHALL restare fluidi con 100.000 volti, caricando a pagine, e aggiornarsi senza ricaricamenti periodici. Tutte le azioni SHALL essere disponibili da tastiera.

#### Scenario: Revisione da tastiera
- **WHEN** l'utente scorre "Da rivedere" con le frecce, preme N, scrive un nome e conferma
- **THEN** il gruppo diventa una persona con quel nome e il focus passa al gruppo mostrato subito sotto nell'elenco

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 volti in 3.000 gruppi
- **THEN** l'elenco e la griglia dei volti si aprono e scorrono senza blocchi percepibili

#### Scenario: Aggiornamento dopo l'analisi
- **WHEN** termina un'analisi dei volti mentre il modulo Persone è aperto
- **THEN** elenco e conteggi si aggiornano senza perdere la selezione e la posizione di scorrimento

#### Scenario: Elenco subito visibile
- **WHEN** l'utente apre "Da rivedere" in una finestra alta 900 px
- **THEN** sopra il primo gruppo dell'elenco ci sono solo il campo di ricerca e il menu "Ordina e filtra"

## ADDED Requirements

### Requirement: Scorrimento e barra di stato
Solo l'area del modulo SHALL scorrere: barra superiore, navigazione e barra di stato MUST restare ferme in ogni modulo e durante i caricamenti. La barra di stato SHALL stare sempre in fondo alla finestra, su una sola riga. Ciò che non entra SHALL essere abbreviato o mostrato nel modulo, mai mandato a capo.

#### Scenario: Pagina lunga
- **WHEN** l'utente scorre fino in fondo la pagina Esporta
- **THEN** scorre solo il contenuto della pagina; barra superiore, navigazione e barra di stato restano ferme e la finestra non mostra una propria barra di scorrimento

#### Scenario: Barra di stato durante un caricamento
- **WHEN** l'utente apre Attività mentre l'elenco è ancora in caricamento
- **THEN** la barra di stato resta in fondo alla finestra, su una riga

#### Scenario: Barra di stato in Sequenza
- **WHEN** l'utente apre Sequenza a 1440 px di larghezza
- **THEN** la barra di stato occupa una sola riga, e le scorciatoie del modulo sono nel foglio delle scorciatoie e nei menu, non nella barra di stato

### Requirement: Intestazione di pagina
L'intestazione di ogni modulo SHALL avere la stessa posizione, altezza e struttura: titolo ed eventuale sottotitolo a sinistra, eventuali schede del modulo accanto al titolo, comandi della pagina a destra. Ogni pannello richiudibile SHALL avere un solo comando per richiuderlo, con la stessa forma e posizione in tutti i moduli.

#### Scenario: Titolo nello stesso punto
- **WHEN** l'utente passa da Importa a Cataloghi e poi ad Attività
- **THEN** il titolo della pagina resta nella stessa posizione e con lo stesso corpo nei tre moduli

#### Scenario: Comando principale della pagina
- **WHEN** l'utente apre Cataloghi
- **THEN** "Nuovo catalogo" sta a destra nell'intestazione della pagina, come i comandi delle intestazioni degli altri moduli

### Requirement: Apertura e selezione in Persone
In Persone un clic semplice su una voce dell'elenco SHALL selezionare solo quella voce e aprirla. Ctrl+clic e Maiusc+clic SHALL aggiungere o estendere la selezione, come in un gestore di file. Le azioni in blocco SHALL comparire in una barra in fondo all'elenco solo con due o più voci selezionate. MUST NOT esserci pulsanti "Seleziona tutto" o "Deseleziona" sempre visibili: Ctrl+A, Esc e il menu Modifica agiscono sul riquadro che ha il focus.

#### Scenario: Clic semplice
- **WHEN** l'utente fa clic su un gruppo dell'elenco
- **THEN** quel gruppo è l'unico selezionato e si apre; non compare la barra delle azioni in blocco

#### Scenario: Selezione di più gruppi
- **WHEN** l'utente fa Ctrl+clic su tre gruppi di "Da rivedere"
- **THEN** in fondo all'elenco compare la barra "3 gruppi selezionati" con "Unisci a…", "Ignora" e "Deseleziona"

### Requirement: Ordine di avanzamento in Persone
L'avanzamento automatico dopo aver dato un nome o unito un gruppo, e le frecce su e giù, SHALL seguire l'ordine in cui l'elenco è mostrato, sezioni comprese.

#### Scenario: Ordine dopo i suggeriti
- **WHEN** l'elenco mostra due gruppi suggeriti seguiti da "Altri gruppi" e l'utente dà un nome al primo suggerito
- **THEN** si apre il secondo suggerito, cioè il gruppo mostrato subito sotto, anche se ha meno foto dei gruppi in "Altri gruppi"

#### Scenario: Freccia giù alla fine dei suggeriti
- **WHEN** è aperto l'ultimo gruppo suggerito e l'utente preme la freccia giù
- **THEN** si apre il primo gruppo di "Altri gruppi"
