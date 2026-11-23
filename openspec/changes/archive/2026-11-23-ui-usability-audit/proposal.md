# Proposal

## Why

L'audit completo è in `audit.md`. È stato fatto sull'app reale (build 0.5.1), con un catalogo di prova tratto da `D:\Foto` e con misure prese dall'interfaccia. Dopo la revisione di novembre l'interfaccia è cresciuta di nuovo una change alla volta (unione multipla, assegnazione in blocco, Varianti RAW, Analisi AI), e il modulo Persone è diventato difficile da usare. Ci sono:
- due selezioni indipendenti nella stessa schermata;
- cinque verbi diversi per spostare volti;
- 11 comandi su tre righe sopra i volti, 5 dei quali disattivati;
- "gruppo" con tre significati;
- un elenco che dedica metà dell'altezza ai controlli.

A questo si aggiungono difetti visibili: l'avanzamento che salta il gruppo mostrato, una riga che dice "non collegata" per una persona collegata, nomi interni delle fasi, Esporta che fa scorrere l'intera finestra. C'è anche una richiesta degli utenti: mostrare la versione in modo esplicito, con build, licenze di terze parti e autori, e avere una barra dei menu da app desktop.

## What Changes

- **Persone riorganizzato** (mockup `Main`, `PeopleSelection`, `PeoplePerson`):
  - schede "Da rivedere / Persone / Ignorati" su una riga nell'intestazione della pagina;
  - elenco con un campo di ricerca e un menu "Ordina e filtra" al posto di chip, select e pulsanti;
  - **una sola selezione per schermata**: un clic apre un gruppo; per sceglierne più d'uno si usano Ctrl/Maiusc o la casella che compare al passaggio del mouse, e compare una barra in fondo all'elenco;
  - sopra i volti solo i comandi di vista (Ordina, Vista). Selezionando dei volti, una **barra delle azioni** prende il posto dell'intestazione: "Sposta in…", "Non è questa persona", "Altro" (Nuovo gruppo, Usa come copertina, Rimuovi volti…). La barra non va mai a capo;
  - **verbi unificati**: "Sposta in…" per i volti, "Unisci a…" per gruppi e persone. Un solo selettore con ricerca, che mostra persone e gruppi senza nome con conteggi e miniature, e permette di creare una persona nuova;
  - scheda con una decisione per volta: il suggerimento diventa "È Gioia?" con i due volti affiancati e Sì / No. Poi "Chi è?", poi le azioni secondarie;
  - scheda di una persona con dettagli salvati subito, senza "Salva"; riconoscimento espresso come sensibilità; "Unisci a…", "Esporta" ed "Elimina persona…" nel menu ⋯;
  - niente punteggi 0–1 né chip colorati per i conteggi.
- **Glossario esteso**: *persona* (al posto di "identità" nei testi), *persona senza nome* (gruppo di volti), *cerchia* (al posto del campo "Gruppo" dell'identità), *Sposta in…* e *Unisci a…*. Gli avvisi dicono sempre che cosa è stato fatto e su che cosa.
- **Barra dei menu** (nuova, mockup `MenuBar`):
  - menu File, Modifica, Vista, Vai, Strumenti, Aiuto nella barra superiore, alimentati dal registro dei comandi, con le scorciatoie accanto e i comandi disattivati che spiegano il motivo;
  - Alt o F10 portano il focus sulla barra;
  - su macOS gli stessi menu stanno nella barra di sistema;
  - il menu "?" della barra superiore viene sostituito dal menu Aiuto.
- **Informazioni su photo-cat** (nuova finestra completa, mockup `About`):
  - versione, build (commit, data, configurazione), sistema, motore della finestra, acceleratore AI, percorso dei dati, autori, repository, licenza;
  - **licenze di terze parti** consultabili con ricerca (modelli, librerie native, interfaccia, componenti Rust), generate alla build;
  - comando "Copia informazioni";
  - la stessa scheda è in Impostazioni › Informazioni.
- **Coerenza della shell**:
  - titolo di pagina sempre nello stesso punto;
  - scorre solo il contenuto, mai la finestra; la barra di stato sta sempre in basso, su una riga;
  - niente gruppo di navigazione con una sola voce: "Analisi AI" passa in Organizza, nello stesso ordine e con le stesse scorciatoie;
  - "Varianti RAW" senza il modello della fotocamera nel nome della voce;
  - i suggerimenti al primo uso non coprono comandi.
- **Fondamenta visive**:
  - scala tipografica ridotta a quattro corpi;
  - un solo stile di intestazione di sezione;
  - monospazio solo per percorsi e codice;
  - niente etichette in maiuscolo;
  - stati selezionati presi dai componenti condivisi.
- **Difetti da correggere** (F1–F9 in `audit.md`):
  - ordine di avanzamento e frecce in Persone;
  - riga della persona;
  - passo "Crea un catalogo" dei Primi passi;
  - nomi delle fasi;
  - scorrimento di Esporta;
  - barra di stato;
  - plurali;
  - formato dei decimali;
  - conteggio sulla riga "+ Nuovo catalogo".
- **Ritocchi degli altri moduli** (Primo avvio, Attività, Sequenza, Tag, Libreria, Impostazioni), elencati in `audit.md` §5–§6.

Nessuna modifica tocca dati o file. Le scorciatoie Ctrl+1…9 e l'ordine dei moduli non cambiano.

## Capabilities

### New Capabilities

- `application-menu`: barra dei menu dell'app (contenuto, tastiera, comportamento su macOS) collegata al registro dei comandi.
- `app-about`: informazioni sull'app (versione, build, sistema, autori, licenza, licenze di terze parti, copia delle informazioni) nella finestra Informazioni e in Impostazioni.

### Modified Capabilities

- `desktop-shell`:
  - modificati:
    - "Moduli di navigazione": gruppi senza "AI", stesso ordine e scorciatoie, voce "Varianti RAW";
    - "Stile visivo": scala di quattro corpi, un solo stile di intestazione di sezione;
    - "Impostazioni": sezione Informazioni, cache delle anteprime RAW in Avanzate;
    - "Revisione delle persone": schede nell'intestazione, elenco subito visibile;
  - nuovi:
    - "Scorrimento e barra di stato";
    - "Intestazione di pagina";
    - "Apertura e selezione in Persone";
    - "Ordine di avanzamento in Persone".
- `face-recognition`: nuovi requisiti "Comandi di correzione dei volti", "Selettore della destinazione" e "Suggerimento di identità nella scheda". I requisiti sull'algoritmo di raggruppamento non cambiano qui: li tocca la change sull'aggregazione delle identità.
- `identity-registry`:
  - "Dati di un'identità": la "cerchia" al posto del gruppo nei testi;
  - "Gestione delle identità": dettagli salvati subito, unione ed eliminazione dal menu ⋯.
- `ui-foundations`:
  - modificato "Avvisi non modali": avvisi che nominano azione e oggetto;
  - nuovi "Barre dei comandi", "Numeri e punteggi" e "Terminologia di Persone".
- `in-app-guidance`:
  - "Primi passi": conta solo i cataloghi dell'utente;
  - "Suggerimenti al primo uso": non coprono comandi;
  - "Menu Aiuto": diventa un menu della barra dei menu.

## Impact

- **UI** (`app/ui/src`):
  - `modules/people/*`: `GroupList`, `FaceGrid`, `PersonCard`, `IdentityEditor`, `PersonPicker`, `MergeDialog`, `people.css`;
  - nuovi `components/shell/MenuBar.tsx` e `components/about/*`;
  - `TopBar`, `ModuleNav`, `modules/registry.ts`, `ModulePage`, `shell.css`, `ui.css`, `theme/tokens.ts`;
  - `FirstSteps`, `Sidebar`, `JobsPage`, `SequencePage`, `TagsPage`, `OrganizePage`, `CatalogsPage`, `Onboarding`, `SettingsPage`;
  - i18n it/en.
- **Rust**:
  - `app/src-tauri/build.rs` (commit, data e configurazione della build);
  - comando `app_info` esteso (sistema, WebView, acceleratore, autori, repository);
  - nuovo comando che legge le licenze di terze parti installate;
  - menu nativo su macOS (API dei menu di Tauri) con gli eventi inoltrati al registro dei comandi.
- **Build**:
  - generazione dell'elenco completo delle licenze (dipendenze Rust e dell'interfaccia) in `assets/licenses/`, già incluso nell'installer;
  - `repository` nei metadati di `Cargo.toml`.
- **Test**:
  - vitest per Persone (una selezione, barra delle azioni, ordine di avanzamento), menu, Informazioni, glossario e scala tipografica;
  - prova sull'app reale con la stessa libreria di prova.
- **Rischio**: la change tocca quasi tutto il modulo Persone, che ha molti test. È divisa in fasi che si possono integrare una alla volta (vedi `tasks.md`).
