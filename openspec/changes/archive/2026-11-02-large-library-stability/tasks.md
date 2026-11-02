# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **`D:\Foto` è solo in lettura. Non rimuovere, spostare, rinominare o modificare NULLA lì dentro**, nemmeno temporaneamente. Le prove usano `D:\photo-cat-loadtest` con collegamenti fisici e una cartella dati separata (`PHOTO_CAT_DATA_DIR`), mai la cartella dati dell'utente in `%APPDATA%`. Gli script devono rifiutarsi di scrivere o cancellare dentro `D:\Foto`.
> - **Prima di correggere, riprodurre**: il gruppo 1 misura e classifica. Se la prova a 100.000 foto mostra cause diverse da quelle nel design, aggiornare il design prima di implementare i gruppi successivi.
> - **Coordinamento**: la selezione per filtro (design §2) cambia la firma dei comandi bulk. Se `catalog-exclusions` o `similar-photo-stacks` sono già in `main`, adeguare le loro chiamate. `loading-feedback` tocca `Grid.tsx`, `TopBar.tsx` e l'avvio. `people-faces-workspace` riscrive `PeoplePage.tsx`: se è già unito, la paginazione dei volti va fatta nel nuovo modulo.
> - **Evidenza del crash**: i dati citati nel Context del design vengono dalla macchina di sviluppo. Non copiare dump o log dell'utente nel repository.

## 1. Riproduzione e misura

- [ ] 1.1 Script `scripts/loadtest/make-library.ps1` e `.sh` (collegamenti fisici fino a 100.000 file, varianti, controllo che `D:\Foto` resti invariata prima e dopo) e script di pulizia limitato a `D:\photo-cat-loadtest` (design §6). Verifica: esecuzione dello script, conteggio dei file in `D:\photo-cat-loadtest\library`, e confronto di numero di file e date di modifica di `D:\Foto` prima e dopo, identici.
- [ ] 1.2 Script `scripts/loadtest/monitor.ps1` per l'RSS dell'app e dei processi WebView2 figli, e conteggio dei long task dell'UI in modalità diagnostica. Verifica: CSV prodotto durante un import di prova di 5.000 file.
- [ ] 1.3 Prova di base sulla versione attuale: import completo di `D:\photo-cat-loadtest` con cartella dati separata; registrare esito, picco di memoria, eventuali crash, e classificare tutti i `DECODE_FAILED` per estensione e messaggio. Annotare i risultati nel design (Context e Open Questions). Verifica: tabella dei risultati nel design, con i file di esempio per ogni causa (solo percorsi, nessuna copia dei file).

## 2. Crash sempre registrati

- [ ] 2.1 Marcatore `session.lock` con aggiornamento periodico, cancellazione su `RunEvent::Exit` e avviso al riavvio con ora e memoria dell'ultima attività (design §1). Verifica: test in `pc-diag` per marcatore presente e assente, e prova manuale terminando il processo dal Task Manager.
- [ ] 2.2 Gestione di `ProcessFailed` di WebView2 (Windows) e di `web-process-terminated` (Linux): registrazione, ricarica, avviso nell'UI, pausa dei job e dialogo oltre 3 terminazioni in 5 minuti. Verifica: prova manuale terminando `msedgewebview2.exe` dal Task Manager durante un import (interfaccia ricaricata, job ancora presente, voce nel log); test UI per l'avviso dopo `app://ui-recovered`.
- [ ] 2.3 Abort nativi (filtro delle eccezioni non gestite con minidump su Windows, segnali su Unix) e comando di sviluppo che provoca un abort solo in debug. Verifica: in debug, il comando produce la riga nel log e il minidump in `logs/crash/`, e al riavvio compare l'avviso.
- [x] 2.4 Dump di WebView2 e minidump nel pacchetto diagnostico, con i limiti di numero e dimensione. Verifica: test in `pc-diag/src/bundle.rs` con una cartella Crashpad finta.

## 3. Thread principale e protocolli

- [x] 3.1 Comandi del catalogo `async` con `spawn_blocking`, e un test o lint che fallisce se compare un nuovo comando sincrono che legge il catalogo (design §3). Verifica: test che elenca i comandi sincroni ammessi; `cargo test` passa.
- [x] 3.2 `thumb://` asincrono, `Cache-Control` senza `immutable`, `request_id` e scarto dei conteggi e facet superati, con interruzione della query. Verifica: test UI con risposte fuori ordine (vince l'ultima) e test Rust dell'interruzione.

## 4. Interfaccia a memoria limitata

- [x] 4.1 Griglia con `maxPages` e paginazione bidirezionale per cursore, `gcTime` breve per le chiavi inattive. Verifica: test con `ipcMock` che, dopo lo scorrimento di 50 pagine, il numero di pagine in cache è ≤ 5 e che tornando su la pagina viene richiesta di nuovo.
- [x] 4.2 Selezione come insieme (`ids` o `filter` più eccezioni con `snapshot`) nello store, in Grid e BulkBar; `Target` nei comandi bulk risolto lato Rust a blocchi. Verifica: test Rust e UI per "Seleziona tutto su una libreria grande" e "Risultati cambiati dopo la selezione".
- [x] 4.3 `person_faces` paginato, polling sostituito da `faces://changed`, con la stessa invalidazione su evento per `list_people`. Verifica: test con `ipcMock` che non partono richieste periodiche e che l'evento invalida le query.
- [x] 4.4 Eventi dei job limitati a 250 ms, `upsertJob` mirato, risposta a `app://trim-caches`. Verifica: test di `live.ts`.

## 5. Errori raggruppati

- [x] 5.1 Buffer degli errori con `errors://batch` ogni 500 ms e scrittura a lotti nel DB (design §4). Verifica: test Rust con 10.000 errori sintetici: al massimo un evento ogni 500 ms e tutti i 10.000 record presenti in `error_log`.
- [x] 5.2 Vista "per codice" in Diagnostica con elenco dei file paginato. Verifica: test con `ipcMock` per "Migliaia di file non decodificabili" ed "Errori diversi".

## 6. Import robusto

- [x] 6.1 Limiti e decodifica senza copie inutili per PNG, WebP e TIFF, `MAX_PIXELS`, permessi corretti, anteprima RAW estratta una sola volta (design §5). Verifica: test in `pc-ingest` con un PNG sintetico enorme ("Immagine enorme") e misura dei permessi richiesti.
- [ ] 6.2 Decodificatori di riserva scelti in base al task 1.3 e cause classificate (`DecodeCause`) con messaggi it/en. Verifica: test con file di prova generati (JPEG CMYK, JPEG troncato) per gli scenari "JPEG CMYK" e "File troncato"; nuova esecuzione della classificazione su `D:\photo-cat-loadtest` con il numero di `DECODE_FAILED` residui annotato nel design.
- [x] 6.3 Concorrenza dinamica dei worker, sospensione della decodifica in pressione critica con stato del job "in pausa per memoria", e misura della memoria dei processi WebView2 nel governor. Verifica: test di `pc-governor` con una sonda simulata per gli scenari "Interfaccia che cresce" e "Pressione critica".
- [x] 6.4 Checkpoint del WAL durante l'import e `temp_store=FILE`. Verifica: test che durante un import sintetico di 20.000 elementi il file WAL resta sotto una soglia definita.

## 7. Verifica d'insieme

- [x] 7.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [ ] 7.2 Prova a 100.000 foto con `D:\photo-cat-loadtest` e cartella dati separata, monitor attivo: import, analisi, scorrimento completo della griglia, "Seleziona tutto" con un'azione bulk, apertura di Persone durante l'analisi. Verifica: criteri di accettazione del design §6 rispettati, CSV della memoria e risultati riassunti nel design, `D:\Foto` invariata (stesso controllo del task 1.1).
- [ ] 7.3 Ripetere la prova con il profilo "Leggero" e annotare i tempi. Verifica: nessuna chiusura anomala.
- [x] 7.4 Aggiornare il `README.md` (librerie grandi, dove trovare log e dump dopo un crash, script di carico), allineare le delta e verificare che `openspec validate large-library-stability --strict` passi.

## Workflow follow-up

- Coordinare il merge con `loading-feedback`, `people-faces-workspace`, `catalog-exclusions` e `similar-photo-stacks`.
- Archiviare il change dopo la revisione.
