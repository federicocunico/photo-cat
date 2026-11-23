# Proposal

## Why

Le nuove versioni di photo-cat escono su GitHub (0.5.2 e 0.5.3 nello stesso giorno), ma l'app non lo dice. Chi la usa deve ricordarsi di aprire la pagina delle release, scaricare a mano un installer da 470 MB e lanciarlo con l'app ancora aperta. Così molti restano su versioni vecchie, con difetti già corretti.

## What Changes

- **Controllo delle versioni**: all'avvio (disattivabile) e su richiesta ("Controlla aggiornamenti…" nel menu Aiuto e in Impostazioni › Informazioni) l'app legge l'ultima release pubblicata su GitHub. Se è più recente di quella in uso, mostra una **notifica che si può chiudere**: "È disponibile photo-cat 0.5.4", con "Novità", "Aggiorna" e la chiusura.
- **Download in background** (Windows): "Aggiorna" scarica l'installer della release in background, con una **barra di avanzamento visibile** nella notifica e nella barra superiore. Il download riprende se la connessione cade e si può interrompere. Il file si verifica con lo SHA-256 pubblicato nella release prima di essere usato.
- **Installazione**: a download finito compare nella barra superiore un **badge "Aggiornamento pronto"**. Il badge chiede se aggiornare ora: "Aggiorna ora" avvia l'installer e chiude photo-cat, così l'utente prosegue con l'installazione; "Più tardi" lascia il badge.
- **macOS e Linux**: la notifica è la stessa, ma "Aggiorna" apre la pagina della release, perché DMG e pacchetti .deb si installano con gli strumenti del sistema.
- **Pulizia**: dopo l'aggiornamento gli installer di versioni già installate vengono eliminati.

## Capabilities

### New Capabilities
- `app-updates`: controllo delle nuove versioni, notifica, download verificato dell'installer con avanzamento e avvio dell'installazione.

### Modified Capabilities
- `app-about`: la sezione Informazioni ospita l'impostazione del controllo automatico e il comando "Controlla ora".

## Impact

- Backend (`app/src-tauri`): nuovo modulo `updates.rs`.
  - Comandi `check_update`, `download_update`, `cancel_update_download`, `install_update`.
  - Eventi di avanzamento.
  - Riuso di `pc_models::download` per ripresa, verifica e annullamento.
  - Cartella `updates/` nei dati dell'app.
- Rete: una richiesta all'API pubblica di GitHub per controllo; nessun dato dell'utente inviato (solo l'User-Agent "photo-cat/versione").
- UI:
  - store `updates`, notifica, badge nella barra superiore e dialogo di conferma;
  - voce di menu Aiuto e controlli in Impostazioni › Informazioni;
  - testi it/en.
- Test: Rust (confronto delle versioni, scelta dell'installer, lettura dello SHA-256, URL ammessi, pulizia) e vitest (notifica, avanzamento, badge, installazione, piattaforme senza download).
