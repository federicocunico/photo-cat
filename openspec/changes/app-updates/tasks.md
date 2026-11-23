# Tasks

D = `design.md`. Build con `CARGO_BUILD_JOBS` ≤ 4 e `CARGO_PROFILE_DEV_DEBUG=0`, un solo processo cargo alla volta.

## 1. Backend

- [x] 1.1 Modulo `updates.rs`: lettura di `releases/latest`, confronto delle versioni, scelta dell'installer per la piattaforma, URL ammessi; test su confronto, scelta dell'asset, tag non leggibili e URL rifiutati (D1)
- [x] 1.2 Lettura dello SHA-256 dal `.sha256` o dal `digest` dell'asset; download con `pc_models::download`, eventi di avanzamento, pronto e errore, annullamento; installer già scaricato riconosciuto e pulizia delle versioni vecchie; test su lettura dello SHA-256 e sulla pulizia (D2)
- [x] 1.3 `install_update`: nuova verifica del file, avvio dell'installer separato e chiusura regolare dell'app; `open_release_page` con URL ammessi; registrazione dei comandi e binding (D3)

## 2. Interfaccia

- [x] 2.1 Store `updates` alimentato dagli eventi; controllo all'avvio con la preferenza `checkUpdatesOnStart`; comando Aiuto › "Controlla aggiornamenti…" (D4)
- [x] 2.2 Notifica che si chiude, con "Novità", "Aggiorna", barra di avanzamento e "Interrompi"; su macOS e Linux "Aggiorna" apre la pagina della release; verificare con test vitest gli scenari di "Controllo delle nuove versioni", "Notifica che si può chiudere" e "Aggiornamento su macOS e Linux"
- [x] 2.3 Badge nella barra superiore (avanzamento e "Aggiornamento pronto") con il dialogo "Aggiorna ora" / "Più tardi"; verificare gli scenari di "Download dell'aggiornamento in background" e "Installazione dell'aggiornamento"
- [x] 2.4 Impostazioni › Informazioni: "Controlla gli aggiornamenti all'avvio" e "Controlla ora" con l'esito; testi it/en; verificare lo scenario "Controllo automatico disattivato"

## 3. Verifica

- [x] 3.1 `make test` in locale, `make test-linux` in Docker e build NSIS locale
- [x] 3.2 README: sezione sugli aggiornamenti
- [ ] 3.3 Prova sull'app reale (per l'utente): con la release successiva pubblicata, notifica, download con avanzamento, badge e installazione
