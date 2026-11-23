# Spec Delta

## Purpose

Definisce un unico modo di selezionare gli oggetti in tutta l'app, come in un gestore di file: con il mouse, con la tastiera e dal menu contestuale. Lo stesso gesto ha lo stesso effetto su foto, volti, persone e cataloghi.

## ADDED Requirements

### Requirement: Raccolte selezionabili
Il modello di selezione SHALL valere per:
- le foto di Libreria, cataloghi, Cestino, foto escluse, Sequenza, risultati e foto simili del modulo AI;
- i volti della griglia di Persone;
- i gruppi e le persone dell'elenco di Persone;
- i cataloghi della pagina Cataloghi.

Ogni nuova raccolta di oggetti su cui si possono eseguire azioni SHALL adottarlo.

#### Scenario: Stesso gesto, stessa selezione
- **WHEN** l'utente fa clic sul secondo elemento e Maiusc+clic sul quinto, una volta nella griglia della Libreria e una volta nell'elenco di Persone
- **THEN** in entrambi i casi risultano selezionati gli elementi dal secondo al quinto

### Requirement: Selezione con il mouse
In ogni raccolta un clic SHALL selezionare solo l'elemento e farne l'ancora. Ctrl+clic (Cmd+clic su macOS) SHALL aggiungerlo o toglierlo e farne l'ancora. Maiusc+clic SHALL selezionare l'intervallo dall'ancora senza spostarla, e Ctrl+Maiusc+clic SHALL aggiungere l'intervallo alla selezione.

#### Scenario: Intervallo aggiunto
- **WHEN** nell'elenco di Persone l'utente fa clic sul primo gruppo, Ctrl+clic sul quarto e Ctrl+Maiusc+clic sul sesto
- **THEN** risultano selezionati il primo gruppo e i gruppi dal quarto al sesto

### Requirement: Spazio vuoto e rettangolo
Un clic su uno spazio vuoto di una raccolta SHALL svuotare la selezione. Nelle griglie, trascinare da uno spazio vuoto SHALL disegnare un rettangolo di selezione; con Ctrl il rettangolo SHALL aggiungere alla selezione.

#### Scenario: Clic su uno spazio vuoto
- **WHEN** nella pagina Cataloghi sono selezionati due cataloghi e l'utente fa clic sotto l'ultima scheda
- **THEN** la selezione si svuota

#### Scenario: Rettangolo con Ctrl
- **WHEN** nella griglia dei volti sono selezionati due volti e l'utente trascina con Ctrl un rettangolo su altri quattro
- **THEN** risultano selezionati sei volti

### Requirement: Selezione da tastiera
Quando una raccolta ha il focus, frecce, Home, Fine, Pagina su e Pagina giù SHALL spostare il focus e selezionare solo l'elemento raggiunto, che diventa l'ancora. Maiusc con gli stessi tasti SHALL estendere la selezione dall'ancora; Ctrl con gli stessi tasti SHALL spostare solo il focus; Ctrl+Spazio SHALL aggiungere o togliere l'elemento con il focus. Nelle griglie le frecce si muovono su righe e colonne, negli elenchi solo su e giù.

#### Scenario: Frecce senza modificatori
- **WHEN** in Libreria è selezionata la foto 4 e l'utente preme la freccia destra
- **THEN** è selezionata solo la foto 5, che ha il focus

#### Scenario: Estendere con Maiusc
- **WHEN** nell'elenco di Persone è selezionato il secondo gruppo e l'utente preme Maiusc+giù tre volte
- **THEN** risultano selezionati i gruppi dal secondo al quinto e il focus è sul quinto

#### Scenario: Selezione sparsa da tastiera
- **WHEN** l'utente seleziona la foto 2, preme Ctrl+destra tre volte e poi Ctrl+Spazio
- **THEN** risultano selezionate le foto 2 e 5, e il focus è sulla 5

### Requirement: Comandi di selezione da tastiera
Ctrl+A (Cmd+A su macOS) SHALL selezionare tutti gli elementi della raccolta attiva, anche quelli non ancora caricati, Ctrl+Maiusc+I SHALL invertire la selezione ed Esc SHALL svuotarla. Le scorciatoie di selezione MUST NOT agire quando il focus è in un campo di testo.

#### Scenario: Seleziona tutto i gruppi
- **WHEN** l'elenco "Da rivedere" ha 420 gruppi, ne mostra 60 e l'utente preme Ctrl+A con il focus sull'elenco
- **THEN** risultano selezionati tutti i 420 gruppi

#### Scenario: Campo di testo
- **WHEN** il focus è nella ricerca dell'elenco di Persone e l'utente preme Ctrl+A
- **THEN** si seleziona il testo del campo e la selezione dei gruppi non cambia

### Requirement: Apertura degli elementi
Invio e doppio clic SHALL aprire l'elemento con il focus: una foto nel visualizzatore, un catalogo in Libreria, una persona o un gruppo nei riquadri di Persone. Spazio SHALL mostrare l'anteprima di foto e volti. Aprire un elemento MUST NOT cambiare la selezione.

#### Scenario: Aprire un catalogo da tastiera
- **WHEN** nella pagina Cataloghi il focus è su "Viaggio Toscana" e l'utente preme Invio
- **THEN** si apre la Libreria sul catalogo "Viaggio Toscana"

### Requirement: Menu contestuale sulla selezione
Un clic destro su un elemento non selezionato SHALL selezionarlo, al posto della selezione precedente, prima di aprire il menu. Un clic destro su un elemento selezionato SHALL lasciare la selezione com'è. Le voci del menu SHALL agire su tutta la selezione e dirne il numero quando gli elementi sono più di uno. Il tasto Menu e Maiusc+F10 SHALL aprire il menu dell'elemento con il focus.

#### Scenario: Menu su una selezione
- **WHEN** sono selezionate 5 foto e l'utente fa clic destro su una di esse
- **THEN** le 5 foto restano selezionate e il menu propone, ad esempio, "Sposta 5 foto nel Cestino"

#### Scenario: Menu su un elemento non selezionato
- **WHEN** sono selezionate 5 foto e l'utente fa clic destro su una sesta foto
- **THEN** è selezionata solo la sesta foto e il menu agisce su di essa

### Requirement: Riquadro attivo
Quando una schermata mostra più raccolte, ad esempio l'elenco dei gruppi e la griglia dei volti in Persone, le scorciatoie di selezione, il menu contestuale e il menu Modifica SHALL agire sulla raccolta che ha avuto per ultima il focus o il clic. Il riquadro attivo SHALL essere riconoscibile senza affidarsi solo al colore. Ogni raccolta SHALL conservare la propria selezione quando il riquadro attivo cambia.

#### Scenario: Ctrl+A nel riquadro giusto
- **WHEN** l'utente fa clic su un volto e poi preme Ctrl+A
- **THEN** si selezionano tutti i volti del gruppo aperto e la selezione dell'elenco dei gruppi non cambia

### Requirement: Focus e selezione visibili
L'elemento con il focus SHALL essere sempre visibile, con un indicatore diverso da quello della selezione; muovendo il focus da tastiera la raccolta SHALL scorrere per mostrarlo. Il numero degli elementi selezionati SHALL essere mostrato e annunciato ai lettori di schermo a ogni cambiamento.

#### Scenario: Focus fuori vista
- **WHEN** l'utente preme Pagina giù tre volte nella griglia della Libreria
- **THEN** la griglia scorre in modo che la foto con il focus sia visibile, e il lettore di schermo annuncia "1 selezionata"

### Requirement: Ricerca per iniziale
Negli elenchi con un nome per elemento e senza scorciatoie a lettera singola (pagina Cataloghi, elenco dei tag) digitare dei caratteri entro un secondo l'uno dall'altro SHALL portare il focus, e la selezione, sul primo elemento il cui nome inizia con quei caratteri.

#### Scenario: Raggiungere un catalogo
- **WHEN** nella pagina Cataloghi l'utente digita "vi"
- **THEN** focus e selezione passano al catalogo "Viaggio Toscana"
