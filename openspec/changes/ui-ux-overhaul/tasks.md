# Tasks

## 0. Prerequisiti (vincolanti: senza questi non si inizia)

- [x] 0.1 Verificare che `loading-feedback` e `people-faces-workspace` siano integrate in `main` e archiviate, e che `catalog-exclusions` sia archiviata (`openspec list --json` senza change aperte oltre a questa; `openspec/changes/archive/` le contiene). Se una non c'è ancora, fermarsi.
- [x] 0.2 Fare il rebase di `feat/ui-ux-overhaul` su `main`, poi riallineare le delta:
  - `specs/desktop-shell/spec.md`: "Indicatori globali", "Moduli di navigazione" e "Impostazioni" sul testo archiviato (`openspec show desktop-shell --type spec`), mantenendo tutti gli scenari esistenti;
  - `specs/external-apps/spec.md`, sul testo archiviato;
  - verificare con `openspec validate ui-ux-overhaul --strict`.
- [x] 0.3 Rileggere l'audit (`audit.md`) sul codice integrato: aggiornare righe e riferimenti, segnare i punti già risolti da `loading-feedback` e `people-faces-workspace`, aggiungere eventuali nuovi problemi.
- [x] 0.4 Decidere le domande aperte di `design.md` (dipendenze D3, visualizzatore dei RAW, nome Esporta, "Smart") e annotare la scelta in `design.md`.

## 1. Fondamenta

- [x] 1.1 Token di spaziatura, raggio, tipografia, elevazione, movimento e `z` in `theme/tokens.ts`, scritti come `--pc-*`.
  - Tema `system`, che segue `prefers-color-scheme`.
  - Nuove coppie di contrasto 3:1 (bordo, icona, focus).
  - Verifica: `tokens.test.ts` per entrambi i temi.
- [x] 1.2 Aggiungere le dipendenze scelte in 0.4 e le relative licenze in `assets/licenses`.
  - Sostituire le icone con l'adattatore `components/icons.tsx` e rimuovere i glifi di testo (▸ ▾ ↻ ✎ × ↑ − ‹).
  - Verifica: `npm run build` e `make test-linux`.
- [x] 1.3 Componenti in `components/ui/`:
  - `Button` (primary, secondary, ghost, danger, con spiegazione del disabilitato);
  - `IconButton` (etichetta obbligatoria);
  - `PageHeader`, `Card`, `FormField`, `InlineAlert`, `EmptyState`, `Badge`, `Kbd`;
  - `SegmentedControl`, `Tabs`, `Disclosure`, `Tooltip`;
  - `Menu` e `ContextMenu`, `Popover`;
  - `Dialog` e `ConfirmDialog` (verbo esplicito, conteggio, effetto sui file);
  - `FolderPicker`, `DataTable` (righe selezionabili da tastiera).

  Verifica: un test vitest per componente su ruoli, focus, Esc, ritorno del focus e Invio.
- [x] 1.4 `ToastHost` unico, a partire dal Toast di `loading-feedback`: pila, chiusura, azione, durata, `role="status"` e `alert` per gli errori, nessuna copertura del focus. Verifica con un test sulla pila di due avvisi.
- [x] 1.5 Store `undo`, il registro globale dell'ultima operazione annullabile, e scorciatoia Ctrl+Z.
  - Elencare le operazioni annullabili esistenti e i loro comandi di annullamento.
  - Annotare qui quelle che non hanno un annullamento lato backend prima di continuare.
  - Verifica: test sul registro (registrazione, esecuzione, sostituzione).

  **Inventario (3 novembre 2026).** Il backend ha un solo registro LIFO (`undo_log`): `undo_bulk` annulla l'ultima voce qualunque sia, `undo_people` solo se è una correzione delle persone (ed emette `faces_changed`). Lo store ricorda quindi una sola operazione, e ogni comando che scrive in `undo_log` la fa decadere (`UNDO_LOG_COMMANDS` in `store/undo.ts`), così Ctrl+Z non annulla mai un'operazione diversa da quella mostrata.

  Con annullamento lato backend:
  - `startBulk`: tag, cataloghi, esclusioni e reinserimenti, categorie, luogo, tag del set → `undoBulk` (non `assign_person` e `remove_library`, vedi sotto);
  - `renameTag`, `mergeTag`, `deleteTag` → `undoBulk`;
  - `renamePlace`, `deleteCategory` → `undoBulk`;
  - `setGroupShift`, `placeUndated` (Sequenza) → `undoBulk`;
  - `correctPerson` (unione, separazione, sposta volti, ignora, ripristina, non è la stessa persona, copertina) → `undoPeople`.

  **Senza annullamento lato backend** (nessun nuovo comando in questa change):
  - definizione dei cataloghi: `saveCatalog`, `deleteCatalog` → eliminazione con `ConfirmDialog` (7.2);
  - rimozione dalla libreria: `removeSource`, `removeMissing` → `ConfirmDialog` (3.3, 7.1);
  - tassonomia: `addCategory`, `saveTaxonomy` → modifica esplicita con "Applica" (6.2);
  - set di tag e colori: `saveTagSet`, `duplicateTagSet`, `deleteTagSet`, `reorderTagSets`, `addTagToSet`, `removeTagFromSet`, `setTagColor` → `ConfirmDialog` solo per `deleteTagSet` (7.3);
  - foto simili: `similarRemove`, `similarGroupTogether`, `similarDissolve`, `similarSetCover` → nessuna conferma, si correggono a mano;
  - identità: `saveIdentity`, `deleteIdentity`, `mergeIdentities`, `removeIdentityFace`, `linkPersonIdentity` → `ConfirmDialog` per eliminazione e unione (7.4);
  - `recomputePlaces` → nessuna conferma (ricalcolo).
  - `startBulk` con `assign_person` (`people::assign` non registra) e `remove_library` (cancella righe e miniature) → `ConfirmDialog` nella barra delle azioni (3.2).

  Da cablare nelle fasi successive: Libreria (3.3), Tag (7.3), Persone (7.4; finché il registro globale è vuoto, Ctrl+Z in Persone chiama ancora `undoPeople`), Sequenza e Luoghi (3.3, 7.5).
- [x] 1.6 Glossario in `i18n/GLOSSARY.md`; chiavi `common.*` (Annulla, Salva, Chiudi, Interrompi, Annulla operazione, Riprova, Dettagli); formattazione delle date con `Intl`. Test sui termini vietati per lingua.
- [x] 1.7 Test di accessibilità sul DOM: bersagli sotto 24 px, controlli annidati, pulsanti con sola icona senza nome. Va eseguito sulle pagine di ogni modulo nei test esistenti.
  - Fatto con `test/a11y.ts` (`checkedCleanup` al posto di `afterEach(cleanup)` nei test di pagine e componenti). La dimensione si legge dallo stile (classi e stile in linea: `width`/`height` e minimi), perché jsdom non calcola il layout: i bersagli senza dimensione nello stile non sono misurati e restano alla prova manuale (8.4).
  - Problemi aperti in `test/a11yKnown.ts`, ognuno con il task che lo risolve (3.5, 3.7, 7.4); 8.1 lo svuota.

## 2. Shell e navigazione

- [x] 2.1 `modules/registry.ts` a gruppi (Sfoglia, Organizza, Importa ed esporta, più le voci in fondo); `ModuleNav` con etichette di gruppo, badge (Persone da rivedere, Attività in corso), modalità compatta (Ctrl+B) e passaggio automatico sotto i 1.008 px. Verifica: `shell.test.tsx`.
- [x] 2.2 Modelli AI e Diagnostica come sezioni di Impostazioni:
  - `openModule({ module, section, target })` al posto di `setModule` e `openDiagnostics`;
  - migrazione di `store/layout.ts` (`version` +1, `models` / `diagnostics` → `settings/<sezione>`, insieme a quella di Identità);
  - aggiornare tutti i collegamenti (barra di stato, avvisi di crash, Attività › "Vedi errori").

  Verifica: test della migrazione da preferenze salvate con i vecchi valori.
- [x] 2.3 Modulo **Attività** (ex Job):
  - titoli in parole (tipo + oggetto);
  - "Interrompi" con conferma solo se l'attività scrive file;
  - gestione degli errori dei comandi pausa/riprendi/interrompi;
  - storico con titolo, date e operazioni annullabili;
  - stato vuoto con "Importa una cartella".
- [x] 2.4 Barra superiore: indicatore delle attività in parole, menu Aiuto (Diagnostica, Informazioni; le voci della guida si aggiungono con 5.4). Il campo che apre la palette si fa con 4.2, dove nasce la palette: fino ad allora il campo filtra la Libreria come prima. Barra di stato: aggiungere RAM (spostata dalla barra superiore, con icona di pressione) e modelli; scorciatoie contestuali al posto del testo lungo di Sequenza.
- [x] 2.5 Pannelli: Ctrl+Maiusc+B nasconde e mostra i laterali, Ctrl+I l'ispettore; sezioni del pannello sinistro richiudibili e persistite. Verifica: test di `Panels`.
  - Tab resta allo spostamento del focus (WCAG 2.1.1): scelta dell'utente del 3 novembre 2026, spec `desktop-shell` e `command-palette` aggiornate.
- [x] 2.6 `PageHeader` su tutte le pagine dei moduli, con titolo uguale alla voce di navigazione (Importa, Esporta, Attività, Sequenza…).
  - `ModulePage` per Sequenza, Persone e Tag; `PageHeader` in Importa, Esporta, Cataloghi, Attività e Impostazioni. La Libreria ha la sua intestazione con il percorso in 3.1 (`LibraryHeader`).

## 3. Libreria

- [x] 3.1 `LibraryHeader`:
  - percorso, conteggio, chip, menu "Ordina" e "Vista", interruttore dell'ispettore;
  - rimuove `CatalogHeader`, `CatalogFeedback` e la vecchia `Toolbar`;
  - "Solo foto con simili" va al pannello Filtri, con il suo chip.

  Verifica: `LibraryPage.test.tsx` (catalogo, escluse, chip).
- [x] 3.2 `SelectionBar` al posto di `BulkBar`:
  - visibile solo con una selezione;
  - comandi diretti con popover di ricerca, "Altro", Esc per deselezionare;
  - "Esporta" apre il modulo Esporta con l'ambito già impostato;
  - mantenere tutte le azioni e i test di `bulk-operations`.

  Verifica: `BulkBar.test.tsx` riscritto come `SelectionBar.test.tsx`.
- [x] 3.3 Tutte le operazioni della Libreria registrate nello store `undo`, con avvisi "Annulla operazione"; nessun `window.confirm` (la rimozione dalla libreria diventa annullabile oppure usa `ConfirmDialog`, secondo 1.5).
- [x] 3.4 Inspector:
  - nuovo ordine delle sezioni;
  - `Disclosure` persistite;
  - modifica sul campo (`InlineEdit`) al posto del modulo "Modifica";
  - selezione multipla con valori comuni.

  Verifica: `Inspector.test.tsx`.
- [x] 3.5 Griglia:
  - focus "roving" con frecce, Home/Fine, Pagina su/giù, Maiusc+freccia, Ctrl+Spazio, Ctrl+A;
  - focus distinto dalla selezione;
  - il gruppo di simili diventa un pulsante separato (niente controlli annidati).

  Verifica: test della griglia virtualizzata.
- [x] 3.6 Visualizzatore condiviso `components/viewer/Viewer.tsx`, estratto da Sequenza:
  - Spazio/Invio, frecce, 1–9/0, I, Esc;
  - stessa foto con il focus alla chiusura;
  - opzione "Visualizzatore di photo-cat" per il doppio clic (`external-apps`).

  Verifica: test in Libreria e Sequenza.
- [x] 3.7 Barra laterale: Cartelle (ex Sorgenti) con nomi leggibili e percorso nel suggerimento, Cataloghi, Filtri con ricerca; il comando "aggiorna cartella" passa nel menu della cartella; bersagli di almeno 24 px.

## 4. Palette e scorciatoie

- [x] 4.1 Registro dei comandi (`commands/registry.ts`) e `useHotkeys` globale, che ignora i campi di testo; migrare i tasti dei tag e quelli di Sequenza. Verifica: test di digitazione in un campo (nessun tag assegnato).
- [x] 4.2 Palette Ctrl+K (aperta anche dal campo di ricerca della barra superiore, spostato qui da 2.4):
  - comandi, persone, tag, cataloghi, luoghi, nomi di file;
  - recenti, "Nessun risultato", azioni sulla selezione;
  - apertura della Libreria filtrata.

  Verifica: test di ricerca, navigazione da tastiera e azione.
- [ ] 4.3 Foglio delle scorciatoie (`?`), generato dal registro, e scorciatoie mostrate in menu e suggerimenti.

## 5. Guida nell'app

- [ ] 5.1 Nuovo onboarding in 3 passi (`Onboarding.tsx`):
  - lingua e tema;
  - cartella con `FolderPicker` e trascinamento;
  - riepilogo del sistema in frasi e download in background.

  Ogni passo si può saltare e alla fine si apre la Libreria. Verifica: `Onboarding.test.tsx`.
- [ ] 5.2 Store `photo-cat.guide`, persistito con `mirroredStorage`, e lista "Primi passi" in Libreria, completata dagli eventi esistenti. Verifica: test di completamento da un'altra strada e di persistenza.
- [ ] 5.3 `Coachmark` e suggerimenti al primo uso di Persone, Cataloghi, Tag e Sequenza, più quello "Dove sono Modelli AI e Diagnostica?" per gli utenti che aggiornano.
- [ ] 5.4 Menu Aiuto: scorciatoie, introduzione, primi passi, suggerimenti, guida, segnala un problema, informazioni.

## 6. Impostazioni

- [ ] 6.1 Layout a sezioni con sotto-navigazione e ricerca (con evidenziazione del campo); contenuto di `ModelsPage` e `DiagnosticsPage` spostato nelle sezioni; rimozione dei duplicati.
- [ ] 6.2 Salvataggio immediato con "Salvato" accanto al campo per le preferenze; barra "Ripristina / Applica" con anteprima per soglie, tassonomia e percorso dei dati; nessun "Salva" a metà pagina. Verifica: `SettingsPage.test.tsx` (le soglie dei gruppi restano preservate).
- [ ] 6.3 Sensibilità:
  - tabella del verso di ogni soglia in `tuning.ts`;
  - scala "Più risultati ↔ Più precisi" e controlli segmentati dove bastano tre livelli;
  - valori numerici in "Avanzate";
  - "Elimina tutti i dati dei volti" in stile di pericolo con `ConfirmDialog`.

## 7. Moduli

- [ ] 7.1 **Importa**: titolo "Importa", `FolderPicker`, pattern di esclusione dietro "Avanzate" con esempi in parole; nessun salto automatico ad Attività (avviso con "Segui l'importazione"); azioni della cartella in un menu; dialoghi su `Dialog`.
- [ ] 7.2 **Cataloghi**:
  - editor delle regole leggibile (etichette dei campi e degli operatori, selettori di valore per tipo, frase di riepilogo, conteggio dal vivo);
  - schede con tipo, foto ed escluse;
  - eliminazione con `ConfirmDialog` e annullamento se possibile;
  - stato vuoto.

  Verifica: test dell'editor (nessun id interno nel DOM).
- [ ] 7.3 **Tag**: conferme vicino al comando o `ConfirmDialog`; annullamento tramite lo store `undo` (la nota non rimanda più alla Libreria); errori dei tasti rapidi segnalati con un avviso; un solo "Rinomina".
- [ ] 7.4 **Persone**: allineare `PeopleModule` di `people-faces-workspace` ai componenti condivisi, al glossario e allo store `undo` (unione, separazione, ignora), senza cambiarne il flusso.
- [ ] 7.5 **Sequenza**:
  - `PageHeader`;
  - date formattate;
  - "Pausa tra sessioni" con conferma visibile;
  - iniziale o icona del dispositivo oltre al colore;
  - export con le voci tecniche dietro "Avanzate" e testi comprensibili;
  - caricamento con `QueryView`.
- [ ] 7.6 **Esporta** (ex "Esporta e organizza"):
  - sezioni "Esporta dati", "Organizza in cartelle" e "Sequenza";
  - ambito predefinito "Libreria filtrata" quando non c'è una selezione;
  - passi "Anteprima → Verifica → Crea" come elenco numerato con lo stato;
  - pulsanti con etichetta;
  - storico leggibile;
  - "Sposta file" con `ConfirmDialog`.
- [ ] 7.7 **Impostazioni › Diagnostica**:
  - tabella da tastiera (`DataTable`);
  - errori come `InlineAlert` di errore;
  - fasi e livelli tradotti;
  - "Segnala un problema" in evidenza.
- [ ] 7.8 **Pannelli flottanti**: spostamento e ridimensionamento da tastiera (scorciatoie nel menu del pannello), `z` dai token, etichetta di chiusura propria.

## 8. Pulizia e verifica

- [ ] 8.1 Rimuovere i `window.confirm` rimasti (atteso: 0), le chiavi i18n inutilizzate e quelle prese in prestito; ridurre gli stili inline ai valori dinamici. Svuotare le eccezioni `PENDING` di `i18n/glossary.test.ts` e `test/a11yKnown.ts` (stringhe non ancora allineate al glossario). Verifica: `grep` in CI per `window.confirm` e test delle chiavi.
- [ ] 8.2 Aggiornare il README (navigazione, scorciatoie, Impostazioni, primo avvio) e gli screenshot, se ci sono.
- [ ] 8.3 Eseguire `npm test`, `npm run typecheck`, `npm run build`, `cargo test` e `make test-linux`, correggendo ogni errore.
- [ ] 8.4 Prova sull'app reale, con un backup dei dati:
  - aggiornamento da una versione con preferenze salvate (moduli Identità, Modelli e Diagnostica come ultimi aperti);
  - giro completo solo da tastiera: importa → Libreria → selezione → tag → catalogo → visualizzatore → palette → Impostazioni → Attività;
  - temi chiaro, scuro e sistema;
  - finestra a 900 px;
  - lettore di schermo (Narrator / Orca) su dialoghi, palette e barra delle azioni.
- [ ] 8.5 Confronto finale con i mockup e con l'elenco dei problemi di `audit.md`: ogni voce A e M risolta o motivata.

## Workflow

- Branch `feat/ui-ux-overhaul` (worktree `D:\Github\photo-cat-ux`). Una PR per fase o per gruppi di fasi, merge in `main` dopo la CI. Alla fine si archivia la change e si sincronizzano le spec `desktop-shell`, `external-apps` e `catalog-library`, più le nuove `ui-foundations`, `command-palette` e `in-app-guidance`.
