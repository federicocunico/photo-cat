# Tasks

> Esecuzione: un solo subagente alla volta, con i test eseguiti in ogni task; la revisione finale va a un subagente Opus. Issue di riferimento: #51.

## 1. Backend

- [x] 1.1 `identities::merge_many(from: &[uid], into)` in una sola transazione dell'archivio delle identità, seguita dal riallineamento delle persone nella libreria aperta (`people::relink_identity` per ciascuna, o una variante a lista). Comportamento richiesto:
  - una persona collegata alla destinazione esiste già: le altre vi vengono unite;
  - non esiste: la prima diventa la sua e le altre vi vengono unite;
  - la destinazione tra le selezionate viene ignorata come sorgente;
  - un errore a metà non modifica nessuna identità.

  Comando `merge_identities_many` e un'anteprima dei conteggi (identità, foto, volti di riferimento) per la conferma. Test Rust su ogni scenario di `identity-registry` "Unione di più identità".
- [x] 1.2 Assegnazione in blocco dei gruppi in `people.rs`: verso una persona esistente, un'identità (che crea o riusa la persona collegata) o una nuova persona con nome. Usa `merge_many`, rispetta i rifiuti ("non è questa persona": i volti rifiutati restano da rivedere) e registra un solo annullamento.
  - Aggiunge il filtro per numero di foto nell'elenco dei gruppi e delle identità, e la base di selezione completa dell'elenco filtrato, per "Seleziona tutto" su centinaia di righe.
  - Comandi Tauri e binding.
  - Test Rust: 420 gruppi in una chiamata con un solo annullamento, identità non collegata, nuova persona, rifiuti rispettati, filtro per numero di foto.

## 2. Interfaccia

- [x] 2.1 Sezione "Persone": selezione multipla delle identità con le scorciatoie di selezione, filtro "con al massimo N foto" e dialogo "Unisci in…" con ricerca e conferma con i conteggi. In "Da rivedere" e "Singoli": filtro per numero di foto, "Seleziona tutto" e dialogo "Assegna a…" con ricerca, includendo "Nuova persona…", e avviso con "Annulla". i18n it/en. Test vitest sugli scenari delle spec.
- [x] 2.2 Revisione Opus: aderenza alle spec, prestazioni con centinaia di elementi, sicurezza dell'unione non annullabile (conferma chiara), testi per un utente non tecnico.
  - Unione di identità: `app.db` ora scrive dentro la scrittura della libreria, dopo il riallineamento delle persone (un errore di `app.db` annulla anche la libreria); resta solo la finestra di un commit della libreria fallito dopo quello di `app.db`, documentata in `merge_many`. Anche `merge_identities` (singola) usa lo stesso percorso. Adottando una persona per una destinazione non collegata si preferisce una persona non ignorata.
  - Conferma dell'unione: oltre ai conteggi mostra i primi nomi che spariranno e avvisa se alcune identità non hanno foto in questa libreria (potrebbero averne in altre); la selezione è fissata all'apertura del dialogo e cambiare il filtro per numero di foto azzera la selezione.
  - Scorciatoie: una vista con `hasFocus` (l'elenco Persone) è attiva solo mentre l'utente ci lavora (ultimo clic o focus), indipendentemente dall'ordine di registrazione; Libreria, Sequenza e volti invariati.
  - Prestazioni: elenco delle identità virtualizzato, ricerche per insiemi in UI e in `assign_groups`; test con 10.000 gruppi (limite) e rifiuto oltre il limite già nel dialogo.
  - "Nuova persona…": se l'assegnazione fallisce l'identità appena creata viene eliminata.

## 3. Verifica

- [ ] 3.1 Verifica completa locale: fmt, clippy, test Rust, typecheck, lint e test UI, `make test-linux` e la build NSIS. Poi PR con CI verde.

## Workflow follow-up

- Merge, archiviazione della change, release 0.5.1, chiusura della issue #51.
