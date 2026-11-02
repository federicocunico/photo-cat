# Proposal

## Why

L'utente non capisce quando l'app sta caricando e quando ha finito. Nel codice (`main`):

- non esiste un componente comune di caricamento: c'è solo l'icona `Icon.Spinner`, e nessuna parte della UI usa lo stato delle query in corso;
- quasi tutte le liste usano `data ?? []`, quindi **durante il caricamento mostrano lo stato vuoto** ("nessuna sorgente", "nessuna persona"…), che è un'informazione sbagliata. È il caso di Sidebar (sorgenti, cartelle, cataloghi), Import, Job, Diagnostica, Persone, Identità, Tag, Cataloghi, Modelli e Inspector;
- la griglia mostra un indicatore solo per le pagine successive, non per il primo caricamento, e il conteggio parte dal numero di elementi già caricati;
- molte azioni non mostrano che stanno lavorando, e in Tag, Identità, Inspector, Cataloghi, Sequenza e menu contestuale i pulsanti non vengono nemmeno disabilitati, quindi un doppio clic invia due volte;
- l'indicatore dei job nella barra superiore gira sempre, anche senza job attivi;
- all'avvio non c'è una schermata di caricamento: la finestra compare solo dopo migrazioni e caricamento del runtime AI (fino a 30 secondi di attesa), poi la Libreria appare e l'onboarding può comparirle sopra.

## What Changes

- **Componenti comuni** di caricamento (indicatore in linea, segnaposto a scheletro per liste e griglie, sovrapposizione per pannelli) e una regola valida in tutta l'app: finché un dato non è arrivato si mostra il caricamento, mai lo stato vuoto; quando arriva vuoto si mostra lo stato vuoto; quando fallisce si mostra l'errore con "Riprova".
- **Aggiornamenti in background visibili**: quando un dato già mostrato si sta aggiornando, una barra sottile in cima al pannello lo segnala senza nascondere il contenuto.
- **Azioni**: ogni pulsante che avvia un'operazione mostra lo stato "in corso" (spinner nel pulsante, testo invariato), viene disabilitato fino alla fine e non può essere inviato due volte; le operazioni oltre ~1 secondo mostrano un avviso con l'esito.
- **Indicatore globale**: nella barra superiore un indicatore discreto quando ci sono richieste al motore in corso da più di 300 ms, distinto dall'indicatore dei job; lo spinner dei job gira solo quando ci sono job attivi.
- **Griglia**: segnaposto delle miniature durante il primo caricamento, conteggio "in calcolo…" finché il totale non è noto, indicatore quando un filtro sta ricalcolando i risultati.
- **Avvio**: una schermata di avvio con le fasi (apertura del catalogo, migrazione con backup, caricamento del runtime AI) e l'eventuale errore, al posto della finestra che compare in ritardo; la Libreria o l'onboarding compaiono solo quando la configurazione è letta.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `desktop-shell`: "Indicatori globali" (indicatore delle richieste in corso, spinner dei job solo con job attivi); nuovi requisiti "Stati di caricamento", "Azioni in corso" e "Schermata di avvio".

## Impact

- **UI**:
  - nuovi componenti in `app/ui/src/components/feedback/` (`Spinner`, `Skeleton`, `LoadingState`, `RefreshBar`, `BusyButton`, `Toast`);
  - un helper `QueryView` per il trittico caricamento / vuoto / errore;
  - uno store per le richieste IPC in corso (wrapper in `lib/ipc.ts`);
  - modifiche a tutti i moduli in `app/ui/src/modules/*` e a `TopBar.tsx`, `App.tsx`;
  - i18n it/en.
- **Rust**: `app/src-tauri/src/lib.rs`: la finestra si mostra subito con la schermata di avvio. Migrazioni e caricamento del runtime passano in un'inizializzazione asincrona che emette eventi di fase, e i comandi che dipendono dallo stato attendono che sia pronto.
- **Dipendenze**: nessuna nuova.
- **Coordinamento**: `large-library-stability` sposta alcuni comandi fuori dal thread principale e introduce limiti alle liste. `catalog-exclusions` e `people-faces-workspace` possono servirsi del `Toast` creato qui (vedi note in `tasks.md`).
