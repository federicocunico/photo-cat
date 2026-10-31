# Tasks

Il branch `feat/linux-support` contiene già, non committate, la prima versione di `probe_library` (`pc-models/src/runtime.rs`) e di `open_logs` multipiattaforma (`app/src-tauri/src/diagnostics.rs`). I task 1.1 e 4.1 le completano con test e rifiniture invece di riscriverle. Ogni gruppo lascia `cargo build --workspace` funzionante su Linux e Windows.

## 1. Avvio robusto del runtime ONNX

- [x] 1.1 Completare `probe_library` (design §1) con un commento che rimandi al deadlock di `ort` rc.12, e aggiungere unit test in `pc-models` che, con un file non caricabile (per esempio un file di testo rinominato `.so`/`.dll`) e con un file inesistente, verificano che `init` restituisca `ModelUnavailable` entro un timeout invece di bloccarsi. Verifica: `cargo test -p pc-models` passa.
- [x] 1.2 Correggere l'errore clippy preesistente `unnecessary_sort_by` in `app/src-tauri/src/commands.rs:61` e verificare che `cargo clippy --workspace --all-targets -- -D warnings` passi su Linux.

## 2. Librerie native per piattaforma

- [x] 2.1 Introdurre il modulo che restituisce cartella asset (`win-x64`/`linux-x64`) e nome file del runtime ONNX per piattaforma (design §2), con unit test sui valori per la piattaforma corrente. Verifica: `cargo test -p pc-core` (o il crate scelto) passa.
- [x] 2.2 Usare il modulo in `models::asset_paths` (rinominando `runtime_dll` in `runtime_lib`), in `runtime::dev_dll_path` (rinominato `dev_runtime_path`) e nei test `model-tests`. Verifica: `grep -rn "win-x64/onnxruntime.dll" crates app/src-tauri/src` non trova più occorrenze nel codice Rust.
- [x] 2.3 Aggiungere `assets/runtime/linux-x64/libonnxruntime.so` da `onnxruntime-linux-x64-1.24.4.tgz` ufficiale, con licenze in `assets/licenses/` e una riga in `assets/README.md` (fonte, versione, licenza). Verifica: `sha256sum` del file corrisponde a quello estratto dal tgz e il README elenca il file.
- [x] 2.4 Abilitare `model-tests` su Linux, togliendo `windows` dal `cfg` dove il test non dipende da DirectML, e verificare che `cargo test -p pc-models -p pc-ai --features model-tests` carichi il runtime Linux e superi il self-test su CPU.
- [x] 2.5 Verificare con `make dev` su questa macchina (Ubuntu, Wayland) che la finestra si apra senza variabili d'ambiente, che la schermata Modelli AI mostri i modelli verificati e che come provider sia disponibile solo la CPU, senza DirectML né CoreML.

## 3. HEIC su Linux

- [x] 3.1 Rendere esplicito in `pc_ingest::heif` che fuori da Windows si usa la libheif di sistema. Il messaggio di libreria assente o di simbolo mancante deve suggerire di installare `libheif1` e indicare la versione minima. Verifica: test in `crates/pc-ingest/tests/heif.rs` con `PHOTOCAT_HEIF_LIBRARY` puntato a un file inesistente, che controlla il codice `UnsupportedFormat` e il testo del suggerimento.
- [x] 3.2 Misurare la versione minima di libheif richiesta dai simboli usati in `heif.rs` e verificare la decodifica di un HEIC di test con la libheif di Ubuntu 24.04 (`apt install libheif1 libheif-plugin-libde265`). Annotare il risultato in Open Questions del design e nel README.

## 4. Integrazione col desktop e organizzazione file

- [x] 4.1 Far sì che `open_logs` includa il percorso della cartella nell'errore quando il comando di apertura non parte (spec `diagnostics-logging`), e verificare manualmente su Linux che "Apri cartella log" apra il file manager. Verifica aggiuntiva: il test UI esistente `DiagnosticsPage.test.tsx` continua a passare (`npm --prefix app/ui test`).
- [x] 4.2 Trasformare `developer_mode` in `Option<bool>` e adattare `decide()` (design §7), con unit test su `decide()` per: symlink negato su Windows senza Modalità sviluppatore (`symlink_privilege`), symlink negato su Linux (`symlink_denied`, testo senza "Windows"), symlink consentito. Verifica: `cargo test -p pc-organize` passa.
- [x] 4.3 Eseguire `make test` completo su Linux e correggere eventuali test che falliscono solo su questa piattaforma (percorsi, maiuscole/minuscole, permessi). Verifica: `make test` termina con successo.

## 5. Build e archivio Linux

- [x] 5.1 Creare `app/src-tauri/tauri.linux.conf.json` con `bundle.targets: ["deb"]` e risorse `linux-x64` mappate su `runtime/` (design §3), e verificare che `npm --prefix app/ui run tauri -- build --bundles deb` produca lo staging `bundle/deb/*/data/usr/{bin,lib/photo-cat}` con `runtime/libonnxruntime.so` e `models/`.
- [x] 5.2 Scrivere `scripts/package-linux.sh`, che raccoglie in `target/release/dist/` il `.deb` e l'archivio `photo-cat-<versione>-linux-x64.tar.gz` ricavato dallo staging, ciascuno con il proprio `.sha256` con fine riga LF (design §5). Verifica: `sha256sum -c` passa su entrambi e l'eseguibile estratto dall'archivio in una cartella temporanea si avvia e trova le risorse (schermata Modelli AI con i modelli verificati).
- [ ] 5.3 Verificare il `.deb`: `dpkg-deb -I` mostra le dipendenze (WebKitGTK 4.1 ed eventualmente `libheif1`, design §5). Dopo `sudo apt install ./photo-cat_<versione>_amd64.deb` su questa macchina, photo-cat compare nel menu delle applicazioni, si avvia da lì con i modelli verificati e `sudo apt remove photo-cat` lo rimuove senza toccare `~/.local/share/photo-cat`.
- [x] 5.4 Aggiornare il `Makefile`: `make build` su Linux chiama `scripts/package-linux.sh`, su Windows resta NSIS, e `help` descrive entrambi. Verifica: `make build` su Linux produce `.deb`, archivio e checksum in `target/release/dist/`.
- [x] 5.5 Aggiornare `README.md` con i prerequisiti Linux per compilare (pacchetti apt di design §6), l'installazione del `.deb`, l'uso dell'archivio con le dipendenze da installare a mano (`libwebkit2gtk-4.1-0`, opzionale `libheif1`). Verifica: i comandi documentati funzionano da un clone pulito su questa macchina.

## 6. CI e release

- [x] 6.1 Aggiungere a `.github/workflows/ci.yml` un job `linux` su `ubuntu-22.04` (dipendenze apt, `rust-cache` con chiave separata, fmt, clippy, `cargo test --workspace`, build dell'archivio con upload dell'artefatto) e aggiornare il commento iniziale. Verifica: pipeline verde su una PR del branch.
- [ ] 6.2 Aggiungere a `release.yml` un job `build-linux` accanto al job Windows. Il controllo tag/versione resta sul runner Windows, e `publish` scarica gli artefatti di entrambi e li allega alla release. Verifica: `act` oppure un tag di prova su un fork o con `workflow_dispatch` produce una release, o una bozza, con EXE, `.deb`, archivio Linux e i tre `.sha256`.
- [x] 6.3 Allineare le delta di `specs/` a quanto implementato (nomi dei file d'archivio, dipendenze, messaggi) e verificare che `openspec validate linux-support --strict` passi.
- [ ] 6.4 Estrarre la preparazione comune dei job di build (toolchain Rust, `rust-cache`, Node, `npm ci`, pacchetti apt solo su Linux) in un'azione composita `.github/actions/setup-build`, usata dai job Windows e Linux di `ci.yml` e `release.yml`, senza cambiare cosa produce ciascun job. Verifica: run CI verde su entrambi i sistemi con gli stessi artefatti di prima.
- [ ] 6.5 Nella release, riusare i pacchetti della run CI del push su `main` per lo stesso commit (attesa con `gh run watch`, download degli artefatti, checksum calcolati nella release), ricompilando solo se la run manca, è fallita o gli artefatti sono scaduti (design §6). Verifica: con il tag di prova del task 6.2 la release contiene gli stessi file e il log mostra il riuso; una seconda prova senza run CI disponibile ricompila.

## 7. Valutazione GPU su Linux

- [x] 7.1 Verifica a tempo (al massimo una giornata) del provider WebGPU di ONNX Runtime 1.24 su questa macchina: registrazione del plugin EP tramite `ort` rc.12, esecuzione di YuNet e CLIP quantizzato, confronto dei tempi con la CPU. Consegna: esito e numeri nella sezione Open Questions di `design.md`, e, se l'esito è positivo, la proposta di un change separato. Nessuna modifica al codice di produzione.

## 8. Verifica d'insieme

- [x] 8.1 Su Linux, partendo da una cartella dati vuota (`PHOTO_CAT_DATA_DIR`), importare una cartella di foto di prova con JPEG e HEIC, completare l'analisi AI su CPU, creare un'organizzazione con symlink ed esportare il CSV. Verifica: tutte le fasi completate senza errori inattesi in diagnostica. Esito: import reale con AI ed export CSV verificati a mano su Ubuntu 26.04; organizzazione con symlink e annullamento coperti su Linux da `link_preflight_and_undo_never_remove_link_targets` (`make test`); decodifica HEIC dal task 3.2.

## Workflow follow-up

- Verifica su Windows, non a carico di questo change (deciso il 2026-10-31; la fa chi lavora su Windows): su Windows, verificare che l'installer NSIS della CI si installi e che l'AI giri ancora con DirectML (nessuna regressione).
- Archiviare `linux-support` dopo la review e verificare il risultato con `openspec validate --specs`.
