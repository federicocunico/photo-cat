# Tasks

Ogni gruppo si integra da solo con test verdi (`npm --prefix app/ui test`, `typecheck`, `lint`; `cargo test` quando tocca Rust). D = `design.md`.

## 1. Preparazione

- [x] 1.1 Creare il branch `change/unified-selection` da `main` aggiornato e verificare che `make test` passi prima di ogni modifica

## 2. Modello comune

- [x] 2.1 Spostare le funzioni pure di selezione in `lib/selection/model.ts`, generiche su id numerici o testuali, con riesportazione da `store/library.ts`; verificare che i test esistenti di selezione passino senza modifiche (D1)
- [x] 2.2 `useSelection<Id, F>` al posto di `useGridSelection`, con base del backend o id espliciti; verificare con i test esistenti di `useGridSelection` portati al nuovo hook e con un test su id testuali (D1)
- [x] 2.3 `useSelectionKeys` con la tabella tasto × modificatore per griglie ed elenchi; verificare con test unitari di frecce, Home/Fine, Pagina su/giù, Maiusc, Ctrl, Ctrl+Spazio, Ctrl+A, Ctrl+Maiusc+I ed Esc, e che nulla accada in un campo di testo (D2)
- [x] 2.4 `useActivePane` al posto di `useActiveGrid`, con `data-active-pane`, stile del riquadro attivo e `activePane()` per il registro dei comandi; verificare con un test che Ctrl+A agisca solo sul riquadro attivo e che ogni riquadro conservi la propria selezione (D4)
- [x] 2.5 `selectForContextMenu` e regione `aria-live` condivisa per il conteggio; verificare con test del clic destro su elemento selezionato e non selezionato e dell'annuncio "1 selezionata" (D3, D5)
- [x] 2.6 Contratto di test `test/selectionContract.tsx` con gli scenari della spec `item-selection`; verificare che passi su una raccolta di prova (D8)

## 3. Foto

- [x] 3.1 Griglia della Libreria (anche cataloghi, Cestino e foto escluse) sul modello comune: frecce che selezionano, Ctrl+frecce per il solo focus, clic su spazio vuoto che svuota; aggiornare `Grid.test.tsx` e verificare il contratto sulla Libreria
- [x] 3.2 Menu contestuale della griglia con `selectForContextMenu` ed etichette con il conteggio; verificare con i test di `GridContextMenu`
- [x] 3.3 Sequenza e risultati e foto simili del modulo AI sul modello comune; verificare il contratto su Sequenza e sui risultati del modulo AI
- [x] 3.4 Foglio delle scorciatoie e README con le nuove regole (frecce, Ctrl+frecce, Ctrl+Spazio, clic destro); verificare che il test del foglio delle scorciatoie elenchi i nuovi tasti

## 4. Persone

- [x] 4.1 Griglia dei volti con `useSelectionKeys` (frecce, Maiusc, Ctrl, Ctrl+Spazio, Home/Fine) e clic su spazio vuoto; aggiornare `FaceGrid*.test.tsx` e verificare il contratto sulla griglia dei volti
- [x] 4.2 Elenco di Persone (gruppi e identità) con un solo modello di selezione per sezione, Maiusc+frecce ed estensione con Maiusc+clic; le frecce selezionano e aprono il gruppo raggiunto, con caricamento dei volti annullabile dopo 150 ms; verificare il contratto sull'elenco e lo scenario "Estendere con Maiusc"
- [x] 4.3 `usePeopleHotkeys` senza frecce globali: le frecce agiscono sul riquadro attivo; verificare con un test che, con il focus sulla griglia, la freccia giù si muova tra i volti e non cambi gruppo
- [x] 4.4 Menu contestuale di volti ed elenco con `selectForContextMenu`; verificare con i test di `PeopleCorrections`

## 5. Cataloghi e tag

- [x] 5.1 Rust: comando `delete_catalogs(ids)` in un'unica transazione, che salta i cataloghi di sistema e restituisce eliminati e saltati; rigenerare `bindings.ts`; verificare con un test d'integrazione che le foto restino in libreria e che il Cestino non venga eliminato
- [x] 5.2 Pagina Cataloghi come `listbox` a griglia: clic seleziona, doppio clic e Invio aprono, pulsanti della scheda accanto alla voce e non dentro; verificare il contratto sulla pagina Cataloghi e il controllo "nessun controllo annidato" del test di accessibilità (D6)
- [x] 5.3 "Elimina N cataloghi…" nell'intestazione della pagina, con Canc e menu contestuale, e conferma con i nomi e la nota sui cataloghi di sistema; verificare gli scenari di "Eliminazione di più cataloghi"
- [x] 5.4 `useTypeAhead` nella pagina Cataloghi e nell'elenco dei tag; verificare con un test che digitando "vi" si selezioni "Viaggio Toscana" e che in Persone la ricerca per iniziale non sia attiva (D7)

## 6. Verifica d'insieme

- [x] 6.1 `make test` in locale e `make test-linux` in Docker, con le correzioni necessarie
- [ ] 6.2 Prova sull'app reale (per l'utente): Libreria, Sequenza, Persone (elenco e volti) e Cataloghi solo con la tastiera, poi solo con il mouse, confrontando il comportamento con Esplora file

## Workflow follow-up

- Aprire la PR, attendere la CI verde e fare il merge, prima di iniziare `ui-usability-audit`.
- Archiviare la change al momento della release.
