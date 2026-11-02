# Design

## Context

- Il filtro della libreria (`LibraryFilter`) ha già un intervallo di date inclusivo `dateFrom`/`dateTo` (`AAAA-MM-GG`), confrontato con l'ora locale corretta (`LOCAL_TIME_SQL`: istante reale più fuso, oppure l'ora letta dal file).
- Oggi la lista Anno usa solo questo intervallo: scegliere "2023" imposta `2023-01-01`…`2023-12-31` (`facets.ts`, `toggleIn`). Nel backend `FacetKey::Year` raggruppa per `substr(LOCAL_TIME_SQL, 1, 4)`, ignora la propria selezione (`without_own` toglie `date_from`/`date_to`) e ricava l'anno scelto dai primi 4 caratteri di `date_from`.
- Il filtro della libreria non è salvato tra i riavvii (`store/library.ts` non lo include in `partialize`), e i cataloghi smart usano le regole, non `LibraryFilter`.

## Goals / Non-Goals

**Goals:**
- Mesi e giorni dentro l'anno e il mese scelti, con conteggi coerenti con gli altri filtri.
- Nessuna modifica al catalogo né al formato del filtro.

**Non-Goals:**
- Intervalli di date liberi (da/a) o più anni insieme.
- Mese e giorno nei cataloghi smart (la regola "Anno" resta com'è).
- Gerarchia nel pannello flottante "Apri in finestra" degli anni: resta l'elenco degli anni; sceglierne uno lì sostituisce l'intera scelta della data, come nella barra laterale.

## Decisions

### 1. Il livello scelto si ricava dall'intervallo esistente
Anno = `AAAA-01-01`…`AAAA-12-31`; mese = `AAAA-MM-01`…ultimo giorno del mese; giorno = `dateFrom == dateTo`. Una funzione condivisa (Rust in `query.rs`, TypeScript in `facets.ts`) restituisce il percorso scelto (`year`, `month` come `AAAA-MM`, `day` come `AAAA-MM-GG`) e lo usa sia per i conteggi sia per la UI. Un intervallo che non è un anno, un mese o un giorno intero (non prodotto dall'app) viene trattato come scelta del solo anno di `dateFrom`, come oggi.

*Alternativa scartata*: un nuovo campo `date: "2023-05"` nel filtro. Più esplicito, ma cambia l'IPC e duplica l'intervallo già supportato da query, ordinamento e test; senza vantaggi per l'utente.

### 2. Mesi e giorni nel riepilogo dei filtri
`Facets` riceve due campi opzionali: `months: Option<FacetList>` (presente quando è scelto un anno; valori `AAAA-MM`) e `days: Option<FacetList>` (quando è scelto un mese o un giorno; valori `AAAA-MM-GG`). Si calcolano con il filtro **senza la scelta della data** più l'intervallo dell'anno (o del mese) scelto, raggruppando per `substr(LOCAL_TIME_SQL, 1, 7)` o `1, 10`, in ordine cronologico. Al massimo 12 o 31 voci, quindi sempre complete (`total` = numero di voci) e senza elenco paginato: nessun nuovo `FacetKey` né comando IPC.

La voce scelta compare anche con 0 foto (come `summarize` per le altre liste), così si può sempre togliere la spunta.

*Alternativa scartata*: nuovi `FacetKey::Month/Day` interrogati da `library_facet_values`. Servirebbe una chiamata in più per livello e porterebbe ricerca e ordinamento dove non servono.

### 3. Interazione nella UI
Nuovo componente `DateFacet` al posto di `FacetList` per la lista Anno: righe degli anni (le prime voci più "Mostra tutti" e "Apri in finestra", come oggi), sotto l'anno scelto le righe dei mesi rientrate, sotto il mese scelto quelle dei giorni rientrate di un altro livello. Riusa `FacetRow` per caselle e conteggi.

Azioni (funzione pura `toggleDate` in `facets.ts`, testata):
- anno non scelto → intervallo dell'anno; anno già scelto (a qualunque livello) → nessun filtro per data;
- mese non scelto → intervallo del mese; mese scelto (anche con un giorno) → intervallo dell'anno;
- giorno non scelto → quel giorno; giorno scelto → intervallo del mese.

Nomi: mesi con `Intl.DateTimeFormat(lingua, { month: "long" })`, giorni come numero con il giorno della settimana abbreviato ("14 dom"); chip con `{ day: "numeric", month: "long", year: "numeric" }` o `{ month: "long", year: "numeric" }`. La data viene costruita in UTC dai componenti della stringa, per non spostarla con il fuso del computer.

## Risks / Trade-offs

- [Un mese aggiunge due query al riepilogo] → Sono raggruppamenti sulle sole foto già filtrate, uguali alla query degli anni; vengono eseguiti solo quando un anno o un mese è scelto. Test di risposta sulla libreria grande già presente in `pc-store` copre gli anni; i nuovi livelli usano lo stesso indice su `media_dates`.
- [Intervalli impostati a mano non riconosciuti] → Non ci sono altri punti dell'app che impostano `dateFrom`/`dateTo`; il caso viene trattato come scelta dell'anno.

## Migration Plan

Nessuna migrazione: il catalogo e il formato del filtro non cambiano. Il ritorno alla versione precedente non richiede passi.
