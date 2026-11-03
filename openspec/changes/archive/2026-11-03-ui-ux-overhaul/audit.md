# Audit UI/UX di photo-cat

Data: 2026-11-03.

**Base analizzata**:
- `main` f40beb5, che contiene già `preferences-survive-reinstall`, `large-library-stability`, `similar-photo-stacks` e `catalog-exclusions`;
- le change ancora sui rispettivi branch: `loading-feedback` e `people-faces-workspace`, con il suo mockup `People.html`.

**Aggiornamento dopo il rebase** (stessa data): il branch `feat/ui-ux-overhaul` ora parte dalla release 0.3.0, che integra anche `loading-feedback` e `people-faces-workspace`. Tutti i riferimenti `file:riga` sono stati ricontrollati su questo codice. I punti già risolti restano nell'elenco, segnati come **Risolto** o **In parte risolto**. I punti nuovi sono segnati con *(nuovo)*.

**Verifica finale 8.5** (stessa data, su `feat/ui-ux-overhaul` dopo le fasi 7–8): ogni voce **A** e **M** è stata ricontrollata sul codice corrente in `app/ui/src` e marcata **Risolto**, **In parte risolto** (con ciò che resta) o **Motivato** (fuori ambito / differito). Sintesi in §9. Le voci **B** non sono oggetto di questa verifica.

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

*(Nota storica dell’audit iniziale.)* Dopo le fasi 7–8 lo stato A/M aggiornato è in **§9**; scorciatoie, palette, undo e componenti condivisi sono stati generalizzati oltre Persone.

---

## 2. Shell e navigazione

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| S1 | 11 voci piatte… — **Risolto**: navigazione a gruppi (`browse` / `organize` / `transfer` / `bottom`); Modelli e Diagnostica non sono più voci di primo livello (stanno in Impostazioni). | `modules/registry.ts:22-55`, `ModuleNav.tsx:134-170` | A |
| S2 | La navigazione è sempre larga 200 px… — **Risolto**: modalità compatta (`navCompact`, Ctrl+B) e scorciatoie di modulo Ctrl+1…7. | `ModuleNav.tsx:37-61,79-156`, `registry.ts:57-71` | M |
| S3 | La ricerca globale filtra solo la Libreria… — **Risolto**: il campo apre la palette dei comandi (`openPalette`), non un filtro Libreria. | `TopBar.tsx:44-65,210`, `AppShell` + `CommandPalette` | M |
| S4 | Barra superiore: il pallino dei job dice "Nessun job attivo" e gira sempre — **Risolto da `loading-feedback`**: senza job mostra un'icona ferma (`TopBar.tsx:81`). Resta il chip RAM, che porta a Job: è un'informazione tecnica sempre in vista. | `TopBar.tsx:92-121` | B |
| S5 | La barra di stato mostra contenuti diversi a seconda del modulo; le scorciatoie di Sequenza ci finiscono come testo lungo. | `ReadyApp.tsx:111-125`, `SequencePage` | B |
| S6 | Titolo Importa / Esporta misto… — **Risolto**: titolo Importa = "Aggiungi una cartella"; Esporta è un modulo unico con sezioni (dati / cartelle / sequenza) per design D3. | `i18n/it.ts:161`, `OrganizePage.tsx` | M |
| S7 | Impostazioni duplicate… — **Risolto**: Modelli e Diagnostica sono sezioni di Impostazioni; un solo posto per provider, profilo e livello di log. | `layout.ts:10-26,115-126`, `SettingsPage.tsx:24-31,97-107` | A |
| S8 | Nessun Aiuto / scorciatoie / palette… — **Risolto**: `HelpMenu`, `CommandPalette`, `ShortcutSheet` (anche `?`); scorciatoie Persone elencate nel foglio. | `TopBar.tsx:104-222`, `AppShell.tsx:115-116` | A |
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
| L1 | Fino a 5 fasce sopra la griglia… — **Risolto**: stack = `SelectionBar` *oppure* `LibraryHeader`, poi griglia; pannelli flottanti a parte. | `LibraryPage.tsx:22-33` | A |
| L2 | Toolbar mescola sort/size/simili… — **Risolto**: header = Seleziona / Ordina / Vista / inspector; "Raggruppa simili" nel menu Vista; "Solo foto con simili" tra i filtri laterali (+ chip). | `LibraryHeader.tsx:200-268`, `Sidebar.tsx:353-360` | M |
| L3 | Bulk bar sempre visibile + flusso azione/valore/Applica… — **Motivato**: `SelectionBar` solo con selezione > 0; azioni dirette. Residuo accettato: Esporta / Organizza aprono `organize` senza etichetta extra (i nomi delle voci bastano; toast dedicato fuori change). | `LibraryPage.tsx:26`, `SelectionBar.tsx:263-371` | A |
| L4 | Nessun visualizzatore interno… — **Risolto**: `Viewer` condiviso; Spazio/Invio; doppio clic rispetta la preferenza viewer/esterno. | `Grid.tsx:346,517-527,719-811` | A |
| L5 | Griglia non navigabile con frecce… — **Risolto**: frecce / Page / Home / End; Canc resta contestuale al catalogo (esclusione). | `Grid.tsx:479-496,588-605` | M |
| L6 | Sidebar senza sezioni, icone testuali… — **Risolto**: sezioni richiudibili, icone SVG, aggiorna cartella nel menu, bersagli ≥ 24 px. | `Sidebar.tsx:142-209,252-344` | M |
| L7 | Inspector con modulo generico "Modifica"… — **Risolto**: sezioni `Disclosure`; modifica sul campo (`InlineEdit` / tag / categorie); niente form generico in testa. | `Inspector.tsx:56-65,288-425` | M |
| L8 | L'intestazione del catalogo è testo, conteggio e un pulsante "Escluse (N)", senza gerarchia visiva. La vista delle escluse si raggiunge solo da qui (Cataloghi non mostra il conteggio). | `CatalogHeader.tsx:32-37`, `CatalogsPage.tsx:99` | B |
| L9 | Toast esclusione senza chiusura / non impilati… — **Risolto**: `ToastHost` con chiusura, pausa su hover/focus, errori `role="alert"`; esclusioni e undo Persone passano da `notify` / `notifyUndoable` / `notifyPeopleUndo`. | `ToastHost.tsx:11-66`, `catalogActions.tsx:51-65`, `peopleUndo.ts` | M |
| L10 | *(nuovo)* Badge simili annidato nel button… — **Risolto**: cella `role="option"` e badge `button.thumb-group` come sibling, non annidati. | `Grid.tsx:113-280`, `library.css:144-163` | M |
| L11 | *(nuovo)* Aprire un gruppo di simili o selezionarlo carica i membri senza alcun indicatore; se la richiesta fallisce non succede nulla e l'errore non si vede. | `Grid.tsx:399,411` | B |
| L12 | *(nuovo)* "Foto simili" Inspector: sciogli senza conferma, no BusyButton, sezione sparisce… — **Motivato**: accettato (inventario undo 1.5: si correggono a mano); busy/errori della sezione → change successiva. | `SimilarSection.tsx:29,85-106` | M |
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
- **A**: focus/Esc della modale — **Risolto**: `Dialog` / `ConfirmDialog` Radix; Esc chiude subito (`import/manage.tsx`, `Dialog.tsx`).
- **M**: "Avvia importazione" → Job — **Risolto**: resta su Importa; toast con azione opzionale verso Attività (`ImportPage.tsx:251-253`).
- **M**: pattern glob in vista — **Motivato**: sotto «Avanzate» con hint in italiano; sintassi `**/…` nel campo accettabile (7.1).
- **M**: fino a 5 pulsanti per riga — **Risolto**: «Aggiorna» + menu ⋯ (`ImportPage.tsx:55-124`).
- **B**: la colonna destra resta vuota quando non c'è un import in corso; anche l'elenco delle sorgenti sparisce del tutto quando è vuoto (`ImportPage.tsx:30`).

### Cataloghi
- **A**: RuleEditor con codici grezzi — **Risolto**: etichette tradotte; valori = indici (`RuleEditor.tsx:225-247`).
- **M**: `window.confirm` + pulsante non danger — **Motivato**: `ConfirmDialog` (confirm danger) ok; stile danger del trigger sulla card differito. Nessun `window.confirm` nel progetto.
- **M**: editor senza titolo — **Risolto**: `Card` con titolo nuovo/modifica (`CatalogsPage.tsx:200`).
- **B**: conteggio senza unità ("Smart · 123"), nessuno stato vuoto (`QueryView` con `isEmpty={() => false}`), il numero di foto escluse non è mostrato (`CatalogsPage.tsx:92,99`).
- **Riferimento**: Smart Album di Apple e Capture One, raccolte smart di Lightroom. Le regole si leggono come frasi ("Anno è 2023 e tag contiene Mare"), con il conteggio aggiornato mentre si scrive.

### Tag
- **A**: nota "Annulla" senza Undo — **Risolto**: nota allineata; Ctrl+Z / Annulla operazione via store (`TagsPage.tsx:42-55,478-486,654`).
- **M**: conferma Elimina set lontana — **Risolto**: trigger danger + `ConfirmDialog` (`TagsPage.tsx:293-295,759-766`).
- **M**: errori hotkey ignorati — **Risolto**: `notifyError` via `bulk.start` (`bulkJob.ts:86-98`).
- **B**: "Rinomina" compare due volte (`TagsPage.tsx:451,581`); colori fissi in `TagChip` (`TagChip.tsx:28`).

### Persone (integrato con `people-faces-workspace`)
Punti dell'audit precedente:
- **A**:
  - l'unione avviene senza conferma e senza annullamento — **Risolto**: `MergeDialog` + Ctrl+Z; dopo l'unione `notifyPeopleUndo` mostra toast con Annulla (`MergeDialog.tsx:84-101`, `peopleUndo.ts`).
  - lo stato vuoto rimanda a una "rianalisi da Modelli AI" che non esiste — **Risolto**: i testi vuoti ora spiegano cosa fare (`i18n/features.ts:131-133`).
- **M**:
  - id interni in vista ("Persona senza nome 12", `#1234`) — **Risolto**: i gruppi senza nome si chiamano "Gruppo di N volti" (`GroupList.tsx:95-98`);
  - "Apri in Persone" non apre la persona — **Risolto**: `OpenInPeople` apre la persona o il volto indicato (`people/OpenInPeople.tsx:21`).

Problemi trovati nel modulo integrato *(nuovi)*:
- **A**: "Sposta in…" al change della select — **Risolto**: la select aggiorna solo lo stato; lo spostamento avviene sul pulsante «Sposta»; undo via toast (`FaceGrid.tsx:374-410`).
- **M**:
  - scorciatoie a lettera singola (N, M, X, U, I) — **Motivato**: elencate in Aiuto; senza modificatore/spegnimento → change a11y; non sparano dietro i `Dialog` (`modalOpen()` in `keys.ts`).
  - I (ignora) e Ctrl+Z senza riscontro — **Risolto**: toast «Gruppo ignorato» / undo; errori → `notifyError` (`PeopleModule.tsx:254-282`, `peopleUndo.ts`).
  - due modi diversi di unire — **Motivato**: due flussi intenzionali (gruppi vs identità); allineamento UX completo differito.
  - numeri 0–1 senza spiegazione — **Motivato**: scala % già in Impostazioni › Sensibilità; decimali di debug in Persone → passata dedicata.
  - "Rimuovi volto" con `window.confirm` — **Risolto**: `ConfirmDialog` (`FaceGrid.tsx:544-551`).
  - `MergeDialog` senza focus trap — **Risolto**: usa `Dialog` Radix (`MergeDialog.tsx:97-100`).
- **B**:
  - ruoli ARIA sbagliati: `role="row"` su `<button>` nella tabella delle persone, `role="option"` senza `listbox` e con dentro il pulsante "Ripristina", `role="grid"` senza righe né celle, `aria-selected` su `<button>` (`GroupList.tsx:475,487,602,659`, `FaceGrid.tsx:573`);
  - il pulsante principale del suggerimento e di `MergeDialog` usa la classe `btn primary` invece di `btn-primary`, quindi non ha lo stile principale (`FaceGrid.tsx:487`, `MergeDialog.tsx:205`);
  - il motivo dello scarto di un volto è solo nel `title`, invisibile da tastiera (`FaceGrid.tsx:684`);
  - un secondo `<main>` dentro quello della shell (`PeopleModule.tsx:442`); titolo di pagina a 18 px, un'altra dimensione ancora (`FaceGrid.tsx:328`);
  - il caricamento dell'elenco è un testo "Caricamento…" invece dello `Skeleton` usato altrove (`GroupList.tsx:468,566`);
  - la scheda mostra il percorso del file delle identità in monospazio (`PersonCard.tsx:229`).

### Sequenza
- **A**: pulsante annidato (badge simili) — **Risolto**: toggle gruppo sibling fuori dal `BusyButton` (`SequencePage.tsx:400-415`).
- **M**:
  - pagina senza titolo — **Risolto**: `ModulePage title={…}` (`SequencePage.tsx:910`).
  - date ISO grezze — **Risolto**: `fmtDate(localDate(…))` (`SequencePage.tsx:532-535`).
  - "Pausa tra sessioni" senza riscontro — **Risolto**: `gapState` + `role="status"/"alert"` (`SequencePage.tsx:145-163,247-264`).
  - gergo nell'export — **Risolto**: etichette in prosa (dettaglio colori, verify senza SSIM, hardlink spiegato) (`i18n/modules.ts:254-282`, `SequenceExport.tsx`).
- **B**: durante il caricamento non si vede nessun indicatore e la griglia resta vuota; `loading-feedback` non ha toccato questa pagina (`SequencePage.tsx:869-878,984`).

### Esporta e organizza
- **A**:
  - ambito predefinito "Selezione" a 0 — **Risolto**: default `selection` solo se c'è selezione, altrimenti filtro; alert esplicativo (`OrganizePage.tsx:98-100,147-152,276-278`).
  - "Crea cartelle" richiede anteprima+verifica senza dirlo — **Risolto**: passi, badge `createWaiting`, `disabledReason` (`OrganizePage.tsx:202-229,435-503`).
- **M**:
  - pulsanti "↑" / "×" senza etichetta — **Risolto**: testo + `aria-label` (`OrganizePage.tsx:394-419`).
  - storico grezzo — **Risolto**: `historyItem` / `historyMeta` (`OrganizePage.tsx:558-577`).
  - "Annulla" = torna indietro — **Risolto**: undo = «Elimina le cartelle create…» (`OrganizePage.tsx:579-586`).
- **Motivato** (fuori ambito, `design.md` punto 5): «Sposta file» non è offerto; il backend espone solo `copy` / `hardlink` / `symlink` (`OrganizePage.tsx:45`).

### Job
- **A**: tipo job come id inglese — **Risolto**: `jobTitle` / `jobs.kinds.*` (`lib/jobs.ts:66-78`, `JobsPage.tsx:126,149-151`).
- **M**:
  - "Annulla" senza conferma — **Motivato**: etichetta «Interrompi»; conferma solo sui job che scrivono file (2.3). Errori gestiti (`JobsPage.tsx:182-198`).
  - storico senza titolo — **Risolto**: `<h2 id="jobs-history">` (`JobsPage.tsx:280-283`).
- **B**: lo stato vuoto non ha un pulsante per andare a Importa (`JobsPage.tsx:392`).

### Modelli AI
- **M**:
  - impostazioni duplicate (vedi S7) — **Risolto**: unico posto in Impostazioni › Modelli / Diagnostica.
  - colonna SHA-256 — **Motivato**: utile in verifica; nascosta dietro divulgazione progressiva → change successiva.
  - "Verifica tutti" come azione principale — **Motivato**: priorità Scarica vs Verifica differita (cosmetica).
- **Gergo**: Runtime AI, ONNX, Provider, Self-test, Budget.

### Diagnostica
- **A**: righe solo mouse — **Risolto**: `DataTable` con `tabIndex` e frecce (`DataTable.tsx:64-119`, `DiagnosticsPage.tsx:352-363`).
- **M**:
  - errori come notizie neutre — **Risolto**: badge `danger`/`warn` (`DiagnosticsPage.tsx:19,191`, `ErrorGroups.tsx:102-105`).
  - fase `modelli` non tradotta — **Risolto**: `jobs.phases.modelli` (`i18n/it.ts:129`).
  - livello di log duplicato con valori grezzi — **Risolto**: un solo controllo; label `diag.logLevels.*` (`DiagnosticsPage.tsx:266-283`).

### Impostazioni
- **A**:
  - "Salva" a metà pagina — **Risolto**: autosave + «Salvato» sul campo (`settingsState.tsx:1-6,133-148`).
  - "Elimina tutti i dati dei volti" stile normale — **Risolto**: `variant="danger"` + `ConfirmDialog` (`AdvancedSection.tsx:183-192`).
- **M**: tre modi di salvare / blocco misto — **Risolto**: sezioni + autosave vs `ApplyBar` (`SettingsPage.tsx:24-32,214-225`).
- **Linguaggio**: "embedding", valori 0–1 con nomi diversi per lo stesso concetto (vedi §6).

### Onboarding
- **M**: passo sistema con numeri senza etichette / non saltabile — **Risolto**: `SystemSummary` in prosa; «Lo faccio dopo» (`Onboarding.tsx:209-227`).
- **M**: passo modelli "3 / 7" bloccante — **Risolto**: niente contatore 3/7; skip con later (`Onboarding.tsx:209-246`).
- **M**: percorso in campo di testo — **Motivato**: `FolderPicker` + campo editabile intenzionale per chi preferisce digitare.
- **M**: fine → Job — **Risolto**: apre Libreria (`Onboarding.tsx:66-69`).
- **M**: non si può rivedere — **Risolto**: Aiuto › «Rivedi l'introduzione» (`TopBar.tsx:129-133`).

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

## 9. Verifica finale 8.5

Confronto A/M di §2–§4 con il codice corrente su `feat/ui-ux-overhaul` (fasi 7–8). Le voci **B** non sono conteggiate.

| Priorità | Totale | Risolto | In parte risolto | Motivato | Aperto |
|---|---:|---:|---:|---:|---:|
| **A** | 19 | 18 | 0 | 1 | 0 |
| **M** | 46 + 1* | 36 | 0 | 10 + 1* | 0 |

\* **Motivato** aggiuntivo (non era una riga A/M dell’audit originale): «Sposta file» in Esporta › Organizza — fuori ambito; backend solo `copy` / `hardlink` / `symlink` (`design.md`).

### Motivato (A)

| # | Motivo |
|---|---|
| **L3** (residuo) | Esporta / Organizza dalla `SelectionBar` cambiano modulo senza etichetta extra: accettato — le voci del menu usano già i nomi dei moduli di destinazione; un toast/indicatore dedicato è fuori da questa change. |

### Motivato (M)

| # | Motivo |
|---|---|
| **L12** | Sciogli/rimuovi simili senza conferma/busy: accettato per ora (si correggono a mano, inventario undo 1.5); busy/errori della sezione → change successiva. |
| **Persone · due UX unione** | Due flussi intenzionali (gruppi vs identità); allineamento UX completo differito. |
| **Persone · punteggi 0–1** | Scala % già in Impostazioni › Sensibilità (6.3); i decimali di debug in Persone restano fino a una passata dedicata. |
| **Modelli · SHA-256** | Utile in Diagnostica/verifica; nascosto dietro divulgazione progressiva → change successiva. |
| **Modelli · Verifica tutti** | Priorità del bottone Scarica vs Verifica: rifinitura cosmetica differita. |
| **Importa · glob** | Pattern sotto Avanzate con esempi in parole: residuo accettabile (7.1). |
| **Cataloghi · Elimina** | `ConfirmDialog` ok; stile danger del trigger sulla card differito. |
| **Persone · hotkey 1 lettera** | Elencate in Aiuto; spegnimento/modificatore → change a11y dedicata. |
| **Job · Interrompi** | Conferma solo sui job che scrivono file: conforme al design (2.3). |
| **Onboarding · cartella** | `FolderPicker` + campo editabile per percorso: intenzionale per chi preferisce digitare. |

### Controlli handoff (verificati)
- Moduli fase 7 allineati (Importa, Cataloghi, Tag, Persone, Sequenza, Esporta, Diagnostica, pannelli).
- «Sposta in…» solo sul pulsante; undo ok.
- Scorciatoie non sparano dietro i `Dialog` condivisi.
- `window.confirm` assente; `PENDING` glossario / `A11Y_KNOWN` svuotati.

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
