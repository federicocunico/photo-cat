# Proposal

## Why

photo-cat ha ormai tutte le funzioni di un catalogatore professionale, ma l'interfaccia è cresciuta per accumulo, una change alla volta, e oggi risulta ambigua e affollata. L'audit completo è in `audit.md`. Riguarda il codice di `main` (f40beb5), le change ancora da integrare (`loading-feedback`, `people-faces-workspace`) e il confronto con Lightroom Classic, digiKam, Apple Foto, Google Foto, Immich, PhotoPrism, Mylio, Photo Mechanic e con le linee guida NN/g, Fluent 2, Apple HIG e WCAG 2.2.

Problemi principali:

- **Navigazione**:
  - 12 voci piatte e senza gruppi: contenuti, strumenti e pagine di sistema hanno lo stesso peso;
  - impostazioni duplicate in Modelli e Diagnostica, con un modo di salvare diverso;
  - termini incoerenti ("Job", "Annulla" con cinque significati, Importa/Import/"Aggiungi una sorgente").
- **Libreria**: cinque fasce sopra la griglia; la barra bulk è sempre visibile e funziona come un modulo "azione + valore + Applica"; non c'è un visualizzatore interno; la griglia non si naviga con le frecce.
- **Componenti**:
  - non esistono dialoghi, menu o messaggi comuni;
  - 4 modi diversi di chiedere conferma, 7 `window.confirm`, quasi 600 stili inline, 9 corpi di testo diversi, titoli di pagina incoerenti;
  - finestre modali senza focus.
- **Linguaggio tecnico in vista**:
  - editor delle regole con `eq/gte/people_count`;
  - `#12 · import`, soglie 0–1, SHA-256, glob.
- **Vicoli ciechi**:
  - lo stato vuoto di Persone rimanda a un'azione che non esiste;
  - la nota di Tag rimanda a un "Annulla" che c'è solo in Libreria;
  - "Crea cartelle" resta disabilitato senza spiegazione.
- **Primo avvio e aiuto**: l'onboarding mostra numeri senza etichette e alla fine porta a Job invece che alle foto. Non c'è aiuto contestuale, né elenco delle scorciatoie, né palette dei comandi.

Questa change arriva **dopo** tutte le altre già previste, così il riordino si fa una volta sola sull'interfaccia definitiva.

## What Changes

- **Architettura dell'informazione**:
  - navigazione a gruppi:
    - **Sfoglia**: Libreria, Sequenza;
    - **Organizza**: Cataloghi, Persone, Tag;
    - **Importa ed esporta**: Importa, Esporta;
    - in fondo **Attività** (ex Job) e **Impostazioni**;
  - Modelli AI e Diagnostica diventano sezioni di Impostazioni, sempre raggiungibili da Attività, dalla barra di stato e dalla palette;
  - navigazione richiudibile a sole icone, con scorciatoie Ctrl+1…7.
- **Libreria**:
  - una barra unica con contesto (percorso "Tutte le foto › Catalogo …"), conteggio, filtri attivi, menu "Ordina" e "Vista";
  - una **barra delle azioni sulla selezione** che sostituisce la barra degli strumenti quando c'è una selezione, con pulsanti diretti e menu "Altro";
  - **visualizzatore** interno (Spazio/Invio) condiviso con Sequenza;
  - navigazione da tastiera nella griglia;
  - Inspector con informazioni in alto e modifica sul campo.
- **Palette dei comandi** (Ctrl+K): moduli, azioni sulla selezione, foto, persone, tag, cataloghi e impostazioni. Foglio delle scorciatoie con `?`.
- **Guida nell'app**:
  - onboarding in 3 passi che finisce in Libreria mentre le foto arrivano;
  - lista "Primi passi";
  - suggerimenti brevi al primo uso di Persone, Cataloghi, Tag e Sequenza, che si possono saltare e rivedere dal menu Aiuto.
- **Impostazioni** divise in categorie con sotto-navigazione e ricerca:
  - un solo modo di salvare: subito per le preferenze, "Applica" solo per ciò che ricalcola dati;
  - soglie espresse come "sensibilità", con valori leggibili.
- **Fondamenta visive**:
  - token di spaziatura, raggio, tipografia, elevazione e movimento;
  - set di icone unico al posto di glifi come ▸ ↻ ✎ ×;
  - libreria di componenti condivisi: PageHeader, Dialog, ConfirmDialog, Menu, Toast in pila, EmptyState, InlineAlert, DataTable, FormField, FolderPicker, SegmentedControl, Tabs, Badge, Kbd, Tooltip;
  - tema "Come il sistema" predefinito.
- **Regole di interazione**:
  - conferma solo per ciò che non si può annullare o tocca i file;
  - tutto il resto si annulla da avviso o con Ctrl+Z;
  - un comando disabilitato spiega perché;
  - errori in linguaggio comprensibile, nel punto in cui servono;
  - glossario dei termini.
- **Accessibilità WCAG 2.2 AA**:
  - dialoghi con focus intrappolato;
  - bersagli di almeno 24 px;
  - righe e tabelle utilizzabili da tastiera;
  - nessun controllo annidato in un altro;
  - nessun significato affidato al solo colore.
- **Editor delle regole dei cataloghi** in linguaggio naturale, con anteprima.
- **Mockup** sul canvas di design "photo-cat — revisione UI/UX" (vedi `design.md`), con copia dei sorgenti in `mockups/`.

Nessuna modifica rompe dati o file. Il modulo "Diagnostica" e il modulo "Modelli AI" non sono più voci di primo livello: le preferenze di layout salvate che li indicano come modulo attivo vengono migrate.

## Capabilities

### New Capabilities

- `ui-foundations`: componenti condivisi, regole di conferma e annullamento, messaggi d'errore, stati vuoti utili, comandi disabilitati spiegati, terminologia, requisiti di accessibilità.
- `command-palette`: palette dei comandi (Ctrl+K), scorciatoie globali e foglio delle scorciatoie.
- `in-app-guidance`: suggerimenti al primo uso, lista "Primi passi", menu Aiuto.

### Modified Capabilities

- `desktop-shell`:
  - requisiti modificati:
    - "Layout a pannelli";
    - "Moduli di navigazione", sul testo di `people-faces-workspace`;
    - "Indicatori globali", sul testo di `loading-feedback`;
    - "Pannello di ispezione";
    - "Stile visivo";
    - "Primo avvio guidato";
    - "Impostazioni";
  - nuovi requisiti:
    - "Barra delle azioni sulla selezione";
    - "Selettori delle azioni sulla selezione";
    - "Visualizzatore delle foto";
    - "Comandi del visualizzatore";
    - "Navigazione da tastiera nella griglia";
    - "Contesto della vista in Libreria".
- `catalog-library`: nuovi requisiti "Editor delle regole comprensibile" e "Conteggi nell'elenco dei cataloghi".
- `external-apps`: "Azione del doppio clic" aggiunge il visualizzatore interno tra le scelte; il valore predefinito resta l'app del sistema.

## Impact

- **Prerequisiti**: devono essere integrate e archiviate `loading-feedback` e `people-faces-workspace`, e archiviata `catalog-exclusions`, che è già integrata ma non ancora archiviata. Prima di implementare, le delta di `desktop-shell` si riallineano ai testi archiviati.
- **UI** (`app/ui/src`):
  - nuovi `components/ui/*` e `components/command/*`, `components/help/*`;
  - shell (`ModuleNav`, `TopBar`, `StatusBar`, `AppShell`) e `modules/registry.ts`;
  - `store/layout.ts`, con migrazione del modulo persistito;
  - tutte le pagine dei moduli: Libreria per prima, poi Impostazioni, Onboarding, Cataloghi, Esporta, Attività, Importa, Sequenza, Tag, Persone;
  - `theme/tokens.ts`, con nuovi token e tema "system";
  - `styles/global.css`, icone, i18n it/en (glossario, chiavi rinominate, chiavi inutilizzate rimosse).
- **Rust**: nessuna nuova logica di dominio. Restano possibili piccoli comandi di supporto (stato "primi passi" e suggerimenti visti in `config.toml` o `ui-prefs.json`) e il tema `system` in `Settings.ui.theme`.
- **Test**: vitest per componenti e regole (focus, conferme, palette), aggiornamento dei test di pagina, controllo automatico di contrasto e dimensione dei bersagli, prova sull'app reale.
- **Rischio**: la change tocca quasi ogni file UI. Per questo è divisa in fasi che si possono integrare una alla volta, ciascuna con test verdi (vedi `tasks.md`).
