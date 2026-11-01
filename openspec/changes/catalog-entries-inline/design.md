# Design

## Context

- La tassonomia è un JSON in `settings_kv` (`pc-ai/src/taxonomy.rs`). `validate()` richiede per ogni voce almeno un prompt e per ogni gruppo almeno una voce attiva. `save()` richiede `version = expected + 1` (concorrenza ottimistica).
- `version` ha due ruoli: concorrenza delle modifiche e versione di analisi. I risultati CLIP la registrano in `analysis_version.taxonomy_ver` (`analyze.rs`, `stages.rs`) e `stale_count`/`pending` la confrontano, quindi **ogni** modifica rende "da rianalizzare" tutta la libreria.
- `text_cache::cached_or_compute` calcola i vettori dei concetti di tutte le voci attive. La cache è valida solo se la tassonomia è identica a quella salvata nella cache.
- L'Inspector (`Edits`) e la `BulkBar` usano `BulkAction::SetLabel { group, value }`. `bulk::set_label` scrive con `origin = 'manual'` e registra l'undo, senza verificare che il valore esista nella tassonomia.
- La UI mostra i valori con `t(\`labels.${v}\`, { defaultValue: v.replace(/_/g, " ") })` in `Inspector.tsx`, `Sidebar.tsx` e `chips.ts`. Le voci create dall'utente non hanno traduzioni, quindi oggi apparirebbe l'id.
- Impostazioni › Tassonomia permette di aggiungere voci (con prompt) ma non gruppi.

## Goals / Non-Goals

**Goals:**
- Voci e gruppi manuali senza prompt, creati anche durante la correzione.
- Separare la versione "di modifica" da quella "di analisi", così che solo i cambiamenti rilevanti per CLIP richiedano la rianalisi.
- Mostrare ovunque il nome della voce definito nella tassonomia.

**Non-Goals:**
- Traduzione automatica del nome della voce in un prompt inglese per CLIP: una voce diventa automatica solo se l'utente scrive una descrizione.
- Voci con più valori per foto nello stesso gruppo: la correzione manuale resta a un valore per gruppo, come oggi.
- Gerarchie di voci (sottovoci).

## Decisions

### 1. Voci manuali = voci senza prompt
`Category.prompts` può essere vuoto. Una voce con `prompts` vuoto è **manuale**: è esclusa da `text_cache` e da `classify::labels`, ma è valida per `set_label`, filtri e regole. `validate()` cambia così:
- una voce richiede id e nome non vuoti; i prompt, se presenti, rispettano i limiti attuali;
- un gruppo può non avere voci automatiche attive (gruppo solo manuale) ma deve avere almeno una voce;
- i gruppi automatici (con almeno una voce automatica attiva) restano soggetti al controllo attuale.
*Alternativa scartata*: un flag `manual: bool` separato dai prompt — due fonti di verità; "senza descrizione" è già il significato che l'utente vede.

### 2. `analysis_version` separata
La tassonomia guadagna `analysis_version: i64` (serde default = `version` al caricamento, tramite deserializzazione in due passi). `save()` confronta la parte rilevante per l'analisi della tassonomia nuova con quella vecchia: gruppi con le loro soglie, voci automatiche attive con id e prompt (l'ordine non conta). Se cambia, `analysis_version = max(analysis_version, version)` e si aggiorna; altrimenti resta invariata. `analyze.rs` e `stages.rs` registrano `analysis_version` in `taxonomy_ver`, e `stale_count`/`pending` la usano. Le analisi esistenti hanno `taxonomy_ver = version` attuale, che coincide con il valore iniziale di `analysis_version`: nessuna libreria diventa "da rianalizzare" all'aggiornamento.

`text_cache` confronta la proiezione rilevante invece dell'intera tassonomia, così rinominare una voce non ricalcola i vettori.

Se il change `manual-edits-open-with-ai-tuning` è già unito, le soglie per voce fanno parte della proiezione rilevante e la rianalisi proposta per una voce resa automatica passa dal suo job `retune` (che ricalcola dalle embedding senza rieseguire il modello immagine). Se non è unito, si propone la rianalisi normale.

### 3. Id delle voci create dall'utente
Id generato dal nome: `fold` (minuscole, senza accenti), caratteri non alfanumerici → `_`, con suffisso numerico in caso di collisione (`barche`, `barche_2`). Gli id restano stabili quando il nome cambia. Le chiavi di traduzione `labels.<id>` restano per le voci predefinite.

### 4. Comando unico "crea e assegna"
Nuovo comando IPC `add_category(group, label, prompts: Vec<String>, assign_to: Option<Vec<MediaId>>) -> CategoryRef`:
1. carica la tassonomia; se esiste una voce del gruppo con lo stesso nome dopo `fold`, la riusa;
2. altrimenti la aggiunge con un nuovo id e salva (`version + 1`, con `analysis_version` invariata perché la voce è manuale);
3. se `assign_to` è presente, avvia `BulkAction::SetLabel` con lo stesso percorso (job sopra la soglia, undo, esito). L'undo annulla l'assegnazione ma non elimina la voce, che resta disponibile.

In caso di conflitto di versione (un'altra modifica concorrente) il comando ricarica la tassonomia e riprova una volta.

### 5. Eliminazione di una voce
`delete_category(group, id)`: conta le etichette con quel valore, la UI chiede conferma con il numero; dopo la conferma la voce viene rimossa dalla tassonomia e le etichette con quel valore cancellate in un'unica transazione, registrata in `undo_log` come `LabelsReplaced` per poterla annullare. Se la voce era automatica, `analysis_version` aumenta.

### 6. Nome visualizzato
Un hook `useLabelName()` (basato sulla query `["taxonomy"]` già esistente) restituisce: la traduzione `labels.<id>` se esiste e la voce non è stata rinominata dall'utente, altrimenti `category.label`, e come ultima risorsa l'id con gli `_` sostituiti. Sostituisce le tre chiamate dirette a `t(\`labels.${v}\`)` in `Inspector.tsx`, `Sidebar.tsx` e `chips.ts`, e viene usato in `RuleEditor.tsx`. Per sapere se una voce predefinita è stata rinominata si confronta `category.label` con il nome predefinito della tassonomia di fabbrica.

### 7. UI
- **Selettore dei valori** (Inspector e BulkBar): ultima opzione "Nuova voce…", che mostra sul posto un campo di testo con Conferma/Annulla; Invio conferma, Esc annulla. Nessun dialogo modale.
- **Impostazioni › Tassonomia**: "Aggiungi gruppo"; per ogni voce il campo "Descrizione per il riconoscimento automatico (inglese)" è facoltativo, con un'etichetta "manuale" quando è vuoto; eliminazione con conteggio; avviso "questa modifica richiede la rianalisi" mostrato solo quando cambia la parte rilevante per l'analisi.

## Risks / Trade-offs

- [Gruppi creati dall'utente con id che `bulk::set_label` rifiuta: oggi accetta solo `[a-z_]`] → l'id dei gruppi viene generato con la stessa regola dell'id delle voci, ristretta a `[a-z_]`, con suffisso alfabetico in caso di collisione (`evento`, `evento_b`). Test dedicato.
- [Conflitti con `manual-edits-open-with-ai-tuning`, che modifica `taxonomy.rs` (soglia per voce), `classify.rs`, `SettingsPage.tsx`, `Inspector.tsx`, `BulkBar.tsx`] → cambi in `taxonomy.rs` limitati a campi e funzioni nuove; UI in componenti separati (`CategoryPicker`, `useLabelName`). Chi arriva per secondo fa il rebase.
- [Conflitti con `facet-lists-expand` su `Sidebar.tsx`] → in quel change `FacetList` accetta già una funzione `render`: basta passarle `useLabelName`.
- [Proiezione rilevante per l'analisi incompleta, che fa saltare una rianalisi necessaria] → test che enumerano ogni campo della tassonomia e verificano se cambia o no `analysis_version`.

## Migration Plan

Nessuna migrazione di `catalog.db`. Al primo caricamento della tassonomia `analysis_version = version`. Rollback: una versione precedente rifiuta le voci senza prompt in `validate()`. Prima del rollback vanno eliminate le voci manuali, oppure ripristinata la tassonomia predefinita da Impostazioni. Va documentato nelle note di rilascio.
