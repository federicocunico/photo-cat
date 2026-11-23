# Proposal

## Why

Selezionare oggetti oggi funziona in modo diverso a seconda di dove si è:
- nella griglia della Libreria le frecce spostano solo il focus, e per selezionare serve Maiusc o Ctrl+Spazio. In Esplora file e nel Finder invece le frecce selezionano;
- nella griglia dei volti e in Sequenza mancano del tutto frecce e Maiusc+frecce;
- nell'elenco di Persone le frecce aprono il gruppo successivo, ma non selezionano né estendono;
- i cataloghi della pagina Cataloghi non si possono selezionare né gestire in blocco.

Chi lavora su migliaia di foto e centinaia di gruppi di volti si aspetta che tutto si selezioni **come in un gestore di file**: clic, Ctrl+clic, Maiusc+clic, frecce, Maiusc+frecce, Ctrl+A, Esc, menu contestuale sulla selezione. Questa change fissa un unico modello e lo applica a ogni raccolta di oggetti dell'app. È la base su cui `ui-usability-audit` riorganizza Persone.

## What Changes

- **Un modello di selezione comune** per foto (Libreria, cataloghi, Cestino, foto escluse, Sequenza, risultati e foto simili del modulo AI), volti, elenco di Persone (gruppi e persone) e cataloghi (pagina Cataloghi):
  - **mouse**:
    - clic seleziona solo quell'elemento;
    - Ctrl+clic aggiunge o toglie;
    - Maiusc+clic seleziona l'intervallo dall'ancora;
    - Ctrl+Maiusc+clic aggiunge un intervallo;
    - clic su uno spazio vuoto svuota la selezione;
    - selezione a rettangolo nelle griglie;
  - **tastiera**:
    - le frecce, Home/Fine e Pagina su/giù spostano il focus e selezionano l'elemento raggiunto;
    - Maiusc più gli stessi tasti estende dall'ancora;
    - Ctrl più gli stessi tasti sposta solo il focus;
    - Ctrl+Spazio aggiunge o toglie l'elemento con il focus;
    - Ctrl+A seleziona tutto, Ctrl+Maiusc+I inverte, Esc svuota;
  - **apertura**: Invio o doppio clic aprono l'elemento; Spazio mostra l'anteprima di foto e volti;
  - **menu contestuale**: clic destro su un elemento non selezionato lo seleziona prima di aprire il menu; su un elemento selezionato il menu agisce su tutta la selezione. Il tasto Menu e Maiusc+F10 aprono il menu dell'elemento con il focus;
  - **riquadro attivo**: in una schermata con più raccolte (Persone: elenco e volti) le scorciatoie e il menu Modifica agiscono sul riquadro con il focus, che si riconosce a colpo d'occhio;
  - **focus e selezione** sempre distinti e visibili; il numero dei selezionati viene annunciato ai lettori di schermo.
- **Cataloghi selezionabili**: nella pagina Cataloghi si possono selezionare più cataloghi ed eliminarli in blocco con una conferma. I cataloghi di sistema si selezionano ma non si eliminano, e il motivo è spiegato. Nelle liste con nomi (Cataloghi, elenco dei tag) basta digitare le prime lettere per raggiungere un elemento.
- **BREAKING (comportamento)**: in Libreria le frecce senza modificatori **selezionano** l'elemento raggiunto, invece di spostare solo il focus. Il vecchio comportamento resta con Ctrl+frecce.

## Capabilities

### New Capabilities

- `item-selection`: modello di selezione comune a tutte le raccolte di oggetti dell'app (mouse, tastiera, apertura, menu contestuale, riquadro attivo, ricerca per iniziale) e raccolte a cui si applica.

### Modified Capabilities

- `desktop-shell`: "Navigazione da tastiera nella griglia". Le frecce selezionano, Ctrl+frecce spostano solo il focus, e il requisito vale anche per gli elenchi.
- `catalog-library`: nuovo requisito "Eliminazione di più cataloghi".

## Impact

- **UI** (`app/ui/src`):
  - `lib/useGridSelection.ts` diventa `lib/selection/`, con un hook per la selezione (id numerici o testuali) e un hook per i tasti di griglie ed elenchi;
  - `commands/gridShortcuts.ts` passa dalla "griglia attiva" al "riquadro attivo";
  - adottano il modello: `modules/library/Grid.tsx`, `GridContextMenu.tsx`, `modules/sequence/SequencePage.tsx`, `modules/people/FaceGrid.tsx`, `GroupList.tsx`, `usePeopleHotkeys.ts`, `modules/catalogs/CatalogsPage.tsx`, `modules/tags/TagsPage.tsx` (ricerca per iniziale) e i risultati del modulo AI.
- **Rust**: un comando per eliminare più cataloghi in un'unica operazione (`delete_catalogs`), che rifiuta i cataloghi di sistema.
- **Test**: una matrice di test vitest del modello (mouse e tastiera) eseguita su ogni raccolta; i test esistenti della Libreria che si aspettano "le frecce spostano solo il focus" vanno aggiornati.
- **Documentazione**: README (scorciatoie) e foglio delle scorciatoie.
