# Proposal

## Why

Issue #12 ("Gerarchia data?"): il filtro per data della libreria permette di scegliere solo l'anno. Per trovare le foto di un mese o di un giorno preciso l'utente deve scorrere la griglia. Mese e giorno da soli (es. "maggio" di tutti gli anni) non hanno senso, quindi vanno scelti dentro una gerarchia: prima l'anno, poi il mese di quell'anno, poi il giorno di quel mese.

## What Changes

- La lista **Anno** della barra laterale diventa gerarchica: dopo aver scelto un anno compaiono, rientrati sotto di esso, i **mesi** di quell'anno che contengono foto, con il conteggio; scelto un mese compaiono i **giorni** di quel mese con foto, con il conteggio.
- Mesi e giorni non sono filtri a sé: compaiono solo sotto l'anno (o il mese) scelto e valgono solo per quello.
- Scegliere un mese restringe il filtro a quel mese; scegliere un giorno a quel giorno. Togliere la spunta a un giorno torna al mese, al mese torna all'anno, all'anno toglie il filtro per data. Scegliere un altro anno (o un altro mese) sostituisce la scelta precedente.
- I conteggi di mesi e giorni applicano tutti gli altri filtri attivi ma non la scelta della data, come già gli anni: dopo aver scelto un mese gli altri mesi dello stesso anno restano visibili e selezionabili.
- Il chip del filtro attivo mostra il livello scelto ("2023", "maggio 2023", "14 maggio 2023"), con mese e giorno nella lingua dell'interfaccia; rimuoverlo toglie il filtro per data.
- Mesi e giorni usano l'ora locale corretta di ogni foto (fuso e scostamenti), come il filtro per anno; le foto senza data non compaiono in nessun anno, mese o giorno.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `catalog-library`: nuovo requisito "Filtro per data gerarchico" (anno → mese → giorno, conteggi e chip). "Ricerca e filtri", "Elenco completo dei valori dei filtri" e "Conteggi dei filtri indipendenti dalla propria selezione" restano invariati e valgono anche per mesi e giorni.

## Impact

- **Codice Rust**: `crates/pc-store/src/query.rs`: riepilogo dei filtri con le liste di mesi e giorni per l'anno e il mese scelti, calcolate con il filtro senza la scelta della data; test in `pc-store`. `bindings.ts` rigenerato (nuovi campi di `Facets`).
- **UI**: `app/ui/src/modules/library/` (`Sidebar.tsx`, `facets.ts`, `chips.ts`, nuovo componente per la lista gerarchica delle date), i18n it/en, test vitest.
- **Compatibilità**: nessuna migrazione. Il filtro continua a usare l'intervallo di date esistente (`dateFrom`/`dateTo`), il catalogo non cambia e le versioni precedenti restano compatibili. I cataloghi smart non cambiano.
- **Dipendenze**: nessuna nuova (i nomi dei mesi vengono da `Intl.DateTimeFormat`).
