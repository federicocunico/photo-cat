# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Origine**: issue GitHub #4 "Nuova voce in catalogazione". Nel messaggio finale del lavoro citare l'issue, ma non chiuderla senza conferma dell'utente.
> - **Conflitti con `manual-edits-open-with-ai-tuning`** (branch `feat/user-feedback-tuning`, implementazione in corso): tocca `taxonomy.rs` (soglia per voce), `classify.rs`, `SettingsPage.tsx`, `Inspector.tsx`, `BulkBar.tsx` e introduce il job `retune`. Prima di iniziare controllare se è già unito in `main`; se sì, fare il rebase, includere la soglia per voce nella parte della tassonomia rilevante per l'analisi (design §2) e usare `retune` per rendere automatica una voce. Se non è unito, non dipendere dal suo codice.
> - **Conflitti con `facet-lists-expand`** (branch `feat/facet-lists-expand`): riscrive le liste dei filtri in `Sidebar.tsx`; per il nome delle voci passare `useLabelName` alla funzione `render` di `FacetList`.
> - **Nessuna migrazione di `catalog.db`**: se un task sembra richiederla, fermarsi e chiedere.

## 1. Tassonomia

- [ ] 1.1 In `pc-ai/src/taxonomy.rs` permettere voci senza prompt e gruppi senza voci automatiche (design §1), e aggiungere la generazione di id univoci da un nome (design §3, gruppi limitati a `[a-z_]`). Verifica: unit test per una voce manuale valida, un gruppo solo manuale valido, un gruppo vuoto rifiutato, gli id `barche`/`barche_2` e un id di gruppo accettato da `bulk::set_label`; `cargo test -p pc-ai taxonomy` passa.
- [ ] 1.2 Aggiungere `analysis_version` con valore iniziale `version` per le tassonomie salvate e il confronto della parte rilevante per l'analisi in `save()` (design §2). Verifica: test che caricano un JSON senza il campo, e un test per ogni tipo di modifica (voce manuale aggiunta, nome cambiato, prompt cambiato, voce automatica disattivata, soglia di gruppo cambiata) che controlla se `analysis_version` cambia come previsto.
- [ ] 1.3 Escludere le voci manuali da `text_cache` e `classify::labels`, e confrontare in `text_cache` la sola parte rilevante. Verifica: test che una voce manuale non produce concetti né etichette automatiche, e che rinominare una voce non invalida la cache.
- [ ] 1.4 Registrare `analysis_version` in `taxonomy_ver` (`analyze.rs`, `stages.rs`), usata da `pending` e `stale_count`. Verifica: test di integrazione in `crates/pc-ai/tests/` per gli scenari "Voce manuale aggiunta" e "Rinomina del nome visualizzato" (da rianalizzare resta 0) e "Prompt modificato" (diventa > 0).

## 2. Comandi

- [ ] 2.1 Comando `add_category(group, label, prompts, assign_to)` con riuso dei nomi equivalenti, un nuovo tentativo in caso di conflitto di versione e assegnazione tramite `BulkAction::SetLabel` (design §4); rigenerare `bindings.ts`. Verifica: test Rust per "Nuova voce in bulk" (voce creata una volta sola, 60 etichette manuali, annullamento che toglie le etichette ma conserva la voce), "Nome già esistente" e "Nome vuoto".
- [ ] 2.2 Comandi `category_usage(group, id)` e `delete_category(group, id)`, con cancellazione delle etichette in una transazione annullabile (design §5). Verifica: test per "Eliminazione di una voce usata" (25 foto) e per il suo annullamento.

## 3. Interfaccia

- [ ] 3.1 Hook `useLabelName` (design §6), usato in `Inspector.tsx`, `Sidebar.tsx`, `chips.ts` e `RuleEditor.tsx`. Verifica: test che una voce creata dall'utente mostra il suo nome, una voce predefinita la traduzione e una voce predefinita rinominata il nuovo nome.
- [ ] 3.2 Componente `CategoryPicker` con "Nuova voce…" sul posto, usato dall'Inspector e dall'azione "Imposta catalogazione" della `BulkBar`; chiavi i18n it/en. Verifica: test con `ipcMock` in `Inspector.test.tsx` e `BulkBar.test.tsx` per "Nuova voce dall'Inspector", "Nuova voce in bulk" e "Nome vuoto", inclusi Invio ed Esc.
- [ ] 3.3 In Impostazioni › Tassonomia: "Aggiungi gruppo", descrizione facoltativa con etichetta "manuale", eliminazione con conteggio e avviso di rianalisi solo per le modifiche rilevanti (design §7). Verifica: test in `SettingsPage.test.tsx` per "Gruppo nuovo solo manuale", "Voce resa automatica" (avviso visibile) e assenza dell'avviso quando si aggiunge una voce manuale.
- [ ] 3.4 I gruppi solo manuali compaiono tra i filtri della libreria e nell'editor dei cataloghi smart. Verifica: test con `ipcMock` che il gruppo "Evento" appare nella barra laterale e in `RuleEditor`, e test Rust che una regola smart `Evento = Compleanno` restituisce le foto attese.

## 4. Verifica d'insieme

- [ ] 4.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 4.2 Prova sull'app reale: creare una voce dall'Inspector e una dalla barra bulk su una selezione, creare il gruppo "Evento", rinominare e poi eliminare una voce, e verificare che la schermata Modelli/analisi non indichi file da rianalizzare. Poi aggiungere una descrizione e verificare che lo indichi. Verifica: tutti i passi riusciti, nessun errore nuovo in Diagnostica.
- [ ] 4.3 Aggiornare il `README.md` (voci manuali e automatiche, quando serve la rianalisi) e la nota di rollback nelle note di rilascio; allineare le delta e verificare che `openspec validate catalog-entries-inline --strict` passi.

## Workflow follow-up

- Coordinare il merge con `manual-edits-open-with-ai-tuning` e `facet-lists-expand`.
- Dopo il merge, collegare la PR all'issue #4.
- Archiviare il change dopo la revisione.
