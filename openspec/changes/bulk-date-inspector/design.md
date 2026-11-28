# Design

## Context

- `MultiInspector` (`app/ui/src/modules/library/Inspector.tsx`) calcola i valori comuni solo per selezioni esplicite fino a `COMMON_LIMIT`. Per farlo carica `mediaDetail` di ogni foto. Oltre quel limite, o con selezione per filtro, `known` è falso e le righe mostrano "Valori diversi". Le modifiche passano da `useEditing(target, count).apply`, cioè dal job bulk con "Annulla operazione".
- `CapturedRow` e `DateEditor` (`InspectorQuality.tsx`) servono oggi una sola foto. Ricevono `MediaDates` e la data del file già formattata, e `DateEditor` parte con precisione `"day"` quando non c'è una data manuale.
- `ManualDateDialog` (menu contestuale della griglia) parte sempre con precisione `"day"` e la data di oggi.
- Il backend ha già `set_manual_date` e `clear_manual_date` come azioni bulk annullabili, per id e per filtro.

## Goals / Non-Goals

**Goals:**
- Riga "Data di scatto" in `MultiInspector`, con lo stesso modello di valori comuni delle altre righe.
- Data completa (giorno, mese, anno) o a precisione ridotta su tutta la selezione, dall'Inspector e dal menu contestuale.
- Valori iniziali dell'editor sicuri per una selezione.

**Non-Goals:**
- Cambiare solo una parte della data (per esempio l'anno) mantenendo il resto della data di ciascuna foto: la data impostata è la stessa per tutta la selezione.
- Modifiche al backend o all'annullamento.
- Spostare o duplicare le voci del menu contestuale.

## Decisions

### D1. Valore comune della data

In `MultiInspector`, quando `known` è vero, il valore comune è una chiave per foto:
- per una data manuale, periodo e precisione (`"1984-06" | month`);
- altrimenti, la data del file formattata come nell'Inspector di una sola foto.

Se tutte le chiavi coincidono, la riga mostra lo stesso testo dell'Inspector singolo ("giugno 1984 (impostata a mano)" o la data del file). Se no, mostra "Valori diversi" (`common.mixed`). Per una selezione per filtro o troppo grande la riga mostra sempre "Valori diversi" ed è comunque modificabile, come luogo e valutazione.

Alternativa scartata: interrogare il backend per un riepilogo delle date della selezione. Per un'informazione accessoria sarebbe un comando nuovo, mentre i dettagli sono già caricati per le altre righe.

### D2. `CapturedRow` con valore "diverso"

`CapturedRow` accetta `dates: MediaDates | null | undefined`. `undefined` significa valori diversi, come in `RatingRow` e `ColorTraitRow`. Accetta anche `multi?: boolean`.
- In modalità multi, "Ripristina data del file" compare se almeno una foto ha una data manuale, oppure se i valori non sono noti.
- La riga "Fonte della data" resta solo nell'Inspector singolo.

### D3. Valori iniziali

Una funzione pura, `initialDateState(count, common)`, dove `common` è la data manuale comune con la sua precisione, oppure niente:
- con una data manuale comune, parte da quella data e dalla sua precisione;
- altrimenti parte con precisione `"day"`: con più foto i campi giorno, mese e anno sono vuoti, con una sola foto valgono come oggi (data di oggi nel dialogo, campi dell'Inspector singolo invariati).

La usano `DateEditor`, tramite `CapturedRow`, e `ManualDateDialog`, che riceve già `count`. Tutte e tre le precisioni restano sempre selezionabili. I campi vuoti tengono disattivato "Imposta" finché la data non è completa e valida per la precisione scelta: una data di oggi precompilata, confermata per sbaglio su 120 scansioni, sarebbe un errore silenzioso.

Alternative scartate:
- Precisione "anno" di default con più foto: costringe a cambiare precisione per impostare una data completa, che è il caso normale.
- Data di oggi precompilata anche con più foto: è il rischio di conferma per sbaglio descritto sopra.

## Risks / Trade-offs

- [Il confronto delle date del file usa il testo formattato] → Due istanti diversi nello stesso minuto risultano "uguali". È accettabile: la riga mostra lo stesso testo per entrambe.
- [I campi vuoti nel dialogo del menu cambiano un'abitudine] → Vale solo con più foto. Con una foto il dialogo resta com'è.
