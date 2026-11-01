# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Origine**: issue GitHub #7 "Eliminare importazione" e #8 "Aggiornare il contenuto correlato ad una importazione". Rimozione e riscansione esistevano già: il change riguarda ciò che mancava o era incompleto (design, Context). Nel messaggio finale citare le issue, ma non chiuderle senza conferma dell'utente.
> - **Prima di iniziare**: verificare sull'app reale il comportamento attuale descritto nel Context del design (miniature rimaste su disco dopo la rimozione, file modificato non rianalizzato). Se qualcosa risulta diverso, aggiornare il design prima di scrivere codice.
> - **Conflitti con `manual-edits-open-with-ai-tuning`** (branch `feat/user-feedback-tuning`, implementazione in corso): aggiunge l'albero delle cartelle in `Sidebar.tsx` e `LibraryFilter.folder`. Controllare se è già unito in `main` e, se sì, fare il rebase e riusare il suo confronto tra percorsi. Anche `facet-lists-expand` modifica `Sidebar.tsx`.
> - **Mai toccare i file originali sul disco**: ogni test di rimozione deve verificare che i file sorgente siano intatti.
> - **Nessuna migrazione di schema**: se un task sembra richiederla, fermarsi e chiedere.

## 1. Rimozione completa

- [ ] 1.1 Routine di pulizia dei file derivati (miniature; i ritagli dei volti stanno nel database) usata da `media::delete` e dalla rimozione della sorgente, con gli errori di file system registrati in diagnostica (design §1). Verifica: test in `crates/pc-store/tests/` o `pc-ingest/tests/import.rs` che, dopo la rimozione bulk di 3 foto, i loro file in `thumbs` non esistono più e i file originali sono intatti.
- [ ] 1.2 Comando `source_removal_summary` (foto, foto modificate, cataloghi coinvolti, persone con nome che restano senza foto, job attivi). Verifica: test Rust per lo scenario "Persone coinvolte" e i conteggi di cataloghi e foto modificate.
- [ ] 1.3 `remove_source` rivisto: annullamento dei job della sorgente con attesa, cancellazione in transazione, eliminazione delle persone senza nome rimaste senza volti, pulizia dei file. Verifica: test per "Importazione fatta per errore", "Rimozione durante l'import" (import avviato e sorgente rimossa a metà: nessun elemento ricompare) e "Persone senza nome".
- [ ] 1.4 Pulizia una tantum delle miniature orfane all'avvio, in background e marcata in `settings_kv`. Verifica: test che crea miniature non referenziate e controlla che siano cancellate una sola volta e che quelle referenziate restino.

## 2. Aggiornamento

- [ ] 2.1 Contatori e id per nuovi, modificati, spostati, mancanti, tornati presenti e invariati in `ImportSummary` (design §2), salvati nel risultato del job. Verifica: test in `crates/pc-ingest/tests/import.rs` per "Cartella riorganizzata" (30 nuove, 12 spostate, 5 mancanti) e "Nessuna modifica".
- [ ] 2.2 Parametro `folder` del job `import`, con `mark_missing` limitato alla cartella e rifiuto delle cartelle esterne alla sorgente (design §3). Verifica: test per "Sottocartella modificata" e per una cartella esterna rifiutata con `invalid_input`.
- [ ] 2.3 Rianalisi dei file con contenuto cambiato, conservando i dati manuali (design §4). Verifica: test per "Foto ritoccata" (nuova miniatura, analisi in coda, etichetta manuale, tag e cataloghi invariati) e per "File mancante tornato presente".

## 3. Mancanti

- [ ] 3.1 Job `relink_missing(source_id, old_folder, new_folder)` con ricerca per percorso relativo e poi per impronta univoca, e riepilogo (design §5). Verifica: test per "Cartella spostata su un altro disco" (catalogazioni, tag, cataloghi e persone conservati), "File rinominati nella nuova cartella" e due copie identiche che restano mancanti come casi ambigui.
- [ ] 3.2 Comando `remove_missing(source_id)` con conferma e pulizia dei file derivati. Verifica: test per "Rimozione dei mancanti" (5 file rimossi, gli altri invariati).

## 4. Interfaccia

- [ ] 4.1 Componente `SourceMenu` nella barra laterale della Libreria con "Aggiorna", "Aggiorna cartella…", "Rimuovi dalla libreria…", "Apri in Import", e "Ricollega mancanti…"/"Rimuovi mancanti…" quando servono; chiavi i18n it/en. Verifica: test con `ipcMock` in `LibraryPage.test.tsx` per "Rimozione dalla Libreria" e per la comparsa delle voci sui mancanti.
- [ ] 4.2 Dialogo di rimozione con riepilogo, condiviso tra Libreria e pagina Import, al posto di `window.confirm`; "Riscansiona" rinominato "Aggiorna". Verifica: test in `ImportPage.test.tsx` che il dialogo mostra foto, cataloghi, persone e job, e che Annulla non chiama `removeSource`.
- [ ] 4.3 Riepilogo dell'aggiornamento nella pagina Import ("Ultimo aggiornamento") e nella schermata Job, con collegamenti alla libreria filtrata per gruppo, aggiungendo `LibraryFilter.ids` in `pc-store/src/query.rs`. Verifica: test con `ipcMock` che un clic su "5 mancanti" apre la Libreria con quei 5 elementi.
- [ ] 4.4 Dialogo "Ricollega mancanti…" con cartella precompilata (comune ai mancanti), scelta della nuova cartella e risultato. Verifica: test con `ipcMock` del flusso completo e del messaggio per i file ancora mancanti.

## 5. Verifica d'insieme

- [ ] 5.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 5.2 Prova sull'app reale: importare per errore una cartella e rimuoverla durante l'import (controllare cache delle miniature e file originali); riorganizzare una cartella (aggiunte, spostamenti, cancellazioni, un file ritoccato) e aggiornarla controllando il riepilogo; spostare una cartella su un altro percorso e ricollegarla; rimuovere i mancanti. Verifica: tutti i passi riusciti, nessun errore nuovo in Diagnostica.
- [ ] 5.3 Aggiornare il `README.md` (gestione delle importazioni: aggiornare, ricollegare, rimuovere), allineare le delta e verificare che `openspec validate import-management --strict` passi.

## Workflow follow-up

- Coordinare il merge con `manual-edits-open-with-ai-tuning` e `facet-lists-expand` (`Sidebar.tsx`).
- Dopo il merge, collegare la PR alle issue #7 e #8.
- Archiviare il change dopo la revisione.
