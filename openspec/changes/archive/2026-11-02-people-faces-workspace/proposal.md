# Proposal

## Why

Oggi "Persone" (gruppi di volti di una libreria) e "Identità" (archivio delle persone note a livello di app) sono due moduli separati. Per dare un nome a qualcuno l'utente passa dall'uno all'altro, e la gestione dei volti trovati automaticamente è scomoda:

- il modulo Persone è una griglia di schede ricaricata ogni 5 secondi;
- l'unione si fa scegliendo il bersaglio da un menu a tendina;
- non ci sono suggerimenti di gruppi che sono probabilmente la stessa persona, quindi restano doppioni;
- non c'è modo di ignorare una persona che non interessa (passanti, sconosciuti ricorrenti);
- i volti mossi, sfocati o poco visibili entrano nei gruppi: la qualità salvata è solo il punteggio del rilevatore, non c'è una misura di nitidezza;
- "Apri in Persone" apre solo il modulo, non la persona né il volto di partenza;
- "Tutti i volti" carica l'intera tabella dei volti, e con librerie grandi è uno dei fattori di consumo di memoria (vedi `large-library-stability`).

L'utente chiede di unire i due moduli in un'unica vista per gestire persone e volti trovati automaticamente. Chiede anche che siano solo le sue scelte a trasformare un gruppo in una persona, e di ripensare l'interfaccia.

## What Changes

- **Un solo modulo "Persone"** con tre sezioni:
  - **Da rivedere**: gruppi di volti trovati automaticamente, con suggerimenti di nome e di unione;
  - **Persone**: le identità decise dall'utente, valide per tutte le librerie, con i campi dell'attuale editor delle identità;
  - **Ignorati**.

  Layout a tre colonne: elenco, volti, scheda. Il modulo "Identità" scompare dalla navigazione.
- **Solo l'utente crea persone**: un gruppo diventa persona solo quando l'utente gli dà un nome (scegliendo o creando un'identità). Il riconoscimento automatico continua ad associare i volti nuovi alle persone esistenti.
- **Unione e anti-doppioni**:
  - unione di gruppi o persone dall'elenco (selezione multipla) o dalla scheda;
  - suggerimenti "Probabilmente la stessa persona" calcolati dai centroidi dei gruppi, con "Sono persone diverse" ricordato per sempre.
- **Ignora persona**: un gruppo ignorato sparisce da "Da rivedere", dai filtri, dall'Inspector e dai suggerimenti. I nuovi volti simili vi confluiscono silenziosamente. Si ripristina dalla sezione "Ignorati".
- **Qualità dei volti**:
  - per ogni volto si calcola un punteggio di qualità (nitidezza del ritaglio, dimensione, confidenza del rilevatore, frontalità);
  - i volti sotto la soglia "nitidezza minima" sono scartati da gruppi e suggerimenti, ma restano consultabili e recuperabili ("Volti scartati per qualità");
  - copertine e volti di riferimento usano i migliori.
- **Apri in Persone dalla foto**: dall'Inspector (persona o volto non riconosciuto) e dagli altri punti dell'app si apre direttamente la persona o il gruppo del volto, con il volto di quella foto evidenziato e in vista.
- **Prestazioni**:
  - elenchi e volti paginati e virtualizzati;
  - aggiornamento su evento invece che ogni 5 secondi;
  - niente più elenco "Tutti i volti" unico, sostituito da "Volti singoli" paginato.
- **Mockup** della nuova interfaccia in `mockups/People.html`, con i casi principali e le scorciatoie da tastiera.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `face-recognition`:
  - "Rilevamento volti" (punteggio di qualità e soglia di nitidezza);
  - "Raggruppamento in persone" (gruppi da rivedere, suggerimenti di unione);
  - "Identificazione delle persone" (persona solo per scelta dell'utente);
  - "Correzione dei raggruppamenti" (unione multipla, "persone diverse");
  - nuovi requisiti "Persone ignorate", "Volti scartati per qualità" e "Apertura diretta di una persona".
- `identity-registry`:
  - "Gestione delle identità" (nella sezione Persone del modulo unico);
  - "Collegamento con le persone della libreria" (collegamento come esito del "Chi è?").
- `desktop-shell`: "Moduli di navigazione" (Identità confluisce in Persone); nuovo requisito "Revisione delle persone".
- `ai-classification`: "Soglie dell'automatismo configurabili" (nitidezza minima dei volti).

## Impact

- **Rust**:
  - `pc-ai`:
    - nuovo `quality.rs` (nitidezza, condiviso con `similar-photo-stacks`);
    - in `face_analysis.rs`, il calcolo della qualità sul ritaglio allineato e della frontalità dai punti di riferimento;
    - in `cluster.rs`, l'esclusione dei volti scartati, i gruppi ignorati come attrattori e i vincoli "persone diverse";
    - nuovo calcolo dei suggerimenti di unione da `person_centroid`.
  - `pc-store`:
    - migrazione di schema: `face.sharpness`, `face.quality_score`, `person.ignored_at`, tabella `person_not_same`, indici per la paginazione;
    - query paginate in `people.rs`.
  - `app/src-tauri/src/people.rs` e `identities.rs`: comandi paginati, ignora/ripristina, suggerimenti, `locate_face`.
- **UI**:
  - nuovo modulo `app/ui/src/modules/people/` (elenco, griglia volti virtualizzata, scheda, dialoghi) che sostituisce `PeoplePage.tsx` e `identities/IdentitiesPage.tsx`, riusando `WhoIs`, l'editor e `Transfer`;
  - `registry.ts`, `store/layout.ts` (`openPeople(target)`), `Inspector.tsx`, Impostazioni › Soglie AI;
  - i18n it/en.
- **Dati**: migrazione automatica; i dati esistenti (persone, identità, correzioni) restano validi. La qualità dei volti esistenti si calcola con un job di manutenzione dai ritagli già salvati, senza rianalizzare le foto.
- **Coordinamento**: `large-library-stability` (paginazione e niente polling), `loading-feedback` (stati di caricamento), `similar-photo-stacks` (`quality.rs` e lo stesso requisito sulle soglie), `catalog-exclusions` (numero di migrazione).
