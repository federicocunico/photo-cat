# Proposal

## Why

Issue #68: chi scansiona diapositive e negativi si ritrova con foto che portano la data di acquisizione (per esempio 2008) e non quella dello scatto (Praga 1976), e vuole correggere la data di tutto il gruppo insieme. La data di scatto in bulk esiste già dalla 0.5.0, ma solo nel menu contestuale della griglia ("Imposta data di scatto…"). Con più foto selezionate l'Inspector mostra valutazione, colore e luogo, ma non la data. Chi corregge luogo e valutazione dall'Inspector conclude che la data non si possa cambiare in gruppo.

## What Changes

- **Data di scatto nell'Inspector di più foto**: con più foto selezionate, la sezione Dati mostra la riga "Data di scatto", accanto a valutazione, colore e luogo.
  - Se tutte le foto hanno la stessa data (letta dal file o impostata a mano), mostra quella. Altrimenti mostra "Valori diversi".
  - Modificandola si imposta la data di scatto su tutta la selezione. L'avviso offre "Annulla operazione".
  - "Ripristina data del file" agisce su tutta la selezione.
- **Data completa su una selezione**: dall'Inspector e dal dialogo del menu contestuale si imposta su tutta la selezione giorno, mese e anno, oppure una precisione ridotta (mese e anno, solo anno).
  - Se le foto hanno già in comune una data manuale, l'editor parte da quella e dalla sua precisione.
  - Altrimenti parte con precisione "giorno" e con i campi vuoti, così una data precompilata non finisce per sbaglio su tutto il gruppo.
  - Con una sola foto il comportamento resta quello di oggi.

## Capabilities

### New Capabilities

### Modified Capabilities
- `capture-timeline`: la data di scatto manuale, completa o a precisione ridotta, è modificabile anche dall'Inspector con più foto selezionate, con "Valori diversi" e valori iniziali adatti a una selezione, sia nell'Inspector sia nel dialogo del menu contestuale.

## Impact

- UI (`app/ui/src/modules/library`):
  - `Inspector.tsx`: `MultiInspector` aggiunge la riga della data, con i valori comuni calcolati come per luogo e valutazione.
  - `InspectorQuality.tsx`: `CapturedRow` e `DateEditor` accettano il caso "valori diversi" e i valori iniziali per una selezione.
  - `ManualDateDialog.tsx`: valori iniziali secondo il numero di foto e la data comune.
  - i18n se serve una nuova etichetta.
- Backend: nessuna modifica. Le azioni bulk `set_manual_date` e `clear_manual_date`, con annullamento, esistono già.
- Test: vitest su Inspector (più foto: valori comuni, "Valori diversi", data completa, solo anno, ripristino) e sul dialogo del menu contestuale.
