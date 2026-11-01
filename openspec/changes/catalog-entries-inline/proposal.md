# Proposal

## Why

Issue #4 ("Nuova voce in catalogazione"): l'utente vuole poter inserire una voce di catalogazione nuova. Oggi una voce si aggiunge solo da Impostazioni › Tassonomia, scrivendo prompt in inglese per CLIP. Quando l'utente corregge una foto nell'Inspector o con la barra bulk, può scegliere solo tra le voci esistenti. In più, qualsiasi modifica della tassonomia ne aumenta la versione e marca tutta la libreria come "da rianalizzare", anche quando la voce aggiunta non c'entra con l'analisi automatica.

## What Changes

- **"Nuova voce…" durante la correzione**: nei selettori dei valori di catalogazione (Inspector, barra bulk e, quando esisterà, menu contestuale della griglia) l'utente può creare una voce scrivendone solo il nome. La voce viene aggiunta al gruppo e assegnata subito alla foto o alla selezione come correzione manuale.
- **Voci manuali**: una voce può non avere una descrizione per il riconoscimento automatico. In quel caso CLIP non la assegna mai, ma l'utente sì: compare nei filtri, nei cataloghi smart e nell'Inspector. Una descrizione si può aggiungere in seguito per renderla automatica.
- **Nuovi gruppi di catalogazione** (es. "Evento", "Progetto") creati da Impostazioni › Tassonomia senza obbligo di prompt; un gruppo può essere solo manuale.
- **Rianalisi solo quando serve**: aggiungere, rinominare o eliminare voci e gruppi manuali non marca più la libreria come da rianalizzare. Lo fanno solo le modifiche che cambiano l'analisi automatica (prompt, voci automatiche attivate/disattivate, soglie).
- **Nome mostrato**: le voci create dall'utente sono mostrate col loro nome in Inspector, filtri e cataloghi (oggi la UI mostra la traduzione dell'id o l'id stesso).
- Eliminare una voce che ha foto assegnate chiede conferma e indica quante foto la perdono.

## Capabilities

### New Capabilities

<!-- nessuna -->

### Modified Capabilities

- `ai-classification`: "Tassonomia configurabile" (voci e gruppi manuali, creazione senza prompt), "Versionamento delle analisi" (solo le modifiche che toccano l'analisi automatica rendono i file da rianalizzare); nuovo requisito "Creazione di voci durante la correzione".

## Impact

- **Codice Rust**: `pc-ai/src/taxonomy.rs` (voci senza prompt, gruppi senza voci automatiche, `analysis_version` separata da `version`, generazione di id univoci dal nome), `pc-ai/src/text_cache.rs` e `classify.rs` (ignorano le voci manuali), `pc-ai/src/analyze.rs` e `pc-store/src/analysis.rs` (versione di analisi usata per `stale_count` e `pending`), `app/src-tauri` (comando per aggiungere una voce a un gruppo ed eventualmente assegnarla in un passo; eliminazione con conteggio).
- **UI**: `Inspector.tsx` e `BulkBar.tsx` ("Nuova voce…" nel selettore dei valori), `SettingsPage.tsx` (voci e gruppi manuali, descrizione facoltativa), visualizzazione del nome della voce in Inspector, `Sidebar.tsx` e `RuleEditor.tsx`, i18n it/en.
- **Dati**: nessuna migrazione di `catalog.db`. Il JSON della tassonomia guadagna campi con valore predefinito; al primo caricamento `analysis_version` assume il valore di `version`, così le analisi esistenti restano valide.
- **Coordinamento**: `manual-edits-open-with-ai-tuning` (in implementazione sul branch `feat/user-feedback-tuning`) aggiunge le soglie per voce e il job `retune` nella stessa tassonomia; `facet-lists-expand` riscrive le liste dei filtri in `Sidebar.tsx`. Vedi le note in `tasks.md`.
