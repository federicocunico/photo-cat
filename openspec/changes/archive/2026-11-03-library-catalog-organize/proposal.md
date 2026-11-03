# Proposal

## Why

La Libreria oggi mette in primo piano le cartelle (sorgenti e albero) e i cataloghi vengono dopo. Le foto si aggiungono a un catalogo solo dalla barra delle azioni o dall'Inspector. L'utente vuole due ruoli separati:
- la **Libreria** serve ad aggiungere le cartelle, analizzarle e caricarle;
- l'**organizzazione** passa dai cataloghi, con gesti diretti: trascinare le foto su un catalogo, oppure "Aggiungi a catalogo" dal menu contestuale.

Due comportamenti sono poi diversi da quanto l'utente si aspetta:
- Maiusc+clic più volte di seguito sposta l'ancora dell'intervallo. La selezione fatta prima si perde: in Esplora risorse invece l'intervallo parte sempre dallo stesso elemento.
- Una foto spostata nel Cestino resta nei suoi cataloghi. L'utente vuole che ne esca, dopo una conferma.

## What Changes

- **Selezione con Maiusc**: l'ancora dell'intervallo resta la foto dell'ultimo clic semplice o con Ctrl. Maiusc+clic successivi ricalcolano l'intervallo da quella foto (come in Esplora risorse). Stessa correzione nella griglia dei volti di Persone.
- **Trascinare sui cataloghi**:
  - le foto si trascinano dalla griglia ai cataloghi manuali della barra laterale e vengono aggiunte al catalogo;
  - trascinando una foto selezionata si spostano tutte le selezionate (anche la selezione "tutti i risultati del filtro"); trascinando una foto non selezionata si sposta solo quella;
  - durante il trascinamento un'etichetta mostra quante foto si spostano, e il catalogo sotto il puntatore si evidenzia;
  - lasciando le foto su "Nuovo catalogo" si crea un catalogo manuale con quelle foto;
  - lasciandole sul Cestino si spostano nel Cestino;
  - cataloghi smart e gli altri cataloghi di sistema non accettano foto e lo indicano;
  - avviso con "Annulla operazione".
- **"Aggiungi a catalogo" nel menu contestuale** della griglia: un sottomenu con ricerca ed elenco scorrevole dei cataloghi manuali, più "Nuovo catalogo…".
- **Barra laterale della Libreria orientata ai cataloghi**:
  - in alto la sezione **Cataloghi**: cataloghi dell'utente, "Nuovo catalogo" con creazione sul posto, e i cataloghi di sistema separati in fondo alla sezione;
  - poi i **Filtri**;
  - in fondo le **Cartelle**, chiuse per impostazione predefinita, con il comando per gestirle in Importa.
- **Cestino che toglie dai cataloghi** (**BREAKING**: cambia il requisito "Cestino"):
  - spostare nel Cestino toglie le foto anche dai cataloghi manuali;
  - se almeno una foto è in un catalogo manuale, prima compare una conferma che elenca i cataloghi interessati e quante foto perde ognuno; se le foto non sono in nessun catalogo, non compare nessun dialogo;
  - "Annulla operazione" rimette le foto nei cataloghi;
  - "Ripristina" dal Cestino riporta le foto in Libreria senza i cataloghi.

## Capabilities

### New Capabilities

Nessuna.

### Modified Capabilities

- `catalog-library`:
  - "Cestino": toglie dai cataloghi, con conferma;
  - "Ripristino da un catalogo di sistema": dal Cestino tornano senza cataloghi;
  - "Cataloghi manuali": aggiunta per trascinamento e "Nuovo catalogo" con le foto trascinate;
  - "Albero delle cartelle": sezione in fondo, chiusa per impostazione predefinita.
- `bulk-operations`:
  - "Selezione multipla": ancora stabile per Maiusc+clic.
- `desktop-shell`:
  - "Layout a pannelli": ordine del pannello sinistro (cataloghi, filtri, cartelle);
  - "Menu contestuale della griglia": "Aggiungi a catalogo ▸".

## Impact

- **UI, Libreria**:
  - `app/ui/src/modules/library/Grid.tsx`: ancora della selezione, inizio del trascinamento;
  - `Sidebar.tsx`: ordine delle sezioni, bersagli di rilascio, "Nuovo catalogo";
  - `GridContextMenu.tsx`: sottomenu dei cataloghi;
  - `SelectionBar.tsx` e il tasto Canc: Cestino con conferma;
  - un nuovo modulo condiviso per il trascinamento.
- **UI, altri moduli**: `app/ui/src/modules/people/FaceGrid.tsx` (ancora); `store/layout.ts` (sezione Cartelle chiusa per i nuovi profili); testi it/en.
- **Backend**:
  - nuova azione bulk `move_to_trash` (`crates/pc-jobs/src/bulk.rs`, `crates/pc-store/src/bulk.rs`) con annullamento che ripristina le appartenenze;
  - nuovo comando di sola lettura che riassume i cataloghi manuali toccati da una selezione (`app/src-tauri/src/catalogs.rs`);
  - binding rigenerati.
- **Dati**: nessuna migrazione. Le appartenenze tolte vanno nel registro di annullamento esistente.
- **Compatibilità**: `add_catalog` sul Cestino resta valido per i chiamanti esistenti (es. regole interne), ma l'interfaccia usa `move_to_trash`.
