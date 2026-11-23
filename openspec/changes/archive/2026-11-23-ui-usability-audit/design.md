# Design

## Context

Motivazioni e osservazioni sono in `proposal.md` e `audit.md`; gli identificativi (P1, S1, D1…) rimandano all'audit. Le schermate di riferimento sono in `mockups/`.

Lo stato del codice rilevante:
- **Persone** (`modules/people/`):
  - `PeopleModule` tiene due selezioni separate: `selectedIds` e `idSel` dell'elenco (`GroupList`) e la selezione dei volti (`FaceGrid`, con `useSelection` e `useActiveGrid`);
  - le correzioni passano da `commands.correctPerson`, `assignGroups`, `mergeIdentitiesMany`;
  - `PersonPicker` esiste già, con le modalità `identities` e `assign`;
  - `MergeDialog` chiede conferma per le unioni tra gruppi;
  - l'annullamento passa da `notifyPeopleUndo` e dal registro del backend.
- **Comandi**: `commands/registry.ts` è l'unica fonte per palette, foglio delle scorciatoie e gestore dei tasti. I moduli registrano i propri comandi mentre sono montati; `useActiveGrid` instrada Ctrl+A, Ctrl+Maiusc+I ed Esc verso la griglia attiva.
- **Shell**:
  - `TopBar` contiene marchio, ricerca, indicatori e `HelpMenu`;
  - `ModulePage` crea l'intestazione della pagina, ma alcuni moduli la sovrascrivono (Cataloghi, Attività) o non la usano (Libreria);
  - `modules/registry.ts` definisce gruppi e scorciatoie Ctrl+1…9 in base all'ordine.
- **Informazioni**: il comando Rust `app_info` restituisce solo versione e percorso dei dati. Le licenze native e dei modelli sono in `assets/licenses/`, già copiate nell'installer. Le licenze dell'interfaccia sono raccolte da `scripts/ui-licenses.mjs`, ma solo per i pacchetti aggiunti da `ui-ux-overhaul`. Quelle delle dipendenze Rust non sono raccolte affatto.
- **Tema**: `theme/tokens.ts` definisce i colori, mentre corpi del testo e spaziature sono sparsi nei CSS dei moduli.

Vincoli:
- nessuna modifica ai dati;
- scorciatoie e ordine dei moduli invariati;
- Windows, Linux e macOS;
- finestra minima 1024×640;
- i test UI esistenti del modulo Persone (circa 2.500 righe) vanno aggiornati, non buttati.

## Goals / Non-Goals

**Goals:**
- Persone con una selezione per riquadro, comandi che compaiono solo quando servono, due verbi e un solo selettore della destinazione.
- Barra dei menu e finestra Informazioni completa, su tutte e tre le piattaforme.
- Coerenza della shell: intestazione, scorrimento, barra di stato, tipografia.
- Correzione dei difetti F1–F9.

**Non-Goals:**
- Algoritmo di raggruppamento, gruppi piccoli e unione automatica: change separata (aggregazione delle identità).
- Modello di selezione comune a tutte le griglie e liste (frecce, Maiusc, Ctrl+clic): change separata (selezione unificata). Qui Persone adotta le stesse regole, ma senza rifare Libreria, Cataloghi e Tag.
- Nuovi moduli o spostamenti nella navigazione. La Mappa resta in Analisi AI; il menu Vista offre solo un accesso diretto.
- Ridisegno di Sequenza, Esporta e Varianti RAW oltre i ritocchi elencati in `tasks.md`.

## Decisions

### D1. Persone: tre colonne, una selezione per riquadro
Le tre colonne restano: elenco, volti e scheda. Cambia ciò che contengono.
- **Intestazione della pagina**: titolo e schede "Da rivedere / Persone / Ignorati" (componente `Tabs` condiviso), su una riga.
- **Elenco**: un campo di ricerca e un pulsante a icona "Ordina e filtra", un menu che raccoglie l'ordinamento, il filtro "solo con poche foto" e, nella scheda Persone, il filtro per cerchia (P3).
- **Selezione nell'elenco** (P4):
  - un clic seleziona solo quella voce e la apre; Ctrl+clic e Maiusc+clic aggiungono o estendono;
  - una casella compare al passaggio del mouse e al focus;
  - con due o più voci selezionate compare in fondo all'elenco una barra "N selezionati · Unisci a… · Ignora · ✕";
  - una sola struttura di stato (`selection` + `anchor`) sostituisce `selectedIds`/`idSel`.
- **Comandi da tastiera**: Ctrl+A, Esc e Modifica › Seleziona tutto agiscono sul riquadro che ha il focus, come oggi fa `useActiveGrid`. Si tolgono i pulsanti "Seleziona tutto / Deseleziona / Inverti" sempre visibili.

*Alternativa scartata*: un clic apre e non seleziona, e la selezione avviene solo con le caselle. Era la prima bozza dell'audit, ma contraddice il modello "come in un gestore di file" chiesto per la change sulla selezione unificata.

### D2. Barra delle azioni con menu "Altro"
Un nuovo componente condiviso, `ActionBar`, in `components/ui`. Riceve azioni ordinate per priorità e misura lo spazio con `ResizeObserver`: le azioni che non entrano passano nel menu "Altro", dalla meno prioritaria. Non va mai a capo e il testo non esce dai pulsanti (requisito "Barre dei comandi"). La usano:
- la barra dei volti selezionati (P1);
- la barra in fondo all'elenco di Persone;
- la `SelectionBar` della Libreria, che oggi fa la stessa cosa a mano.

Sopra i volti, senza selezione, restano solo i comandi di vista: "Ordina" e "Vista" (mostra volti scartati). I comandi di vista sono un componente distinto e stanno a destra.

*Alternativa scartata*: barra che va a capo su due righe. Sposta i volti in basso e cambia altezza a ogni selezione.

### D3. Un selettore per "Sposta in…" e "Unisci a…"
`PersonPicker` diventa l'unico selettore, con una nuova modalità `destination`:
- ricerca;
- persone con nome, poi gruppi senza nome, ciascuno con copertina e numero di foto;
- "Nuova persona «…»".

Spariscono:
- la select "Riassegna a…" con il pulsante "Riassegna" (`FaceGrid`);
- la select "Unisci con un altro gruppo o persona…" con il pulsante (`PersonCard`);
- i verbi "Assegna a…" e "Unisci in…" (P2, P8).

Conferme, secondo le regole di `ui-foundations` (confermare solo ciò che non si annulla):
- spostamenti e unioni tra gruppi sono annullabili: nessuna conferma, avviso con "Annulla operazione";
- l'unione di più identità non è annullabile (requisito "Unione di più identità"): resta l'anteprima con conferma di oggi, nello stesso selettore.

### D4. Suggerimento come domanda
`GroupRow.mergeHint` fornisce già `otherId`, `otherName`, `otherCover` e lo score. La scheda mostra:
- "È {nome}?" con le due copertine e "Sì" / "No". "Sì" esegue la stessa unione di "Sono la stessa persona"; "No" usa la correzione `not_same` di oggi;
- la somiglianza in parole, con soglie fisse sullo score. Oggi la scala è "alta" sopra il doppio della soglia d'unione e "media" sotto; le soglie definitive le fissa la change sull'aggregazione.

Nell'elenco la riga mostra "forse {nome}". Il valore numerico va solo in Diagnostica.

### D5. Ordine di avanzamento (F1, P5)
`GroupList` calcola già `order` nell'ordine mostrato (suggeriti, poi altri gruppi). `onCatalogChange` passerà le righe in quest'ordine, invece di `rows`, così che `advanceAfterLink` e le frecce in `usePeopleHotkeys` seguano l'elenco visibile. Un test copre il caso osservato: suggeriti da 26 e 17 foto, altri gruppi da 41 e 24.

### D6. Barra dei menu
- **Windows e Linux**: componente React `MenuBar` nella `TopBar`, costruito con `@radix-ui/react-menubar` (MIT, stessa famiglia di `react-menu`, già presente). Il componente gestisce frecce, Esc, lettere di accesso e ruoli ARIA. Alt, rilasciato senza altri tasti, e F10 portano il focus sul primo menu: il gestore globale lo ignora quando un campo di testo ha il focus o una finestra modale è aperta.
- **macOS**: menu nativo di Tauri (`tauri::menu`), costruito dal backend con un elenco di voci che l'interfaccia invia (id del comando, etichetta tradotta, scorciatoia, attivo o no) tramite un comando `set_app_menu`. Il menu si aggiorna quando cambiano lingua o disponibilità; i clic arrivano all'interfaccia come evento `menu://command` e il registro esegue il comando con quell'id. Su macOS il `MenuBar` React non si mostra.
- **Registro dei comandi**: `Command` riceve un campo `menu?: { menu: "file" | "edit" | "view" | "go" | "tools" | "help"; section: number; order: number }` e un `disabledReason?: () => string | undefined`. I menu leggono il registro: nessun elenco separato. Le voci dei moduli non montati (ad esempio File › Apri un progetto FujiView…) sono comandi globali che aprono il modulo e poi l'azione.
- **Nuove scorciatoie**: Ctrl+O (Importa una cartella…) e Ctrl+E (Esporta…). Oggi non sono usate da nessun comando.
- Il `HelpMenu` a icona viene rimosso e le sue voci diventano il menu Aiuto.

*Alternative scartate*:
- menu nativo su tutte le piattaforme: su Windows il menu Win32 non segue il tema scuro, non mostra il motivo di una voce disattivata e va ricostruito a ogni cambio di lingua;
- solo palette dei comandi, che non risponde alla richiesta e resta poco visibile per chi non conosce Ctrl+K.

### D7. Informazioni sull'app
**Rust**:
- `app/src-tauri/build.rs` imposta `PC_BUILD_COMMIT` (da `git rev-parse --short HEAD`, vuoto se non disponibile), `PC_BUILD_DATE` (da `SOURCE_DATE_EPOCH` se presente, così le build sono riproducibili, altrimenti la data attuale) e `PC_BUILD_PROFILE`;
- `app_info` restituisce anche commit, data, configurazione, sistema e architettura, versione del motore della finestra (`tauri::webview_version`), acceleratore attivo (lo stato del provider che già usa la diagnostica), autori, repository e licenza;
- autori, repository e licenza vengono da `CARGO_PKG_AUTHORS`, `CARGO_PKG_REPOSITORY` e `CARGO_PKG_LICENSE`; in `[workspace.package]` va aggiunto `repository`.

**Licenze di terze parti**: un nuovo script, `scripts/licenses.mjs`, che sostituisce ed estende `ui-licenses.mjs`. Genera `assets/licenses/third-party.json` con nome, versione, licenza (SPDX), tipo, testo e nota sulle sorgenti, unendo:
- le dipendenze Rust da `cargo metadata --format-version 1`, cioè quelle effettivamente compilate per il target, con il file di licenza accanto al `Cargo.toml` del pacchetto o il testo SPDX standard;
- le dipendenze di produzione dell'interfaccia da `app/ui/node_modules`;
- modelli e librerie native da un manifest scritto a mano (`assets/licenses/manual.json`), con i dati della tabella di `assets/README.md`.

Lo script fallisce se un pacchetto non ha una licenza riconosciuta. La CI lo esegue e confronta il risultato con il file nel repository. Il comando `third_party_licenses` lo legge dalle risorse installate.

*Alternativa scartata*: `cargo-about`, che richiede un'installazione in più su CI e sulle macchine locali; `cargo metadata` basta.

**Interfaccia**:
- il componente `AboutPanel` è usato in due posti, la finestra Informazioni e Impostazioni › Informazioni;
- "Copia informazioni" usa `navigator.clipboard.writeText`, che nel WebView di Tauri funziona; se fallisce, compare un errore in linea;
- il percorso dei dati non è incluso nella copia.

### D8. Shell: intestazione, scorrimento, barra di stato
- **Intestazione**: `ModulePage` riceve `subtitle`, `tabs` e `actions`, e diventa l'unica intestazione. Cataloghi, Attività, Libreria e Varianti RAW la adottano: in Libreria il titolo è "Libreria" e il contesto ("Tutte le foto", "186 foto") resta nella barra della vista.
- **Scorrimento**: un solo contenitore scorrevole per modulo (`.module-body`), con `html, body, .app` a `overflow: hidden`. Esporta perde lo scorrimento del documento (F5).
- **Barra di stato**: una riga (`white-space: nowrap`, testo abbreviato con i puntini). Sequenza non vi aggiunge più le scorciatoie, che stanno nel foglio delle scorciatoie e nei menu, né il set attivo, che è già nella barra del modulo.
- **Pannelli**: ogni pannello richiudibile ha un solo comando, un'icona di pannello nell'intestazione della colonna; si tolgono i doppioni "<" e ">".

### D9. Navigazione
`NAV_GROUPS`:
- il gruppo `ai` sparisce e la voce `ai` passa in coda a `organize`. L'ordine complessivo resta identico, quindi `moduleForShortcut` dà gli stessi numeri;
- l'etichetta del modulo diventa "Analisi AI";
- la voce RAW diventa "Varianti RAW", con la fotocamera nel sottotitolo della pagina;
- le etichette dei gruppi perdono il maiuscolo.

Un test fissa la corrispondenza Ctrl+N → modulo per tutti i nove numeri.

### D10. Fondamenta visive
- **Token tipografici** in `theme/tokens.ts` e `styles/global.css`: `--pc-font-12`, `--pc-font-14`, `--pc-font-16`, `--pc-font-20`, più un solo stile `.section-title` (16/600). Le dimensioni esistenti si riportano così: 11 → 12, 13 → 14, 17 e 18 → 16.
- Un test vitest scorre i file `.css` e `.tsx` e fallisce per un `font-size` diverso dai token, per `text-transform: uppercase` e per la classe `mono` usata fuori da percorsi e codici (elenco consentito esplicito).
- I conteggi diventano testo normale; i chip colorati restano solo per stati (avviso, errore, completato) e mai per uno zero.
- Gli stili inline degli stati selezionati in `GroupList` e `FaceGrid` passano alle classi dei componenti condivisi (`data-selected`, `aria-current`).

### D11. Testi e numeri
- **Glossario**: si aggiungono "persona", "persona senza nome", "cerchia", "Sposta in…" e "Unisci a…" al test del glossario esistente, con l'elenco di parole vietate nei testi italiani di Persone ("identità", "Riassegna", "Assegna a", "Unisci in", "Gruppo" come campo).
- **Numeri**: `fmtDec` per ogni decimale (anche la sensibilità della persona, che però mostra la scala e non il numero), `_one` / `_other` per ogni conteggio, e un test sulle chiavi i18n con `{{count}}`.
- **Avvisi**: le chiavi generiche `feedback.done` e `feedback.saved` vengono eliminate. Ogni chiamata a `notify` passa un testo specifico; un test cerca usi residui.

### D12. Primi passi, suggerimenti, fasi
- **Primi passi**: la voce "catalogo" conta solo i cataloghi con `system == null` (F3); la chiusura passa in alto a destra.
- **Coachmark**: non è più un popover sopra la barra. Diventa una nota in linea ancorata accanto al titolo o all'elenco, che non si sovrappone ai comandi. `Coachmark` riceve un `placement` vincolato (`inline-start`, `inline-end`) e un test controlla che il suo rettangolo non intersechi i pulsanti della pagina.
- **Fasi**: si aggiungono `geo`, `clip`, `persons` e `faces` (e ogni altra fase emessa) in `jobs.phases`. Un test Rust esporta l'elenco dei nomi delle fasi in `app/ui/src/test/fixtures/phases.json` e un test vitest verifica che ogni nome abbia una traduzione (F4).

## Risks / Trade-offs

- [I test di Persone sono molti e legati alla struttura attuale (`BulkPeople`, `FaceGrid*`, `PeopleCorrections`)] → si riscrivono per fase. Le correzioni del backend non cambiano, quindi i test Rust restano validi; i test UI si aggiornano insieme al componente che cambiano, mai tutti alla fine.
- [Alt per la barra dei menu entra in conflitto con le scorciatoie che usano Alt] → la barra si attiva solo con Alt premuto e rilasciato senza altri tasti, come in Windows; F10 funziona sempre. Un test copre "Alt+1 non apre il menu".
- [`ActionBar` misura lo spazio: possibili sfarfallii al ridimensionamento] → la misura usa la larghezza delle azioni calcolata una volta per lingua e un `ResizeObserver` con `requestAnimationFrame`; niente misure a ogni render.
- [Licenze Rust: crate senza file di licenza o con espressioni SPDX composte] → testi SPDX standard inclusi nello script per le licenze più comuni (MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, MPL-2.0, BSL-1.0, CC0-1.0); per il resto lo script fallisce e chiede di aggiungere il testo a `manual.json`.
- [Menu nativo su macOS: non verificabile sulla macchina di sviluppo Windows] → la logica (elenco delle voci e inoltro degli eventi) è testata in Rust con un finto menu. La prova reale si fa sul Mac in fase di rilascio, come per `publish-mac`.
- [La sezione Informazioni espone il percorso dei dati] → nella finestra sì, perché serve a chi fa assistenza; nella copia negli appunti no, salvo scelta esplicita.
- [Cambio di etichette: "Analisi AI" e "Varianti RAW" compaiono in documenti e altre spec] → le altre spec usano "modulo AI" e "Varianti RAW (Fujifilm X-T5)" come nomi del modulo; i nomi restano comprensibili e non serve modificarle in questa change. Il README si aggiorna.

## Migration Plan

- Nessuna migrazione di dati. Gli id dei moduli non cambiano, quindi le preferenze di layout salvate restano valide.
- Integrazione per fasi (vedi `tasks.md`), ognuna con test verdi e mergiabile da sola:
  1. difetti e testi;
  2. shell e fondamenta visive;
  3. barra dei menu e Informazioni;
  4. Persone;
  5. ritocchi degli altri moduli.
- Rollback: ogni fase è un merge separato e si può annullare con un revert; nessun formato su disco cambia.

## Open Questions

- Testo definitivo di "Sviluppato da" e URL del repository: vengono dai metadati di `Cargo.toml` (oggi `photo-cat contributors`, senza `repository`). Si decidono prima della release senza toccare il codice.
- Soglie in parole del suggerimento (D4): provvisorie, la change sull'aggregazione delle identità le ricalibra.
