# Design

## Context

- `pc-store/src/query.rs::facets(filter)` calcola in un'unica chiamata i valori con conteggio per catalogazioni (per gruppo), luoghi, anni, formati, persone e tag, applicando **tutto** il filtro corrente. Luoghi, persone e tag hanno `LIMIT 100`; le catalogazioni, gli anni e i formati non hanno limiti.
- `Sidebar.tsx::CheckList` mostra `values.slice(0, 12)`, senza indicazione delle voci nascoste. Siccome il filtro include la selezione della lista stessa, scegliere un luogo riduce la lista Luoghi a quel solo luogo.
- La libreria usa già `@tanstack/react-virtual` per la griglia, e `zustand` con `persist` per lo stato dell'interfaccia (`store/layout.ts`).
- Nell'app non c'è un componente di pannello flottante; i dialoghi esistenti (`Onboarding`, `SequencePage`) sono sovrapposizioni fisse.

## Goals / Non-Goals

**Goals:**
- Rendere raggiungibile ogni valore di ogni lista di filtro della libreria, sia sul posto sia in un pannello flottante.
- Un componente di pannello flottante generico, riusabile da altri moduli.
- Liste di filtro che restano utilizzabili per la selezione multipla (conteggi che ignorano la selezione della lista stessa).

**Non-Goals:**
- Finestre native separate del sistema operativo (secondo monitor): vedi Decisione 4.
- Il filtro Persone nella barra laterale (il facet `people` esiste nel backend ma oggi non è mostrato): non cambia in questo change.
- Liste di sorgenti e cataloghi: sono già mostrate per intero.
- Nuove operazioni sui valori (rinomina, unione): appartengono ad altri change.

## Decisions

### 1. Facet "per esclusione"
In `facets(filter)` ogni facet viene calcolato con una copia del filtro in cui la sua selezione è svuotata: `places` per i luoghi, `tags` per i tag, `formats` per i formati, l'intervallo di date per gli anni, il gruppo corrispondente di `labels` per ciascun gruppo di catalogazione. Gli altri campi restano invariati.
Il costo: invece di un solo CTE `f` ci sono fino a 5 + numero di gruppi filtri compilati; quando la lista non ha selezione si riusa il CTE di base, quindi il caso comune (nessuna o una lista selezionata) costa quanto oggi più una query.
*Alternativa scartata*: lasciare il comportamento attuale e mostrare sempre le voci selezionate — risolve la visibilità ma non permette di aggiungere un secondo luogo senza togliere il primo.

### 2. Totali nel riepilogo, elenco completo su richiesta
- `Facets` si estende con il totale delle voci per ogni lista (`FacetList { values, total }`) e restituisce al massimo **50 valori** per lista, ordinati per conteggio, più le voci selezionate se non comprese (con il loro conteggio). Così la risposta di `library_facets`, chiamata a ogni cambio di filtro, resta piccola anche con migliaia di tag.
- Nuovo `facet_values(filter, facet: FacetKey, search: Option<String>, sort: Count|Name, offset, limit) -> FacetPage { values, total }` per l'elenco completo, con lo stesso filtro "per esclusione". `FacetKey = Label(group) | Place | Year | Format | Tag`.
- La ricerca ignora maiuscole e accenti: il raggruppamento avviene in SQL senza limiti, il confronto e la paginazione in Rust con una funzione `fold` (minuscole + rimozione dei segni diacritici, come quella già usata in `pc-ai/src/geo.rs`, spostata in `pc-core` per condividerla). Con migliaia di valori il costo è trascurabile rispetto alla query; se le misure lo smentiscono, si registra una funzione scalare SQLite `fold()` e si filtra in SQL.
- L'ordinamento alfabetico usa lo stesso `fold` con il valore originale come spareggio; per le catalogazioni AI si ordina per etichetta tradotta lato UI solo nella pagina corrente (le etichette tradotte non sono note al backend), documentato come limite.
*Alternativa scartata*: togliere solo i `LIMIT` e lasciare tutto alla UI — risposta pesante a ogni cambio di filtro e nessuna paginazione.

### 3. Lista compatta nella barra laterale
`CheckList` diventa `FacetList`:
- mostra le prime 8 voci, più le voci selezionate non comprese (in cima, marcate);
- se `total` supera le voci mostrate: "Mostra tutti (N)" espande sul posto un elenco con campo di ricerca, che carica pagine da `facet_values` (scorrimento infinito, `react-virtual`, altezza massima ~360 px con scorrimento interno) e "Mostra meno";
- un'icona "Apri in finestra" accanto al titolo della lista apre il pannello flottante (Decisione 4). È mostrata anche quando le voci sono poche, per coerenza.
- React Query: chiave `["facet-values", facetKey, filter, search, sort]`, `placeholderData` per evitare sfarfallii; ricerca con debounce di 200 ms.

### 4. Pannello flottante in-app
Componente generico `FloatingPanel` in `app/ui/src/components/floating/`:
- reso con un portal in un livello sopra `AppShell`, `role="dialog"` con `aria-modal="false"`, titolo come `aria-labelledby`;
- trascinamento dalla barra del titolo e ridimensionamento dall'angolo con Pointer Events (nessuna libreria), limiti minimi 260×240 px e vincolo a restare dentro la finestra (anche su `resize` della finestra);
- z-order: un contatore nello store porta in primo piano il pannello cliccato;
- stato in un nuovo store persistito `store/floating.ts`: `{ [panelId]: { open, x, y, w, h } }`, con lo stesso pattern `persist` di `layout.ts` (id stabili, es. `facet:place`, `facet:label:subject`);
- Esc chiude il pannello che ha il focus e riporta il focus al comando che lo ha aperto.

Il contenuto per i filtri è `FacetPanel`: ricerca, scelta dell'ordinamento, conteggio delle voci, elenco virtualizzato con caselle di spunta, "Deseleziona tutto". Legge e scrive il filtro tramite lo store `useLibrary`, quindi resta sincronizzato con barra laterale e griglia. Se si cambia modulo i pannelli dei filtri della libreria vengono nascosti e ricompaiono tornando in Libreria.

*Alternativa scartata*: finestra nativa Tauri (`WebviewWindow`) — permetterebbe di spostarla su un secondo monitor, ma richiede una seconda istanza della UI, sincronizzazione dello stato tra finestre tramite eventi, capability dedicate e verifiche su Windows/macOS/Linux. Il pannello in-app copre la richiesta ("meglio se flottante") con molto meno rischio; la finestra nativa resta un'estensione possibile riusando `FacetPanel`.

## Risks / Trade-offs

- [Query dei facet più costose con la logica "per esclusione" su librerie da 100.000 foto] → riuso del CTE di base per le liste senza selezione; misura con il test di prestazioni esistente (primo blocco entro 1 s) e, se serve, calcolo dei facet con selezione in query separate solo per le liste effettivamente visibili.
- [Ordinamento alfabetico delle catalogazioni AI fatto sul valore e non sull'etichetta tradotta] → le categorie sono al massimo qualche decina per gruppo: per le catalogazioni la UI chiede tutte le voci in una sola pagina e ordina sull'etichetta tradotta.
- [Conflitto con `manual-edits-open-with-ai-tuning` su `Sidebar.tsx`] → `FacetList` e `FacetPanel` accettano un'azione per riga opzionale (menu "…"), da usare per "Rinomina…" del luogo; chi arriva per secondo integra.
- [Pannelli fuori schermo dopo un cambio di risoluzione] → la posizione salvata viene ricondotta dentro la finestra all'apertura.

## Migration Plan

Nessuna migrazione dati. Il nuovo store persistito parte vuoto (pannelli chiusi, posizione predefinita accanto alla barra laterale). Rollback: revert del change; la chiave di `localStorage` del nuovo store resta inutilizzata.
