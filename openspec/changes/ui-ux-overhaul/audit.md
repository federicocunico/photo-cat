# Audit UI/UX di photo-cat

Data: 2026-11-03.

**Base analizzata**:
- `main` f40beb5, che contiene già `preferences-survive-reinstall`, `large-library-stability`, `similar-photo-stacks` e `catalog-exclusions`;
- le change ancora sui rispettivi branch: `loading-feedback` e `people-faces-workspace`, con il suo mockup `People.html`.

**Aggiornamento dopo il rebase** (stessa data): il branch `feat/ui-ux-overhaul` ora parte dalla release 0.3.0, che integra anche `loading-feedback` e `people-faces-workspace`. Tutti i riferimenti `file:riga` sono stati ricontrollati su questo codice. I punti già risolti restano nell'elenco, segnati come **Risolto** o **In parte risolto**. I punti nuovi sono segnati con *(nuovo)*.

**Metodo**:
- lettura del codice di shell, moduli e i18n;
- inventario dei controlli e dei testi;
- confronto con software dello stesso tipo e con linee guida pubbliche (fonti in fondo).

**Legenda**: le priorità sono **A** = alta, **M** = media, **B** = bassa. I riferimenti `file:riga` si riferiscono a `app/ui/src`.

---

## 1. Sintesi

L'app ha tutte le funzioni che servono a chi cataloga molte foto: faccette gerarchiche, cataloghi smart con esclusioni, set di tag con tasti rapidi, riconoscimento volti, gruppi di foto simili, sequenze multi-camera con export, organizzazione su disco, governo delle risorse. Il problema non è cosa manca, ma **come è presentato**:

1. **Struttura piatta**:
   - 12 voci di navigazione allo stesso livello, senza separare il lavoro quotidiano dagli strumenti e dalle pagine tecniche;
   - Hick e Miller: più scelte non raggruppate rallentano ogni decisione;
   - Fluent consiglia 5–10 voci e Impostazioni in fondo.
2. **Libreria sovraccarica**:
   - sopra la griglia si impilano intestazione del catalogo, avviso, barra degli strumenti con due checkbox, barra bulk sempre visibile e pannelli flottanti;
   - le azioni sulla selezione sono un modulo "Azione ▾ / Valore / Applica" invece di comandi diretti.
3. **Nessun sistema di componenti**:
   - ogni pagina reinventa dialoghi, schede, tabelle, intestazioni e messaggi;
   - ne derivano 4 modi diversi di chiedere conferma, finestre modali in cui Esc non funziona e titoli di pagina di 5 dimensioni diverse (o assenti).
4. **Linguaggio da sviluppatore**:
   - codici (`eq`, `people_count`, `import`, `AUTO`, `DIRECT_ML`), numeri 0–1, glob, hash;
   - termini incoerenti (vedi §6).
5. **Nessun aiuto dopo il primo avvio**:
   - niente scorciatoie visibili (le principali sono solo nella barra di stato di Sequenza), niente palette, niente aiuto contestuale;
   - l'onboarding è un modulo di dati grezzi.

Le change ora integrate migliorano parti importanti:
- `loading-feedback` porta stati di caricamento, toast e pulsanti "in corso";
- `people-faces-workspace` porta un modulo Persone unico e da tastiera.

Restano però locali: `QueryView` ha lo stato vuoto disattivato in 29 usi su 35 (`isEmpty={() => false}`), solo Persone ha scorciatoie e Ctrl+Z. Questa proposta le generalizza.

---

## 2. Shell e navigazione

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| S1 | 11 voci piatte: Libreria, Sequenza, Cataloghi, Tag, Persone, Importa, Job, Modelli AI, Diagnostica, Esporta e organizza, Impostazioni. Diagnostica e Modelli stanno in mezzo ai contenuti; "Esporta e organizza" sta dopo Diagnostica. — **In parte risolto da `people-faces-workspace`**: la voce Identità non c'è più (è dentro Persone). Il resto vale ancora. | `modules/registry.ts:8-20` | A |
| S2 | La navigazione è sempre larga 200 px e non si richiude; non ci sono scorciatoie per cambiare modulo. | `ModuleNav.tsx:13` | M |
| S3 | La ricerca globale filtra solo la Libreria e ci porta l'utente anche se lui stava altrove. Non cerca persone, cataloghi, tag o comandi come elementi. | `TopBar.tsx:33-47` | M |
| S4 | Barra superiore: il pallino dei job dice "Nessun job attivo" e gira sempre — **Risolto da `loading-feedback`**: senza job mostra un'icona ferma (`TopBar.tsx:81`). Resta il chip RAM, che porta a Job: è un'informazione tecnica sempre in vista. | `TopBar.tsx:92-121` | B |
| S5 | La barra di stato mostra contenuti diversi a seconda del modulo; le scorciatoie di Sequenza ci finiscono come testo lungo. | `ReadyApp.tsx:111-125`, `SequencePage` | B |
| S6 | Il modulo chiamato "Importa" ha come titolo "Aggiungi una sorgente"; "Esporta e organizza" contiene due funzioni senza rapporto con Sequenza › Esporta. | `i18n/it.ts:122`, `i18n/screens.ts:78` | M |
| S7 | Le impostazioni sono duplicate: provider e profilo stanno in Modelli (salvataggio immediato) e in Impostazioni (con "Salva"); il livello di log sta in Diagnostica (immediato) e in Impostazioni (con "Salva"); le etichette sono diverse ("DirectML (GPU)" contro `DIRECT_ML`, ottenuto con `toUpperCase`). | `ModelsPage.tsx:312`, `DiagnosticsPage.tsx:244`, `SettingsPage.tsx:169-187,253` | A |
| S8 | Non c'è un menu Aiuto, né un elenco delle scorciatoie, né una palette dei comandi. Le scorciatoie nuove di Persone (N, M, X, U, I, Ctrl+Z) non sono scritte da nessuna parte. | `people/usePeopleHotkeys.ts:5-20` | A |
| S9 | *(nuovo)* La schermata di avvio è sempre in italiano: la lingua salvata si applica solo dopo, in `ReadyApp`. Prima ancora compare la schermata statica bilingue "Apertura… / Opening…", con un carattere e un titolo diversi da quelli di `BootScreen`: al passaggio il testo salta. | `i18n/index.ts:48`, `ReadyApp.tsx:88`, `index.html:21-22`, `BootScreen.tsx:21` | B |
| S10 | *(nuovo)* Se l'avvio fallisce, `BootScreen` mostra il messaggio grezzo del backend e il percorso dei log in monospazio; manca un pulsante per aprire la cartella dei log. Nelle fasi compare il gergo "runtime AI". | `BootScreen.tsx:25-26`, `i18n/feedback.ts:21` | B |

**Riferimenti**:
- **Lightroom Classic**: moduli e filmstrip, con Ctrl+Alt+1–5 per cambiare modulo e Tab per i pannelli.
- **Apple Foto** e **Google Foto**: barra laterale a gruppi (Libreria · Raccolte · Album), con le utility separate.
- **Immich**: le code di lavoro stanno in Amministrazione, lontano dai contenuti.
- **PhotoPrism**: permette di nascondere le sezioni che non si usano.
- **Linear 2026**: la navigazione si fa "più silenziosa" per lasciare spazio al contenuto.

---

## 3. Libreria (modulo principale)

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| L1 | Fino a 5 fasce sopra la griglia (CatalogHeader, CatalogFeedback, Toolbar, BulkBar, griglia), più i pannelli flottanti. La griglia, cioè il contenuto, perde spazio verticale. | `LibraryPage.tsx:107-118` | A |
| L2 | La barra degli strumenti mescola cose diverse: ordinamento (select), dimensione (slider senza valori), "Raggruppa foto simili" (preferenza di vista) e "Solo foto con simili", che è un filtro ma non sta tra i filtri. — **In parte risolto**: "Solo foto con simili" ora genera un chip (`chips.ts:39`). Resta una casella nella barra, lontana dagli altri filtri. | `LibraryPage.tsx:20-78` | M |
| L3 | La barra bulk è sempre visibile, anche con 0 selezionati. Il flusso è: scegliere un'azione da 12 voci, poi un valore, poi "Applica". Due azioni ("Esporta", "Organizza") cambiano modulo senza dirlo. "Seleziona tutto (filtro)" usa un linguaggio da sviluppatore. Prima del menu ci sono altri 5 pulsanti sempre attivi (Seleziona tutto, Inverti, Tutte tranne la principale, Raggruppa come simili, Deseleziona). | `BulkBar.tsx:124-291` | A |
| L4 | Non c'è un visualizzatore interno: il doppio clic apre un'app esterna. Sequenza invece ha uno schermo intero (Spazio). Due comportamenti diversi per lo stesso gesto. | `Grid.tsx:632`, `SequencePage.tsx:908` | A |
| L5 | La griglia non si naviga con le frecce; Canc ed Invio funzionano solo dentro un catalogo. | `Grid.tsx:509-526` | M |
| L6 | Barra laterale: sorgenti (percorsi mono troncati alle ultime 2 cartelle), cataloghi e filtri in un'unica colonna, senza sezioni richiudibili. Icone testuali "▸ ▾ ↻ ✎" con bersagli di circa 20 px. Un "↻ aggiorna cartella" accanto a ogni sottocartella: è un'azione distruttiva per il tempo e sta accanto al filtro. | `Sidebar.tsx:117-150,207-211,319` | M |
| L7 | Inspector: si apre con un modulo generico "Modifica" (select del gruppo + valore + tag + catalogo) prima delle informazioni. La modifica non è sul campo. Le sezioni non si possono richiudere. — **In parte risolto**: il modulo "Modifica" ora sta in fondo, dopo le informazioni (`Inspector.tsx:397`). Restano la modifica non sul campo e le sezioni non richiudibili. | `Inspector.tsx:101,397` | M |
| L8 | L'intestazione del catalogo è testo, conteggio e un pulsante "Escluse (N)", senza gerarchia visiva. La vista delle escluse si raggiunge solo da qui (Cataloghi non mostra il conteggio). | `CatalogHeader.tsx:32-37`, `CatalogsPage.tsx:99` | B |
| L9 | Il toast di esclusione è l'unico uso di `Toast`: non ha pulsante di chiusura e non si impila. — **In parte risolto da `loading-feedback`**: `ToastHost` impila fino a 5 avvisi (`ToastHost.tsx:8`). Restano: nessun pulsante di chiusura; il toast di esclusione e quello di annullamento di Persone non passano da `ToastHost`, quindi compaiono nello stesso angolo e si sovrappongono agli altri; il toast sparisce dopo 8 s anche se il puntatore o il focus sono sopra (WCAG 2.2.1); gli errori usano lo stesso toast neutro (`role="status"`) degli avvisi. | `Toast.tsx:17-28`, `catalogActions.tsx:54`, `people/FaceGrid.tsx:547`, `BusyButton.tsx:35` | M |
| L10 | *(nuovo)* Il badge dei simili sulla miniatura è un `span role="button"` dentro il `<button>` della miniatura: un controllo annidato in un altro, come in Sequenza. Il badge è piccolo (circa 20 px) e da aperto mostra "−3". | `Grid.tsx:105,199-231` | M |
| L11 | *(nuovo)* Aprire un gruppo di simili o selezionarlo carica i membri senza alcun indicatore; se la richiesta fallisce non succede nulla e l'errore non si vede. | `Grid.tsx:399,411` | B |
| L12 | *(nuovo)* "Foto simili" nell'Inspector: "Sciogli gruppo" agisce subito, senza conferma né annullamento; i pulsanti non sono `BusyButton` e si possono premere due volte; durante il caricamento o in caso di errore la sezione sparisce senza dire nulla. La principale e la foto corrente si distinguono solo dal colore del bordo (giallo di avviso contro accento). | `SimilarSection.tsx:29,64-70,87-104` | M |
| L13 | *(nuovo)* Il filtro Persone della barra laterale è fatto a mano: mostra solo le prime 8 persone senza "Mostra altre", segna la persona attiva solo con il grassetto (senza `aria-pressed`), e il clic su un nome senza id non fa nulla. Ogni riga ripete un pulsante "Apri in Persone" con lo stesso nome; lo stesso accade per ogni volto nell'Inspector. | `Sidebar.tsx:345-389`, `Inspector.tsx:349` | B |

**Riferimenti**:
- **Google Foto** e **Apple Foto**: quando si seleziona qualcosa, la barra superiore diventa la barra delle azioni sulla selezione.
- **Lightroom**: visualizzatore (E/Loupe), griglia (G), frecce e Spazio.
- **Photo Mechanic**: valutazione e tag da tastiera, con avanzamento automatico.
- **Mylio QuickReview**: revisione rapida senza interfaccia intorno.
- **Apple HIG Toolbars**: poche azioni nella barra, il resto in un menu di overflow, e l'interruttore dell'inspector in fondo.

---

## 4. Altri moduli

### Importa
- **A**: la finestra modale non sposta il focus e Esc non funziona finché l'utente non ci entra con Tab (`import/manage.tsx:46-50`).
- **M**: "Avvia importazione" porta subito a Job (`ImportPage.tsx:223`) e l'utente lascia la pagina senza una conferma.
- **M**: i pattern glob sono in vista (`**/.thumbnails/**`; `ImportPage.tsx:199`).
- **M**: fino a 5 pulsanti per riga di sorgente (`ImportPage.tsx:74-108`).
- **B**: la colonna destra resta vuota quando non c'è un import in corso; anche l'elenco delle sorgenti sparisce del tutto quando è vuoto (`ImportPage.tsx:30`).

### Cataloghi
- **A**: l'editor delle regole mostra codici grezzi (`tag, person, people_count, width…`, `eq, neq, gte…`; `RuleEditor.tsx:100-104,112-118`).
- **M**: `window.confirm` per l'eliminazione, e il pulsante non è segnalato come pericoloso (`CatalogsPage.tsx:114-124`).
- **M**: l'editor compare sopra la griglia senza titolo (`CatalogsPage.tsx:54`).
- **B**: conteggio senza unità ("Smart · 123"), nessuno stato vuoto (`QueryView` con `isEmpty={() => false}`), il numero di foto escluse non è mostrato (`CatalogsPage.tsx:92,99`).
- **Riferimento**: Smart Album di Apple e Capture One, raccolte smart di Lightroom. Le regole si leggono come frasi ("Anno è 2023 e tag contiene Mare"), con il conteggio aggiornato mentre si scrive.

### Tag
- **A**: la nota dice "si possono annullare con Annulla", ma nella pagina non c'è nessun Annulla (`TagsPage.tsx:701`).
- **M**: la conferma di "Elimina set" compare nel pannello di destra, lontano dal pulsante (`TagsPage.tsx:306,852`).
- **M**: gli errori dei tasti rapidi vengono ignorati senza avviso (`useTagHotkeys.ts:76`).
- **B**: "Rinomina" compare due volte (`TagsPage.tsx:451,581`); colori fissi in `TagChip` (`TagChip.tsx:28`).

### Persone (integrato con `people-faces-workspace`)
Punti dell'audit precedente:
- **A**:
  - l'unione avviene senza conferma e senza annullamento — **Risolto da `people-faces-workspace`**: `MergeDialog` chiede conferma e Ctrl+Z annulla (`people/MergeDialog.tsx`, `PeopleModule.tsx:262`). Resta: dopo l'unione non compare un toast con "Annulla", quindi chi non conosce Ctrl+Z non lo sa;
  - lo stato vuoto rimanda a una "rianalisi da Modelli AI" che non esiste — **Risolto**: i testi vuoti ora spiegano cosa fare (`i18n/features.ts:131-133`).
- **M**:
  - id interni in vista ("Persona senza nome 12", `#1234`) — **Risolto**: i gruppi senza nome si chiamano "Gruppo di N volti" (`GroupList.tsx:95-98`);
  - "Apri in Persone" non apre la persona — **Risolto**: `OpenInPeople` apre la persona o il volto indicato (`people/OpenInPeople.tsx:21`).

Il nuovo modulo risolve questi punti. Questa change lo allinea solo ai componenti e alle regole comuni, senza riprogettarlo. Problemi trovati nel modulo integrato *(nuovi)*:
- **A**: "Sposta in…" agisce appena cambia il valore della select. Su Windows le frecce su una select chiusa cambiano il valore, e il tasto M porta il focus proprio lì: i volti si spostano per sbaglio (`FaceGrid.tsx:357-368`, `usePeopleHotkeys.ts:91-95`).
- **M**:
  - scorciatoie di una sola lettera (N, M, X, U, I) senza modificatori, senza modo di disattivarle e non elencate da nessuna parte (WCAG 2.1.4; `usePeopleHotkeys.ts:86-110`);
  - I (ignora) e Ctrl+Z agiscono senza alcun riscontro: nessun toast, errori ignorati (`PeopleModule.tsx:248-273`);
  - due modi diversi di unire: i gruppi con `MergeDialog` modale e annullabile, le identità con una select in linea nella scheda (`IdentityEditor.tsx:365-411`);
  - numeri 0–1 senza spiegazione: punteggio del nome suggerito ("forse Anna · 0,87"), somiglianza nel suggerimento di unione, qualità sotto ogni volto, soglia dell'identità, punteggi in "Chi è?" (`GroupList.tsx:108`, `FaceGrid.tsx:481,717`, `IdentityEditor.tsx:259`, `WhoIs.tsx:176`);
  - "Rimuovi volto" usa ancora `window.confirm`, mentre le altre correzioni si annullano (`FaceGrid.tsx:411`);
  - `MergeDialog` non sposta il focus all'apertura e non lo trattiene (`MergeDialog.tsx:102-105`).
- **B**:
  - ruoli ARIA sbagliati: `role="row"` su `<button>` nella tabella delle persone, `role="option"` senza `listbox` e con dentro il pulsante "Ripristina", `role="grid"` senza righe né celle, `aria-selected` su `<button>` (`GroupList.tsx:475,487,602,659`, `FaceGrid.tsx:573`);
  - il pulsante principale del suggerimento e di `MergeDialog` usa la classe `btn primary` invece di `btn-primary`, quindi non ha lo stile principale (`FaceGrid.tsx:487`, `MergeDialog.tsx:205`);
  - il motivo dello scarto di un volto è solo nel `title`, invisibile da tastiera (`FaceGrid.tsx:684`);
  - un secondo `<main>` dentro quello della shell (`PeopleModule.tsx:442`); titolo di pagina a 18 px, un'altra dimensione ancora (`FaceGrid.tsx:328`);
  - il caricamento dell'elenco è un testo "Caricamento…" invece dello `Skeleton` usato altrove (`GroupList.tsx:468,566`);
  - la scheda mostra il percorso del file delle identità in monospazio (`PersonCard.tsx:229`).

### Sequenza
- **A**: un pulsante annidato in un altro (badge dei simili, `SequencePage.tsx:238/295`).
- **M**:
  - la pagina non ha titolo (`SequencePage.tsx:926-929`);
  - date ISO grezze nelle intestazioni di sessione (`SequencePage.tsx:468`);
  - "Pausa tra sessioni" si salva all'uscita dal campo senza alcun riscontro (`SequencePage.tsx:185-188`);
  - gergo nell'export ("Crominanza 4:4:4", "SSIM ≥ 0.985", "Hard link"; `i18n/modules.ts:246-256`).
- **B**: durante il caricamento non si vede nessun indicatore e la griglia resta vuota; `loading-feedback` non ha toccato questa pagina (`SequencePage.tsx:869-878,984`).

### Esporta e organizza
- **A**:
  - l'ambito predefinito è "Selezione", che vale 0 se non è selezionato nulla, quindi tutto è disabilitato senza spiegazione;
  - "Crea cartelle" richiede di aver lanciato sia l'anteprima sia la verifica, e non lo dice (`OrganizePage.tsx:32,78,355`).
- **M**:
  - pulsanti "↑" e "×" senza etichetta (`OrganizePage.tsx:280,287`);
  - lo storico è in forma grezza ("#12 · D:\out · 340"; `OrganizePage.tsx:367`);
  - "Annulla" significa "torna indietro" (`OrganizePage.tsx:381`).

### Job
- **A**: il tipo di job compare come identificatore inglese (`#12 · import`; `JobsPage.tsx:126,244`).
- **M**:
  - "Annulla" ferma un job senza conferma e senza gestire l'errore (`JobsPage.tsx:151-158`) — **In parte risolto da `loading-feedback`**: ora è un `BusyButton`, che mostra l'errore in un toast (con il testo grezzo del backend). Manca ancora la conferma;
  - lo storico non ha titolo (`JobsPage.tsx:234-236`).
- **B**: lo stato vuoto non ha un pulsante per andare a Importa (`JobsPage.tsx:392`).

### Modelli AI
- **M**:
  - impostazioni duplicate (vedi S7);
  - una colonna SHA-256 (`ModelsPage.tsx:118-120,391`);
  - "Verifica tutti" è l'azione principale, mentre l'utente comune vuole "Scarica" (`ModelsPage.tsx:361-365`).
- **Gergo**: Runtime AI, ONNX, Provider, Self-test, Budget.

### Diagnostica
- **A**: le righe della tabella si selezionano solo con il mouse (`<tr onClick>` senza tabIndex, `DiagnosticsPage.tsx:387-390`).
- **M**:
  - gli errori appaiono come notizie neutre;
  - la fase `modelli` non è tradotta (`DiagnosticsPage.tsx:21`, manca in `i18n/it.ts:80-96`);
  - il livello di log è duplicato (`DiagnosticsPage.tsx:244-249`), con i valori grezzi `error`, `warn`…

### Impostazioni
- **A**:
  - "Salva" sta a metà pagina, mentre gli avvisi compaiono in cima, fuori schermo (`SettingsPage.tsx:124,329`);
  - "Elimina tutti i dati dei volti" ha lo stile di un pulsante normale (`SettingsPage.tsx:233-250`).
- **M**: tre modi di salvare diversi (Salva, Applica soglie, Salva tassonomia) e un unico blocco senza titoli che mescola risorse, AI, aspetto e percorsi.
- **Linguaggio**: "embedding", valori 0–1 con nomi diversi per lo stesso concetto (vedi §6).

### Onboarding
- **M**: il passo "sistema" mostra numeri senza etichette ("16 GB RAM · 8 CPU · 4000 MB") e non si può saltare (`Onboarding.tsx:69,154`).
- **M**: il passo "modelli" mostra "3 / 7" e blocca finché l'utente non preme (`Onboarding.tsx:79-80`).
- **M**: il percorso si scrive in un campo di testo (`Onboarding.tsx:98`).
- **M**: alla fine porta a Job invece che alle foto (`Onboarding.tsx:130`).
- **M**: non si può rivedere.

### Pannelli flottanti
- **B**: spostamento e ridimensionamento solo con il mouse (WCAG 2.5.7 chiede un'alternativa al trascinamento).
- **B**: `z-index` in conflitto con Toast e Modal: il pannello parte da 40 e sale a ogni clic, i toast stanno a 40 e 50, le modali a 60 (`FloatingPanel.tsx:112`, `Toast.tsx:28`, `feedback.css:52`).

---

## 5. Problemi trasversali

| Tema | Stato attuale | Norma di riferimento |
|---|---|---|
| Dialoghi | Nessun componente condiviso; 7 implementazioni (compresa `MergeDialog` di Persone). Solo 3 spostano il focus all'apertura, nessuna lo intrappola. | WCAG 2.4.3, 2.4.11; APG Dialog |
| Conferme | 6 `window.confirm`, 3 schede "alertdialog" in linea, 2 modali, 1 testo che fa da conferma. Alcune azioni annullabili chiedono conferma (rimuovere dalla libreria, rimuovere un volto), mentre altre senza annullamento non la chiedono (fermare un job, sciogliere un gruppo di simili). Unire persone ora ha conferma e annullamento (`people-faces-workspace`). | NN/g "Confirmation dialogs": conferma solo per le azioni gravi e irreversibili, altrimenti annullamento |
| Annullamento | Esiste per bulk, esclusioni e correzioni di Persone, ma è legato alla barra bulk, a un toast o a Ctrl+Z solo dentro Persone; non c'è un "Annulla" globale. | Nielsen #3 (controllo e libertà) |
| Messaggi | Circa 40 `role="alert"` con colori diversi; errori mostrati come `detail` grezzo del backend. `loading-feedback` ha aggiunto `QueryView` e i toast, ma anche lì l'errore è il testo grezzo: solo 3 `QueryView` su 35 passano un messaggio proprio, e `BusyButton` mette l'errore grezzo in un toast neutro (`QueryView.tsx:39`, `BusyButton.tsx:35`). | NN/g: validazione vicino al campo, notifiche per gli eventi di sistema |
| Stati vuoti | Testi senza azione; in Cataloghi, sorgenti di Importa, storico di Organizza e Modelli mancano del tutto. `QueryView` ha uno stato vuoto, ma 29 usi su 35 lo spengono con `isEmpty={() => false}`. | NN/g Empty states: stato, spiegazione, azione diretta |
| Caricamento | **In gran parte risolto da `loading-feedback`**: `QueryView`, `Skeleton`, `RefreshBar`, `BusyButton` e l'indicatore delle richieste in corso coprono quasi tutte le pagine. Restano scoperti Sequenza, i gruppi di simili nella griglia e la sezione "Foto simili" (vedi L11, L12), e Persone usa un testo al posto dello `Skeleton`. | NN/g: tempi di risposta, indicatori di avanzamento |
| Tipografia | Corpi 11, 12, 13, 14, 16, 17, 18, 20, 26 e h1/h2 del browser; il titolo di pagina è assente in Libreria e Sequenza, a 18 px in Persone e senza stile in 4 pagine. | Gerarchia tipografica chiara (requisito "Stile visivo") |
| Spaziatura e forma | Valori fissi (4, 6, 8, 10, 12, 14, 16, 20, 24…); nessun token per spaziatura, raggio ed elevazione. | Fluent 2 / design token |
| Icone | Glifi testuali (▸ ▾ ↻ ✎ × ↑ − ‹) mescolati alle icone SVG. | Coerenza (Nielsen #4) |
| Accessibilità | Bersagli da 20–22 px; pulsanti annidati (Sequenza e ora anche Libreria, L10); righe non raggiungibili da tastiera; colore come unico segnale (barra del dispositivo, principale dei simili); `role="row"` su `<button>` e altri ruoli ARIA sbagliati in Persone; scorciatoie di una sola lettera non disattivabili (Persone, Tag). | WCAG 2.2: 2.5.8, 2.1.1, 2.1.4, 4.1.2, 1.4.1 |
| Gergo | Job, Smart, Provider, Runtime, Self-test, SHA-256, glob, embedding, SSIM, `eq`/`gte`. | Nielsen #2 (corrispondenza con il mondo reale) |

---

## 6. Glossario: incoerenze trovate e scelta proposta

| Concetto | Oggi | Proposta |
|---|---|---|
| Lavoro in background | Job, job, "#12 · import" | **Attività** ("Importazione di D:\Foto") |
| Chiudere un dialogo senza fare nulla | Annulla | **Annulla** (solo qui) |
| Fermare un'attività | Annulla | **Interrompi** |
| Tornare indietro su un'operazione | Annulla, "Annulla ultima azione" | **Annulla operazione** (toast, Ctrl+Z, storico) |
| Togliere da un contenitore | Rimuovi / Escludi | **Rimuovi da …**, oppure **Escludi da …** per i cataloghi |
| Cancellare dati dell'app | Elimina / Rimuovi | **Elimina** (sempre con un'indicazione di ciò che resta sul disco) |
| Aggiungere foto | Importa / Import / Aggiungi una sorgente | **Importa** (modulo e titolo); "cartella" invece di "sorgente" nella UI |
| Esportare | "Esporta e organizza", Sequenza › Esporta | Modulo **Esporta** con le sezioni "Esporta dati", "Organizza in cartelle" e "Sequenza" |
| Valore 0–1 di affidabilità | Soglia di confidenza, Somiglianza minima, Somiglianza tra volti, Soglia del gruppo, Somiglianza dei gruppi di foto simili; in Persone anche punteggi senza nome ("forse Anna · 0,87", qualità sotto i volti) | **Sensibilità** con scala "Più risultati ↔ Più precisi" e percentuale; il valore numerico resta in "Avanzate" |
| Voce della tassonomia | voce, categoria, etichetta | **Categoria** |
| Catalogo con regole | Smart | **Smart** (come in Apple Foto e Lightroom in italiano), con la descrizione "aggiornato dalle regole" |

---

## 7. Software di riferimento: cosa prendere e cosa evitare

- **Lightroom Classic**:
  - *da prendere*: pannelli richiudibili, Tab per nasconderli, scorciatoie di modulo, visualizzatore e griglia con le frecce, raccolte smart;
  - *da evitare*: troppi moduli e pannelli; è il difetto che le recensioni citano di più.
- **Apple Foto (Sequoia)**: barra laterale a gruppi e raccolte automatiche. Il ridisegno ha diviso gli utenti, quindi conviene cambiare la struttura una volta sola.
- **Google Foto**: la barra delle azioni compare con la selezione. Tre ridisegni della navigazione in tre anni hanno fatto protestare gli utenti. Lezione: una sola riorganizzazione, poi stabilità.
- **digiKam**:
  - *da prendere*: lo stato "da confermare" per i volti;
  - *da evitare*: stati confermato e non confermato poco distinguibili (bug 404167).
- **Immich**: digitare un nome che esiste già propone l'unione; si possono nascondere le persone; le code di lavoro sono separate dai contenuti.
- **PhotoPrism**: una coda "Revisione"; si possono disattivare le sezioni non usate.
- **Mylio**: calendario anno → mese → giorno (già adottato con `date-filter-hierarchy`) e QuickReview.
- **Photo Mechanic**: velocità e uso da tastiera, a prezzo di un'interfaccia densa. Da noi la densità resta, ma con divulgazione progressiva.
- **Linear / Raycast**: palette Ctrl+K, scorciatoie in chip monospazio, interfaccia sobria.

---

## 8. Principi di progetto adottati

1. **Il contenuto prima dei controlli**: la griglia occupa lo spazio; i controlli compaiono quando servono (selezione, hover, menu).
2. **Una cosa, un nome, un posto**: glossario unico; ogni impostazione esiste in un solo punto, ma la si può raggiungere da più punti.
3. **Annullare invece di confermare**: si conferma solo ciò che tocca i file o non si può annullare.
4. **Divulgazione progressiva**: le opzioni avanzate stanno dietro "Avanzate"; i valori tecnici restano disponibili ma non in primo piano.
5. **Tastiera di prima classe**: ogni azione frequente ha una scorciatoia visibile nei menu e nella palette.
6. **Sistema visibile ma discreto**: un indicatore delle attività sempre presente; i dettagli tecnici solo a richiesta.
7. **Accessibile per costruzione**: i componenti condivisi garantiscono focus, etichette, bersagli e contrasto.
8. **Stabilità**: una sola riorganizzazione della navigazione, dopo che tutte le funzioni previste sono integrate.

---

## Fonti

**Software di riferimento**
- Lightroom Classic, area di lavoro: https://helpx.adobe.com/lightroom-classic/help/workspace-basics.html
- Lightroom Classic, volti: https://helpx.adobe.com/lightroom-classic/desktop/organize-photos-in-lightroom-classic/face-recognition.html
- Lightroom, People view: https://helpx.adobe.com/uk/lightroom/desktop/organize-photos/people-view.html
- digiKam:
  - People view: https://docs.digikam.org/en/main_window/people_view.html
  - bug 404167: https://bugs.kde.org/show_bug.cgi?id=404167
- Apple Foto (Sequoia): https://tidbits.com/?p=70111 · https://9to5mac.com/2024/07/05/ios-18-photos-app-redesign-interview/
- Google Foto, Collections: https://9to5google.com/2024/08/08/google-photos-library-collections-redesign/ · https://www.androidauthority.com/google-photos-library-tab-redesign-3435078
- Immich, volti: https://docs.immich.app/features/facial-recognition/
- PhotoPrism, impostazioni generali: https://docs.photoprism.app/user-guide/settings/general
- Mylio, People view: https://manual.mylio.com/24.3/en/topic/browse-the-people-view
- Photo Mechanic: https://shotkit.com/photo-mechanic-review/
- Excire Foto: https://petapixel.com/2023/01/30/excire-foto-review-supercharged-ai-driven-digital-asset-management

**Linee guida e norme**
- NN/g:
  - 10 euristiche: https://www.nngroup.com/articles/ten-usability-heuristics/
  - tempi di risposta: https://www.nngroup.com/articles/response-times-3-important-limits/
  - indicatori di avanzamento: https://www.nngroup.com/articles/progress-indicators/
  - skeleton screen: https://www.nngroup.com/articles/skeleton-screens/
  - stati vuoti: https://www.nngroup.com/articles/empty-state-interface-design/
  - tutorial: https://www.nngroup.com/articles/mobile-tutorials/
  - dialoghi di conferma: https://www.nngroup.com/articles/confirmation-dialog/
  - acceleratori: https://www.nngroup.com/articles/ui-accelerators/
  - divulgazione progressiva: https://www.nngroup.com/articles/progressive-disclosure/
- Fluent NavigationView: https://learn.microsoft.com/windows/apps/develop/ui/controls/navigationview
- Apple HIG:
  - Sidebars: https://developer.apple.com/design/human-interface-guidelines/sidebars
  - Toolbars: https://developer.apple.com/design/human-interface-guidelines/toolbars
  - Onboarding: https://developer.apple.com/design/human-interface-guidelines/onboarding (pagina non verificata)
- Laws of UX: https://lawsofux.com/hicks-law/ · https://lawsofux.com/fittss-law/ · https://lawsofux.com/jakobs-law/ · https://lawsofux.com/millers-law/
- WCAG:
  - novità di WCAG 2.2: https://www.w3.org/WAI/standards-guidelines/wcag/new-in-22/
  - 2.5.8 (dimensione dei bersagli): https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
  - WCAG2ICT: https://www.w3.org/TR/wcag2ict-22/
- Linear:
  - UI refresh di marzo 2026: https://linear.app/changelog/2026-03-12-ui-refresh
  - ridisegno: https://linear.app/blog/how-we-redesigned-the-linear-ui
- Raycast: https://raycast.com/blog/a-fresh-look-and-feel
