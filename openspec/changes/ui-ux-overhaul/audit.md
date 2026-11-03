# Audit UI/UX di photo-cat

Data: 2026-11-03.

**Base analizzata**:
- `main` f40beb5, che contiene già `preferences-survive-reinstall`, `large-library-stability`, `similar-photo-stacks` e `catalog-exclusions`;
- le change ancora sui rispettivi branch: `loading-feedback` e `people-faces-workspace`, con il suo mockup `People.html`.

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

Le change in sospeso migliorano parti importanti:
- `loading-feedback` porta stati di caricamento, toast e pulsanti "in corso";
- `people-faces-workspace` porta un modulo Persone unico e da tastiera.

Restano però locali. Questa proposta le generalizza.

---

## 2. Shell e navigazione

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| S1 | 12 voci piatte: Libreria, Sequenza, Cataloghi, Tag, Persone, Identità, Importa, Job, Modelli AI, Diagnostica, Esporta e organizza, Impostazioni. Diagnostica e Modelli stanno in mezzo ai contenuti; "Esporta e organizza" sta dopo Diagnostica. Con `people-faces-workspace` le voci diventano 11. | `modules/registry.ts:8` | A |
| S2 | La navigazione è sempre larga 200 px e non si richiude; non ci sono scorciatoie per cambiare modulo. | `ModuleNav.tsx:13` | M |
| S3 | La ricerca globale filtra solo la Libreria e ci porta l'utente anche se lui stava altrove. Non cerca persone, cataloghi, tag o comandi come elementi. | `TopBar.tsx:12` | M |
| S4 | Barra superiore: il pallino dei job dice "Nessun job attivo" e gira sempre (risolto da `loading-feedback`). Il chip RAM porta a Job: è un'informazione tecnica sempre in vista. | `TopBar.tsx:44,70` | B |
| S5 | La barra di stato mostra contenuti diversi a seconda del modulo; le scorciatoie di Sequenza ci finiscono come testo lungo. | `App.tsx`, `SequencePage` | B |
| S6 | Il modulo chiamato "Importa" ha come titolo "Aggiungi una sorgente"; "Esporta e organizza" contiene due funzioni senza rapporto con Sequenza › Esporta. | `ImportPage`, `OrganizePage` | M |
| S7 | Le impostazioni sono duplicate: provider e profilo stanno in Modelli (salvataggio immediato) e in Impostazioni (con "Salva"); il livello di log sta in Diagnostica (immediato) e in Impostazioni (con "Salva"); le etichette sono diverse ("DirectML (GPU)" contro `DIRECT_ML`). | `ModelsPage`, `DiagnosticsPage`, `SettingsPage:146` | A |
| S8 | Non c'è un menu Aiuto, né un elenco delle scorciatoie, né una palette dei comandi. | — | A |

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
| L1 | Fino a 5 fasce sopra la griglia (CatalogHeader, CatalogFeedback, Toolbar, BulkBar, griglia), più i pannelli flottanti. La griglia, cioè il contenuto, perde spazio verticale. | `LibraryPage.tsx:100` | A |
| L2 | La barra degli strumenti mescola cose diverse: ordinamento (select), dimensione (slider senza valori), "Raggruppa foto simili" (preferenza di vista) e "Solo foto con simili", che è un filtro ma non sta tra i filtri e non genera un chip. | `LibraryPage.tsx:19` | M |
| L3 | La barra bulk è sempre visibile, anche con 0 selezionati. Il flusso è: scegliere un'azione da 12 voci, poi un valore, poi "Applica". Due azioni ("Esporta", "Organizza") cambiano modulo senza dirlo. "Seleziona tutto (filtro)" usa un linguaggio da sviluppatore. | `BulkBar.tsx` | A |
| L4 | Non c'è un visualizzatore interno: il doppio clic apre un'app esterna. Sequenza invece ha uno schermo intero (Spazio). Due comportamenti diversi per lo stesso gesto. | `Grid.tsx:382`, `SequencePage:784` | A |
| L5 | La griglia non si naviga con le frecce; Canc ed Invio funzionano solo dentro un catalogo. | `Grid.tsx:497` | M |
| L6 | Barra laterale: sorgenti (percorsi mono troncati alle ultime 2 cartelle), cataloghi e filtri in un'unica colonna, senza sezioni richiudibili. Icone testuali "▸ ▾ ↻ ✎" con bersagli di circa 20 px. Un "↻ aggiorna cartella" accanto a ogni sottocartella: è un'azione distruttiva per il tempo e sta accanto al filtro. | `Sidebar.tsx` | M |
| L7 | Inspector: si apre con un modulo generico "Modifica" (select del gruppo + valore + tag + catalogo) prima delle informazioni. La modifica non è sul campo. Le sezioni non si possono richiudere. | `Inspector.tsx:86` | M |
| L8 | L'intestazione del catalogo è testo, conteggio e un pulsante "Escluse (N)", senza gerarchia visiva. La vista delle escluse si raggiunge solo da qui (Cataloghi non mostra il conteggio). | `CatalogHeader.tsx` | B |
| L9 | Il toast di esclusione è l'unico uso di `Toast`: non ha pulsante di chiusura e non si impila. | `components/feedback/Toast.tsx` | M |

**Riferimenti**:
- **Google Foto** e **Apple Foto**: quando si seleziona qualcosa, la barra superiore diventa la barra delle azioni sulla selezione.
- **Lightroom**: visualizzatore (E/Loupe), griglia (G), frecce e Spazio.
- **Photo Mechanic**: valutazione e tag da tastiera, con avanzamento automatico.
- **Mylio QuickReview**: revisione rapida senza interfaccia intorno.
- **Apple HIG Toolbars**: poche azioni nella barra, il resto in un menu di overflow, e l'interruttore dell'inspector in fondo.

---

## 4. Altri moduli

### Importa
- **A**: la finestra modale non sposta il focus e Esc non funziona finché l'utente non ci entra con Tab (`import/manage.tsx:43`).
- **M**: "Avvia importazione" porta subito a Job (`ImportPage.tsx:221`) e l'utente lascia la pagina senza una conferma.
- **M**: i pattern glob sono in vista (`**/.thumbnails/**`).
- **M**: fino a 5 pulsanti per riga di sorgente.
- **B**: la colonna destra resta vuota quando non c'è un import in corso.

### Cataloghi
- **A**: l'editor delle regole mostra codici grezzi (`tag, person, people_count, width…`, `eq, neq, gte…`; `RuleEditor.tsx:101,111`).
- **M**: `window.confirm` per l'eliminazione, e il pulsante non è segnalato come pericoloso.
- **M**: l'editor compare sopra la griglia senza titolo.
- **B**: conteggio senza unità ("Smart · 123"), nessuno stato vuoto, il numero di foto escluse non è mostrato.
- **Riferimento**: Smart Album di Apple e Capture One, raccolte smart di Lightroom. Le regole si leggono come frasi ("Anno è 2023 e tag contiene Mare"), con il conteggio aggiornato mentre si scrive.

### Tag
- **A**: la nota dice "si possono annullare con Annulla", ma nella pagina non c'è nessun Annulla (`TagsPage.tsx:674`).
- **M**: la conferma di "Elimina set" compare nel pannello di destra, lontano dal pulsante.
- **M**: gli errori dei tasti rapidi vengono ignorati senza avviso (`useTagHotkeys.ts:76`).
- **B**: "Rinomina" compare due volte; colori fissi in `TagChip`.

### Persone / Identità (in attesa di `people-faces-workspace`)
- **A**:
  - l'unione avviene senza conferma e senza annullamento;
  - lo stato vuoto rimanda a una "rianalisi da Modelli AI" che non esiste.
- **M**:
  - id interni in vista ("Persona senza nome 12", `#1234`);
  - "Apri in Persone" non apre la persona.

Il nuovo modulo risolve gran parte di questi punti. Questa change lo allinea solo ai componenti e alle regole comuni, senza riprogettarlo.

### Sequenza
- **A**: un pulsante annidato in un altro (badge dei simili, `SequencePage.tsx:189/245`).
- **M**:
  - la pagina non ha titolo;
  - date ISO grezze nelle intestazioni di sessione;
  - "Pausa tra sessioni" si salva all'uscita dal campo senza alcun riscontro;
  - gergo nell'export ("Crominanza 4:4:4", "SSIM ≥ 0.985", "Hard link").
- **B**: durante il caricamento si vede solo "…".

### Esporta e organizza
- **A**:
  - l'ambito predefinito è "Selezione", che vale 0 se non è selezionato nulla, quindi tutto è disabilitato senza spiegazione;
  - "Crea cartelle" richiede di aver lanciato sia l'anteprima sia la verifica, e non lo dice (`OrganizePage.tsx:31,77,354`).
- **M**:
  - pulsanti "↑" e "×" senza etichetta;
  - lo storico è in forma grezza ("#12 · D:\out · 340");
  - "Annulla" significa "torna indietro".

### Job
- **A**: il tipo di job compare come identificatore inglese (`#12 · import`; `JobsPage.tsx:124,242`).
- **M**:
  - "Annulla" ferma un job senza conferma e senza gestire l'errore;
  - lo storico non ha titolo.
- **B**: lo stato vuoto non ha un pulsante per andare a Importa.

### Modelli AI
- **M**:
  - impostazioni duplicate (vedi S7);
  - una colonna SHA-256;
  - "Verifica tutti" è l'azione principale, mentre l'utente comune vuole "Scarica".
- **Gergo**: Runtime AI, ONNX, Provider, Self-test, Budget.

### Diagnostica
- **A**: le righe della tabella si selezionano solo con il mouse (`<tr onClick>` senza tabIndex, `DiagnosticsPage.tsx:383`).
- **M**:
  - gli errori appaiono come notizie neutre;
  - la fase `modelli` non è tradotta;
  - il livello di log è duplicato.

### Impostazioni
- **A**:
  - "Salva" sta a metà pagina, mentre gli avvisi compaiono in cima, fuori schermo;
  - "Elimina tutti i dati dei volti" ha lo stile di un pulsante normale.
- **M**: tre modi di salvare diversi (Salva, Applica soglie, Salva tassonomia) e un unico blocco senza titoli che mescola risorse, AI, aspetto e percorsi.
- **Linguaggio**: "embedding", valori 0–1 con nomi diversi per lo stesso concetto (vedi §6).

### Onboarding
- **M**: il passo "sistema" mostra numeri senza etichette ("16 GB RAM · 8 CPU · 4000 MB") e non si può saltare.
- **M**: il passo "modelli" mostra "3 / 7" e blocca finché l'utente non preme.
- **M**: il percorso si scrive in un campo di testo.
- **M**: alla fine porta a Job invece che alle foto.
- **M**: non si può rivedere.

### Pannelli flottanti
- **B**: spostamento e ridimensionamento solo con il mouse (WCAG 2.5.7 chiede un'alternativa al trascinamento).
- **B**: `z-index` in conflitto con Toast e Modal.

---

## 5. Problemi trasversali

| Tema | Stato attuale | Norma di riferimento |
|---|---|---|
| Dialoghi | Nessun componente condiviso; 5 implementazioni, nessuna intrappola il focus. | WCAG 2.4.3, 2.4.11; APG Dialog |
| Conferme | 7 `window.confirm`, 3 schede "alertdialog" in linea, 1 modale, 1 testo che fa da conferma. Alcune azioni annullabili chiedono conferma (rimuovere dalla libreria), mentre altre non annullabili non la chiedono (unire persone, fermare un job). | NN/g "Confirmation dialogs": conferma solo per le azioni gravi e irreversibili, altrimenti annullamento |
| Annullamento | Esiste per bulk ed esclusioni, ma è legato alla barra bulk o a un toast; non c'è un "Annulla" globale. | Nielsen #3 (controllo e libertà) |
| Messaggi | Circa 25 `<p role="alert">` con colori diversi; errori mostrati come `detail` grezzo del backend. | NN/g: validazione vicino al campo, notifiche per gli eventi di sistema |
| Stati vuoti | Testi senza azione; in Cataloghi, sorgenti di Importa, storico di Organizza e Modelli mancano del tutto. | NN/g Empty states: stato, spiegazione, azione diretta |
| Tipografia | Corpi 11, 12, 13, 14, 16, 17, 18, 20, 26 e h1/h2 del browser; il titolo di pagina è assente in Sequenza e senza stile in 4 pagine. | Gerarchia tipografica chiara (requisito "Stile visivo") |
| Spaziatura e forma | Valori fissi (4, 6, 8, 10, 12, 14, 16, 20, 24…); nessun token per spaziatura, raggio ed elevazione. | Fluent 2 / design token |
| Icone | Glifi testuali (▸ ▾ ↻ ✎ × ↑ − ‹) mescolati alle icone SVG. | Coerenza (Nielsen #4) |
| Accessibilità | Bersagli da 20–22 px; pulsanti annidati; righe non raggiungibili da tastiera; colore come unico segnale (barra del dispositivo); `role="row"` su `<button>`. | WCAG 2.2: 2.5.8, 2.1.1, 4.1.2, 1.4.1 |
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
| Valore 0–1 di affidabilità | Soglia di confidenza, Somiglianza minima, Somiglianza tra volti, Soglia del gruppo | **Sensibilità** con scala "Più risultati ↔ Più precisi" e percentuale; il valore numerico resta in "Avanzate" |
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
