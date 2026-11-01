# Design

## Context

Stato attuale su `main`, verificato nel codice:

- **Rimozione**: `remove_source` → `repo::source::delete`: cancella le righe FTS e la sorgente, mentre `media_item` e le tabelle collegate spariscono con `ON DELETE CASCADE`. I file in `<dati>/thumbs` indicati da `media_item.thumb` non vengono mai cancellati, né qui né in `media::delete` (azione bulk "Rimuovi dalla libreria"). Non vengono annullati i job attivi sulla sorgente. La UI (`ImportPage.tsx`) chiede conferma con `window.confirm` e mostra solo il numero di elementi.
- **Aggiornamento**: `start_import(source_id)` accoda il job `import` sull'intera sorgente. Il parametro `paths` limita l'elaborazione a percorsi espliciti (usato per riprovare i falliti). `stage_metadata` salta i file con dimensione e mtime invariati. Riconosce gli spostamenti tramite `quick_hash` + dimensione se il vecchio percorso non esiste più, e li ricollega con `media::relink`. `mark_missing` marca come mancanti i file della sorgente non visti, solo se l'import non è parziale né annullato.
- **File modificati**: un file con contenuto nuovo viene riletto (metadati, miniatura, `content_hash`), ma le righe di `analysis_version` restano quelle della vecchia analisi, quindi l'analisi AI non viene ripetuta.
- **Mancanti**: c'è `repo::media::by_quick_hash` per gli stati `missing`, ma nessun comando di ricollegamento verso una nuova cartella. La rimozione dei mancanti è possibile solo filtrando per stato e usando l'azione bulk.
- **Job**: `JobManager::cancel(id)` esiste; i parametri del job (`sourceId`) permettono di trovare i job di una sorgente.

## Goals / Non-Goals

**Goals:**
- Rimozione di una sorgente senza residui (miniature, dati derivati, job), con un riepilogo che eviti rimozioni accidentali.
- Aggiornamento di una sorgente o di una sottocartella con un riepilogo di cosa è cambiato.
- Ricollegamento e rimozione dei mancanti.
- Rianalisi dei file il cui contenuto è cambiato.

**Non-Goals:**
- Annullare la rimozione di una sorgente: la conferma con riepilogo è la protezione. Reimportare la stessa cartella ricrea le foto, ma senza correzioni manuali.
- Cancellare o spostare file sul disco.
- Unire o dividere sorgenti sovrapposte.

## Decisions

### 1. Rimozione in tre passi
1. `source_removal_summary(id) -> RemovalSummary { items, edited, catalogs: Vec<(name, count)>, named_people_orphaned: Vec<name> }`, in sola lettura. "Edited" = foto con almeno un'etichetta `manual`, un tag o un luogo manuale.
2. `remove_source(id)`:
   - annulla i job `import`/`analysis` attivi o in coda con `sourceId = id`, oppure con `ids` tutti della sorgente, e attende la loro fine (con un timeout dopo il quale la rimozione fallisce con un errore chiaro);
   - raccoglie i percorsi `thumb` (i ritagli dei volti sono nel database e spariscono con la cascata);
   - cancella in una transazione;
   - elimina le persone senza nome rimaste senza volti;
   - cancella i file derivati fuori dalla transazione: un errore di file system viene solo registrato in diagnostica e non annulla la rimozione;
   - aggiorna il watcher.
3. La stessa routine di pulizia dei file derivati viene usata da `media::delete` (azione bulk "Rimuovi dalla libreria" e "Rimuovi mancanti").

Una pulizia una tantum all'avvio (`thumbs` non referenziati da alcun `media_item`), eseguita in background, sistema le librerie già esistenti. Viene marcata come eseguita in `settings_kv` per non ripeterla.

*Alternativa scartata*: cancellare le miniature con un trigger SQL — SQLite non può cancellare file, e un trigger con funzione personalizzata accoppierebbe lo store al file system.

### 2. Riepilogo dell'aggiornamento
`ImportSummary` si estende con i contatori `new`, `modified`, `moved`, `missing`, `restored`, `unchanged` e con le liste di id (limitate a 10.000, oltre si salvano solo i contatori) salvate nel risultato del job. Lo `stage_write` sa già se il file è nuovo (`upsert` senza `existing`), spostato (`moved_from`) o modificato (`existing` con `content_hash` diverso). `mark_missing` restituisce gli id, e i "tornati presenti" sono gli `existing` con stato `missing`. La UI mostra il riepilogo al termine del job (notifica e schermata Job), con collegamenti che impostano un filtro `ids` nella libreria. `LibraryFilter` non ha un filtro per id: va aggiunto `LibraryFilter.ids`.

### 3. Aggiornamento di una cartella
Il job `import` accetta `folder: Option<String>`. La scansione parte da quella cartella, che deve trovarsi dentro la sorgente (altrimenti `invalid_input`), con le stesse regole della sorgente. `mark_missing` considera solo i file della sorgente il cui percorso inizia con `folder + separatore`. Il confronto non distingue maiuscole su Windows, coerentemente con il filtro per cartella di `manual-edits-open-with-ai-tuning`. Per la scelta della cartella si usa il selettore di cartelle di `tauri-plugin-dialog` con partenza dalla sorgente.

### 4. Rianalisi dei contenuti modificati
In `stage_write`, se un elemento esistente ha `content_hash` diverso:
- si cancellano le righe di `analysis_version`, le etichette con origine automatica, `media_people`, l'embedding e i volti non confermati manualmente;
- le etichette manuali, i tag, i cataloghi, il luogo manuale e i volti assegnati manualmente restano;
- il file viene accodato alla fase di analisi successiva come un file nuovo.

I volti assegnati manualmente a un file modificato potrebbero non corrispondere più all'immagine. Restano, ma con una nuova analisi l'associazione automatica può aggiungerne altri. Va documentato.

### 5. Ricollegamento dei mancanti
`relink_missing(source_id, old_folder, new_folder) -> RelinkSummary { relinked, by_hash, still_missing }`, eseguito come job:
1. per ogni mancante sotto `old_folder` si calcola il percorso relativo e si cerca lo stesso file in `new_folder`; se esiste con la stessa dimensione e lo stesso `quick_hash`, viene ricollegato;
2. per i rimanenti si indicizzano i file di `new_folder` per `quick_hash` + dimensione e si ricollega in caso di corrispondenza univoca (verificata con `content_hash`);
3. se `new_folder` è fuori dalla sorgente, la UI propone di aggiungerla come sorgente (o di cambiare il percorso della sorgente se tutti i file sono stati ritrovati). Il ricollegamento aggiorna `source_id` di conseguenza.

`old_folder` viene proposto come la cartella comune più profonda dei mancanti selezionati.

### 6. UI
- **Barra laterale della Libreria**: un pulsante "…" su ogni sorgente apre un menu con "Aggiorna", "Aggiorna cartella…", "Rimuovi dalla libreria…" e "Apri in Import", più "Ricollega mancanti…" e "Rimuovi mancanti…" quando ci sono mancanti.
- **Pagina Import**: "Riscansiona" → "Aggiorna"; rimozione con un dialogo di riepilogo al posto di `window.confirm`; riquadro "Ultimo aggiornamento" per ogni sorgente con il riepilogo e i collegamenti.
- **Job**: nella riga di un import concluso, il riepilogo per categorie.

## Risks / Trade-offs

- [Annullare i job di una sorgente mentre l'utente lavora] → il riepilogo della rimozione indica i job che verranno annullati.
- [Ricollegamento per impronta che sceglie il file sbagliato tra due copie identiche] → si ricollega solo con corrispondenza univoca; i casi ambigui restano mancanti e sono contati nel riepilogo.
- [La pulizia delle miniature all'avvio rallenta il primo avvio su librerie grandi] → eseguita in background con priorità bassa, annullabile, una sola volta.
- [Rianalisi di molti file dopo un ritocco in blocco] → passa per il normale job di analisi con governor e progresso.
- [Conflitti con `manual-edits-open-with-ai-tuning` (albero cartelle in `Sidebar.tsx`, `LibraryFilter.folder`) e con `facet-lists-expand` (`Sidebar.tsx`)] → menu della sorgente in un componente separato (`SourceMenu`); se l'albero cartelle esiste già, "Aggiorna cartella" si aggiunge anche al menu delle cartelle.

## Migration Plan

Nessuna migrazione di schema. Al primo avvio dopo l'aggiornamento parte la pulizia una tantum delle miniature orfane. I risultati dei job di import precedenti non hanno il riepilogo esteso: la UI mostra i soli contatori esistenti. Rollback: revert; le miniature cancellate vengono rigenerate da un aggiornamento della sorgente.

## Implementation Notes

Decisioni prese durante l'implementazione, dove il design lasciava margine o il codice era diverso dal previsto:

- **Riepilogo dell'aggiornamento**: la tabella `job` non ha una colonna per il risultato e il change esclude migrazioni. Il riepilogo dell'ultimo aggiornamento di ogni sorgente si salva quindi in `settings_kv` (`import.last_update.<sorgente>`, `pc_store::sources::UpdateSummary`) con l'id del job. La schermata Job lo mostra nella riga di quel job; gli import precedenti mostrano solo i contatori delle fasi. Un aggiornamento parziale (riprova dei falliti) o annullato non sostituisce il riepilogo.
- **Classificazione**: "modificato" significa contenuto cambiato (`content_hash` diverso). Un file con sola data cambiata conta come invariato. Un mancante tornato presente conta come "tornato presente" anche se il contenuto è cambiato, e in quel caso viene comunque rianalizzato.
- **Miniature**: sono indicizzate per impronta del contenuto e condivise tra duplicati esatti. Si cancellano solo quelle che nessun elemento usa più, anche dopo la rimozione bulk e quando un file modificato cambia miniatura. La pulizia una tantum all'avvio salta i file scritti dopo l'avvio dell'app, perché un import in corso potrebbe non averli ancora registrati.
- **Job della sorgente**: si annullano i job `import`, `analysis` e `relink` con `sourceId` uguale e i job con `ids` tutti della sorgente. Se non si fermano entro 30 secondi la rimozione fallisce con un errore e non cancella nulla.
- **Persone**: l'elenco delle persone include ora quelle con nome rimaste senza volti, con 0 foto (scenario "Persone coinvolte"). Le persone senza nome né identità rimaste senza volti vengono eliminate dopo ogni rimozione o rianalisi.
- **Ricollegamento**: l'esito si salva in `settings_kv` (`import.last_relink.<sorgente>`). Se la nuova cartella è dentro un'altra sorgente, i file ritrovati passano a quella sorgente. Se è fuori da ogni sorgente restano nella sorgente originale e la UI propone due strade. "Aggiungi come sorgente" usa `add_source`, che adotta i file fuori dalla cartella della propria sorgente. "Sposta la sorgente qui" (`move_source`) compare solo se non resta alcun mancante e la cartella precedente era la radice della sorgente.
- **Albero cartelle** (`manual-edits-open-with-ai-tuning`, già in `main`): ogni cartella ha un pulsante ↻ per "Aggiorna cartella". Il confronto dei percorsi (`sources::path_in_folder`) ignora le maiuscole su Windows come il filtro per cartella.
