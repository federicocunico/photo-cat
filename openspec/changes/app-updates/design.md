## Context

photo-cat si distribuisce come release GitHub (`.github/workflows/release.yml`), con questi allegati:
- `photo-cat_<v>_x64-setup.exe`, l'installer NSIS di Windows, con il suo `.sha256`;
- `.deb` e `.tar.gz` per Linux, con i loro `.sha256`;
- il DMG per macOS, pubblicato a mano.

Il repository è pubblico, quindi l'API `releases/latest` risponde senza autenticazione. `pc_models::download` scarica già i modelli AI con ripresa (`Range` su `.part`), verifica SHA-256, tentativi con attesa crescente e annullamento. Il plugin `tauri-plugin-opener` è attivo lato Rust.

## Goals / Non-Goals

**Goals:**
- Dire all'utente che c'è una versione nuova, senza insistere: una notifica che si chiude.
- Su Windows, scaricare l'installer mentre si lavora, con avanzamento visibile, e avviarlo solo su conferma.
- Nessun file non verificato eseguito; nessun dato dell'utente inviato.

**Non-Goals:**
- Aggiornamento silenzioso o differenziale (plugin `tauri-plugin-updater` con firme e `latest.json`): richiederebbe chiavi di firma e un manifest nella release; si potrà valutare dopo.
- Download automatico su macOS e Linux.
- Canali beta: si guarda solo `releases/latest`, che esclude bozze e pre-release.

## Decisions

### D1. Fonte e confronto
- **Richiesta**: `GET https://api.github.com/repos/<owner>/<repo>/releases/latest`.
  - `owner/repo` si ricavano da `CARGO_PKG_REPOSITORY`, non scritti a mano.
  - Header: `User-Agent: photo-cat/<versione>` e `Accept: application/vnd.github+json`; timeout 10 s.
- **Confronto**: versione del tag (senza `v`) contro `CARGO_PKG_VERSION`, numerico per `major.minor.patch`. Un tag non leggibile vale "nessun aggiornamento".
- **Esito** (`UpdateInfo`):
  - versione e URL della pagina della release;
  - data di pubblicazione;
  - installer per questa piattaforma, se c'è: nome, URL, dimensione, URL del `.sha256`.
- **Installer**: su Windows x64 l'asset `photo-cat_<v>_x64-setup.exe`; altrove nessuno.
- **URL ammessi**:
  - per pagina e download, solo `https://github.com/<owner>/<repo>/releases/...`;
  - per i download reindirizzati, solo gli host di GitHub, verificati dopo i redirect.

  Così un comando IPC non può aprire o scaricare indirizzi arbitrari.

### D2. Download
- Lo SHA-256 si legge dal file `.sha256` della release: prima parola, 64 caratteri esadecimali. In mancanza si usa `digest` dell'asset (`sha256:...`). Senza nessuno dei due il download non parte.
- `pc_models::download::download` scrive `updates/photo-cat_<v>_x64-setup.exe` (con `.part` fino alla verifica), con la dimensione dell'asset.
  - L'avanzamento esce con l'evento `update://progress` (byte fatti e totali), a non più di 4 eventi al secondo.
  - A fine download escono `update://ready` (versione e percorso) oppure `update://failed` (messaggio).
- Un solo download alla volta. `cancel_update_download` imposta il flag di annullamento e il `.part` resta per la ripresa.
- **All'avvio**:
  - se nella cartella c'è già l'installer di una versione più recente di quella in uso, `check_update` lo indica (`downloaded: true`) dopo averne verificato lo SHA-256 con il `.sha256` della release; la UI mostra subito il badge;
  - installer e `.part` di versioni uguali o precedenti si eliminano.

### D3. Installazione
- `install_update(version)` controlla che il file esista e corrisponda di nuovo allo SHA-256 memorizzato alla verifica.
- Avvia l'installer come processo separato (Windows: `DETACHED_PROCESS`, senza console) e chiude l'app con `app.exit(0)`, che passa da `RunEvent::Exit` e segna la sessione come chiusa regolarmente.
- L'installer NSIS di Tauri chiude o attende le istanze aperte; l'utente prosegue con le sue pagine.
- L'installer non si avvia in modalità silenziosa: l'utente vede che cosa succede e può annullare.

### D4. Interfaccia
- **Store `updates`** (zustand): `idle`, `available`, `downloading`, `ready`, `failed`, più `dismissed` per la sessione e l'esito dell'ultimo controllo manuale. Gli eventi del backend aggiornano lo store.
- **Notifica** (`UpdateNotice`), in basso a destra sopra gli avvisi:
  - testo "È disponibile photo-cat 0.5.4 (in uso 0.5.3)", con "Novità" (pagina della release), "Aggiorna" e ✕;
  - durante il download, barra determinata con percentuale e MB, più "Interrompi";
  - ✕ nasconde la notifica, ma il download continua.
- **Badge** nella barra superiore, prima dell'indicatore delle attività:
  - durante il download, "Aggiornamento 42%" con la barra;
  - a download finito, "Aggiornamento pronto" (accento).

  Il clic sul badge apre il dialogo "Installare photo-cat 0.5.4 ora?", con "Più tardi" e "Aggiorna ora".
- **macOS e Linux**: "Aggiorna" apre la pagina della release con `open_release_page`. Nessun badge.
- **Controllo**: all'avvio, 5 s dopo che l'app è pronta e l'introduzione è chiusa, se `checkUpdatesOnStart` (preferenza dell'interfaccia, in `ui-prefs.json` come le altre) è attiva. Il controllo su richiesta arriva da Aiuto › "Controlla aggiornamenti…" e da Impostazioni › Informazioni; mostra sempre la notifica, anche se era stata chiusa, e dice l'esito con un avviso.

## Risks / Trade-offs

- [Limite dell'API di GitHub: 60 richieste/ora per IP senza token] → un controllo all'avvio e quelli manuali sono ben sotto il limite. Un 403 o un 429 durante il controllo manuale si spiega come "Troppi controlli, riprova più tardi".
- [Installer non firmato: SmartScreen può avvisare] → l'utente lo vede comunque oggi scaricandolo a mano. La notifica non promette un'installazione silenziosa.
- [Download di 470 MB su connessioni lente] → ripresa, annullamento e badge persistente. Il download parte solo su richiesta, mai da solo.
- [Integrità] → SHA-256 dalla release, sullo stesso host HTTPS. Protegge dalla corruzione e dagli scambi di file, non da un account GitHub compromesso, che è fuori dal perimetro di questa change.

## Migration Plan

Nessuna migrazione. La cartella `updates` si crea al primo download. La preferenza nuova vale `true` se assente.
