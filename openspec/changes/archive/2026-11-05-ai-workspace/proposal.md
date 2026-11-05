# Proposal

## Why

Oggi l'AI si controlla solo in modo indiretto. Le soglie stanno in Impostazioni, i job in Attività, la rianalisi è un'azione sulla selezione, e nessuna vista dice quante foto sono davvero analizzate, con quali risultati e dove. Chi gestisce una libreria di decine di migliaia di foto non vede lo stato reale del sistema. Non può rianalizzare in modo mirato, per esempio "solo questi due cataloghi, forzando", né ripulire in blocco i risultati sbagliati senza che la rianalisi successiva li rimetta.

## What Changes

- **Nuova voce di navigazione "AI"**, in un gruppo proprio tra Organizza e Importa ed esporta. È un pannello di controllo del sistema live, non di impostazioni, con queste sezioni:
  - **Panoramica**: per ogni fase (classificazione, persone, volti, gruppi di simili, impronte) quante foto sono analizzate, da aggiornare, in coda o mai analizzate. Si aggiorna dal vivo durante i job e mostra job AI attivi, modelli caricati e memoria. Ogni segmento apre la Libreria filtrata su quelle foto.
  - **Rianalisi**: ambito a scelta (tutta la libreria, uno o più cataloghi, selezione corrente, risultati filtrati della Libreria), fasi da includere, modalità "solo mancanti o non aggiornate" oppure "forza". Prima dell'avvio mostra quante foto saranno elaborate. Le correzioni manuali restano sempre valide.
  - **Risultati**: grafici interattivi sull'ambito scelto. Distribuzione delle voci per gruppo della tassonomia, istogramma delle confidenze di una voce con la soglia corrente, copertura dell'analisi nel tempo. Clic e selezione di un intervallo portano in Libreria o alle azioni di pulizia.
  - **Pulizia**: togliere un'etichetta automatica da un insieme di foto (ad esempio "Natura" sotto confidenza 0,35 nel catalogo "Vacanze"). La rimozione resta valida dopo la rianalisi e si può annullare.
  - **Simili**: numero di gruppi, foto raggruppate, foto in attesa, distribuzione delle dimensioni dei gruppi, avvio e ricalcolo dell'analisi dei gruppi.
- **Filtro della Libreria per stato dell'analisi** (fase + analizzata / da aggiornare / mai analizzata), usato dai collegamenti della Panoramica.
- **Etichette automatiche rifiutate**: una voce tolta dall'utente non viene più assegnata automaticamente a quella foto.
- Scorciatoie dei moduli: diventano Ctrl+1…Ctrl+8.

La mappa del mondo è una change separata (`photo-map`), da fare dopo. Si aggiungerà come sezione di questo modulo.

## Capabilities

### New Capabilities

- `ai-workspace`: modulo AI, con panoramica live, rianalisi per ambito, grafici dei risultati, pulizia e stato dei gruppi di simili.

### Modified Capabilities

- `desktop-shell`: "Moduli di navigazione" aggiunge il gruppo e la voce AI e le scorciatoie Ctrl+1…Ctrl+8.
- `ai-classification`: nuovo requisito "Etichette automatiche rifiutate".
- `catalog-library`: nuovo requisito "Filtro per stato dell'analisi".

## Impact

- **Rust**:
  - nuove query aggregate in `pc-store`: copertura per fase, istogramma delle confidenze, copertura per mese, statistiche dei gruppi;
  - tabella `label_rejection` (schema v12) rispettata da `analysis::classification` e `relabel`;
  - parametri `force` e `stages` nel job `analysis`;
  - nuovi comandi Tauri (`ai_overview`, `ai_label_histogram`, `ai_reprocess_preview`, `ai_reprocess`, `ai_reject_labels`, ...);
  - `LibraryFilter.analysis`;
  - azione bulk annullabile per il rifiuto delle etichette.
- **UI**: nuovo modulo `app/ui/src/modules/ai/`, voce in `registry.ts`, `ModuleId`, `ReadyApp.tsx`, i18n, palette dei comandi e scorciatoie. Grafici SVG fatti a mano, senza nuove dipendenze.
- **Dipendenze tra change**: la fase "impronte" della Panoramica e della Rianalisi richiede `perceptual-hash-similarity`, da applicare prima.
