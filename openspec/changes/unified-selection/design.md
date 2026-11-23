# Design

## Context

Motivazione in `proposal.md`; il comportamento richiesto è nella spec `item-selection`.

Stato attuale:
- **Selezione**: `lib/useGridSelection.ts` gestisce la selezione per id numerici, in due forme: id espliciti, oppure base del backend (filtro o snapshot) più aggiunte e tolte. Gestisce anche clic, Ctrl+clic, Maiusc+clic, "Seleziona tutto" e "Inverti". Lo usano Libreria, griglia dei volti e Sequenza. `store/library.ts` contiene le funzioni pure (`clickSelect`, `selSet`, `invertOf`…).
- **Tastiera**: solo la griglia della Libreria ha frecce, Pagina su/giù, Maiusc+frecce e Ctrl+Spazio (`Grid.tsx`, `onGridKey`), e lì le frecce spostano solo il focus. L'elenco di Persone usa le frecce globali di `usePeopleHotkeys` per aprire il gruppo precedente o successivo. La griglia dei volti e Sequenza non hanno frecce.
- **Instradamento**: `commands/gridShortcuts.ts` (`useActiveGrid`) manda Ctrl+A, Ctrl+Maiusc+I ed Esc alla griglia che ha avuto l'ultimo clic o il focus.
- **Elenco di Persone**: due selezioni con forme diverse, id numerici per i gruppi e `uid` testuali per le identità.
- **Cataloghi**: la pagina mostra schede con pulsanti e nessuna selezione; l'eliminazione è per singolo catalogo con `ConfirmDialog`.

## Goals / Non-Goals

**Goals:**
- Un solo modello (mouse, tastiera, menu contestuale, riquadro attivo) per tutte le raccolte, verificato da una sola matrice di test.
- Le raccolte grandi (100.000 foto, migliaia di gruppi) restano veloci: la selezione per filtro del backend si mantiene.

**Non-Goals:**
- Riorganizzare Persone (barre delle azioni, schede, verbi): lo fa `ui-usability-audit`, che parte da questo modello.
- Selezione nelle liste di navigazione: cataloghi nella barra laterale della Libreria, sezioni delle Impostazioni, sotto-navigazione del modulo AI. Lì un clic apre e basta.
- Selezione multipla di tag, cartelle di Importa e attività. Per adottarla in futuro basta il modello.

## Decisions

### D1. Una libreria `lib/selection/`
`useGridSelection` diventa `useSelection<Id extends number | string, F>`. La forma resta: id espliciti, oppure base del backend più delta. Cambia il tipo dell'id, così le identità (`uid`) e i cataloghi usano lo stesso codice. Le funzioni pure di `store/library.ts` (`clickSelect`, `selSet`…) passano in `lib/selection/model.ts` e diventano generiche. `store/library.ts` le riesporta, così i chiamanti attuali continuano a funzionare.

*Alternativa scartata*: un hook separato per gli elenchi. Duplicherebbe Maiusc+clic, l'ancora e "Inverti", proprio le regole che devono essere identiche.

### D2. Tasti in un solo hook
`useSelectionKeys({ layout: "grid" | "list", columns, pageSize, order, focused, setFocused, selection })` restituisce `onKeyDown`. La matrice "tasto × modificatore → effetto" della spec è in un'unica tabella dentro l'hook:
- nessun modificatore: sposta e seleziona solo quell'elemento;
- Maiusc: estende dall'ancora;
- Ctrl: sposta solo il focus;
- Ctrl+Spazio: aggiunge o toglie.

`Grid.tsx` sostituisce la propria `onGridKey` (frecce, Pagina, Home/Fine, Ctrl+Spazio) con l'hook e tiene solo le sue scorciatoie (Spazio e Invio per il visualizzatore, Canc, menu contestuale). Sequenza, griglia dei volti, elenco di Persone e pagina Cataloghi usano lo stesso hook.

### D3. Focus con `aria-activedescendant`
Le raccolte sono virtualizzate: gli elementi fuori vista non esistono nel DOM, quindi il focus resta sul contenitore e l'elemento attivo si indica con `aria-activedescendant`, come già fa la griglia della Libreria:
- ruoli `grid` o `listbox` con `aria-multiselectable` e `aria-selected` sulle voci;
- gli annunci del numero di selezionati passano da una regione `aria-live` condivisa;
- muovendo il focus da tastiera si chiama `scrollToIndex` del virtualizzatore.

*Alternativa scartata*: il *roving tabindex*, che con la virtualizzazione perde il focus quando l'elemento esce dal DOM.

### D4. Riquadro attivo
`useActiveGrid` diventa `useActivePane`, con la stessa interfaccia più `id` e `label`. Il riquadro attivo è l'ultimo con un clic o il focus, come oggi. In più:
- espone `activePane()` al registro dei comandi, per il menu Modifica di `ui-usability-audit` e per le voci "Seleziona tutto" della palette;
- marca il contenitore attivo con `data-active-pane`, che il CSS mostra come bordo superiore di 2 px più l'anello di focus dell'elemento: due segnali, non solo un colore.

In Persone le frecce globali di `usePeopleHotkeys` lasciano il posto all'hook del riquadro attivo: con il focus sull'elenco le frecce si muovono tra i gruppi e aprono quello raggiunto, perché "seleziona" vuol dire anche "apri"; con il focus sulla griglia si muovono tra i volti.

### D5. Menu contestuale
Un helper `selectForContextMenu(selection, id)` applica la regola della spec: se l'elemento non è selezionato, la selezione diventa solo lui; se lo è, resta tutto com'è. Lo usano `GridContextMenu`, il menu dei volti, l'elenco di Persone e i cataloghi. Le etichette ricevono il conteggio ("Sposta 5 foto nel Cestino") tramite le chiavi i18n con `_one`/`_other`. Il tasto Menu e Maiusc+F10 aprono il menu sull'elemento con il focus, alle coordinate del suo rettangolo, come già fa la Libreria.

### D6. Pagina Cataloghi
- Le schede diventano voci di una `listbox` con disposizione a griglia (`layout: "grid"`). Un clic sul corpo della scheda seleziona; doppio clic o Invio aprono.
- I pulsanti della scheda (Apri, Modifica, Elimina, comandi dei cataloghi di sistema) stanno in una fila accanto alla voce, non dentro: nessun controllo annidato.
- Con due o più cataloghi selezionati l'intestazione della pagina mostra "Elimina N cataloghi…"; anche Canc e il menu contestuale eliminano.
- La conferma elenca i nomi; i cataloghi di sistema sono esclusi con una riga che lo spiega.

**Rust**: `delete_catalogs(ids)` elimina in un'unica transazione e rifiuta gli id di sistema, restituendo quanti ne ha eliminati e quali ha saltato. `delete_catalog` resta per compatibilità e diventa un caso particolare.

### D7. Ricerca per iniziale
Un hook `useTypeAhead(order, nameOf)` accumula i caratteri digitati entro 1 s e sposta focus e selezione sul primo nome che inizia così, ignorando maiuscole e accenti. Si attiva solo nelle raccolte senza scorciatoie a lettera singola (Cataloghi, elenco dei tag), non in Persone (N, M, I) né nelle griglie di foto (1–9 per i tag).

### D8. Una matrice di test per tutte le raccolte
`test/selectionContract.tsx` esporta `runSelectionContract(name, render)`, che esegue gli scenari della spec su una raccolta montata:
- clic, Ctrl+clic, Maiusc+clic, Ctrl+Maiusc+clic, clic su spazio vuoto;
- frecce, Maiusc+frecce, Ctrl+frecce, Ctrl+Spazio, Home/Fine, Ctrl+A, Ctrl+Maiusc+I, Esc;
- clic destro su elemento selezionato e non;
- nessuna azione quando il focus è in un campo di testo.

Ogni raccolta ha il suo file di test che chiama il contratto. Così una raccolta nuova non può comportarsi diversamente senza far fallire i test.

## Risks / Trade-offs

- [Le frecce che selezionano cambiano un'abitudine della Libreria] → il vecchio comportamento resta con Ctrl+frecce; la novità va nelle note della release e nel foglio delle scorciatoie.
- [Maiusc+Pagina giù oltre gli elementi caricati in una griglia virtualizzata] → l'intervallo si calcola sull'ordine noto; prima di estendere, la griglia carica la pagina di destinazione, come già fa lo scorrimento. Per intervalli più grandi resta Ctrl+A con la base del backend.
- [Le frecce in Persone selezionano e aprono: aprire un gruppo con molti volti a ogni freccia può essere lento] → il caricamento dei volti è annullabile e parte dopo 150 ms di quiete. Tenendo premuta la freccia si scorre l'elenco senza caricare ogni gruppo.
- [I test esistenti che si aspettano "frecce = solo focus" (`Grid`, `useGridSelection`)] → si aggiornano nello stesso gruppo di task del componente.

## Migration Plan

Nessun dato cambia. Due fasi, ognuna con test verdi:
1. la libreria comune e la Libreria;
2. Sequenza, Persone, Cataloghi e Tag.

Si può tornare indietro con un revert per fase.
