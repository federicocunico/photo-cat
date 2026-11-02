# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Mai cancellare né modificare file sul disco.** Per le prove usare `D:\Foto` solo in lettura, con una cartella dati separata (`PHOTO_CAT_DATA_DIR` verso una cartella temporanea), mai la cartella dati dell'utente.
> - **Migrazione di schema**: `catalog-exclusions` introduce anch'essa una migrazione. Prima di iniziare controllare l'ultima versione in `main` e usare la successiva; chi arriva per secondo rinumera.
> - **`quality.rs` (nitidezza)** è condiviso con `people-faces-workspace` (design §4): se esiste già in `main`, riusarlo.
> - **Requisito condiviso**: anche `people-faces-workspace` modifica "Soglie dell'automatismo configurabili" in `ai-classification`. Prima dell'archiviazione, allineare la delta al testo già archiviato dall'altro change.
> - **Selezione**: se `large-library-stability` è già unito, "Seleziona tutte tranne la principale" deve usare la sua rappresentazione della selezione (per filtro più eccezioni) invece di una lista di id.

## 1. Dati

- [x] 1.1 Migrazione con `similar_group`, `similar_member`, `similar_link`, `similar_edge`, `similar_done` e `similar_dissolved` (design §2, §3). Verifica: test di migrazione dall'ultima versione e cancellazione a cascata dei membri quando si rimuove una foto.
- [x] 1.2 `ai.thresholds.similar_photos` in `pc-core/src/config.rs` con validazione 0–1 e predefinito 0,80. Verifica: unit test su `config.toml` senza il campo e su un valore fuori intervallo.

## 2. Algoritmo

- [x] 2.1 `pc-ai/src/quality.rs` (nitidezza ed esposizione) se non esiste già, e SSIM su miniature a 128 px in scala di grigi con controllo delle proporzioni. Verifica: unit test con immagini sintetiche (identica = 1, sfocata più bassa, spostata di pochi pixel sopra 0,8, soggetto diverso sotto 0,5).
- [x] 2.2 `pc-ai/src/similar.rs`: candidati semantici (HNSW), temporali e duplicati esatti, conferma SSIM, union-find con vincoli e divisione dei componenti oltre 50 foto (design §1, §3). Verifica: test con embedding e miniature sintetiche per gli scenari "Raffica", "Stesso luogo, inquadratura diversa", "Foto senza data", i vincoli `must`/`cannot` e un conflitto risolto a favore della correzione più recente.
- [x] 2.3 Fase `similar` nel job di analisi, incrementale con `similar_done`, annullabile, con budget del governor e ripiego a blocchi (design §5). Verifica: test di integrazione in `crates/pc-ai/tests/` per "Nuovo import" (solo i confronti delle foto nuove) e "Interruzione", e test di prestazioni con 100.000 embedding sintetici entro il budget.
- [x] 2.4 Principale proposta e protezione della scelta manuale. Verifica: test per "Principale proposta" e "Scelta dell'utente".
- [x] 2.5 Retune e anteprima per la soglia dei gruppi da `similar_edge`. Verifica: test per "Soglia più severa" senza accesso alle miniature, e confronto tra l'anteprima e l'esito.

## 3. Comandi e query

- [x] 3.1 Comandi per togliere, raggruppare, sciogliere e impostare la principale; `similar_group_members`, `similar_non_covers`; `LibraryFilter.similar_only`; rigenerare `bindings.ts`. Verifica: test Rust per "Foto tolta da un gruppo", "Unione manuale", "Gruppo sciolto" e "Pulizia di un catalogo" (tutti i file su disco intatti).
- [x] 3.2 Pagine della griglia con `group_similar` (un elemento per gruppo, conteggio sui risultati filtrati, paginazione per cursore) (design §6). Verifica: test per "Gruppo chiuso" e "Gruppo parzialmente nei risultati", e test di prestazioni (prima pagina entro 1 s con 100.000 foto).

## 4. Interfaccia

- [x] 4.1 Opzione "Raggruppa foto simili" nella barra della griglia (Libreria, cataloghi, Sequenza), miniatura di gruppo con numero, espansione sul posto, selezione di un gruppo chiuso; chiavi i18n it/en. Verifica: test con `ipcMock` per "Gruppo chiuso" ed "Espansione".
- [x] 4.2 Filtro "Solo foto con simili", azione bulk "Seleziona tutte tranne la principale", sezione "Foto simili" nell'Inspector con "Imposta come principale" e "Togli dal gruppo". Verifica: test per "Inspector" e per la selezione delle non principali.
- [x] 4.3 Soglia nella sezione "Soglie AI" delle Impostazioni, con anteprima del numero di gruppi; proposta dell'analisi dei gruppi per le librerie esistenti. Verifica: test in `SettingsPage.test.tsx` per lo scenario "Soglia dei gruppi di foto simili".

## 5. Verifica d'insieme

- [x] 5.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 5.2 Taratura su `D:\Foto` in sola lettura con una cartella dati separata: importare, analizzare i gruppi, controllare a campione falsi positivi e negativi, e annotare in Open Questions i valori scelti (soglia SSIM, coseno, finestra) e i tempi. Verifica: file in `D:\Foto` invariati (confronto del numero di file e delle date di modifica prima e dopo), valori documentati nel design.
- [x] 5.3 Aggiornare il `README.md` (foto simili), allineare le delta e verificare che `openspec validate similar-photo-stacks --strict` passi.

## Workflow follow-up

- Coordinare il numero di migrazione con `catalog-exclusions` e il requisito condiviso con `people-faces-workspace`.
- Archiviare il change dopo la revisione.
