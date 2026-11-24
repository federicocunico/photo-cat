# Proposal

## Why

In Persone › Persone ogni persona compare con un cerchio con le iniziali, e la scheda con un cerchio vuoto. I gruppi da rivedere hanno invece il loro volto di copertina. "Usa come copertina" esiste già, ma solo nel menu "Altro" della barra che compare con un volto selezionato. La copertina scelta poi non si vede nell'elenco delle persone, né nella scheda. Chi vuole scegliere l'immagine che rappresenta una persona non trova dove farlo, e anche quando ci riesce non vede il risultato.

## What Changes

- **Copertina delle persone**: l'elenco di Persone e la scheda della persona mostrano il volto di copertina della persona nella libreria aperta.
  - Se l'utente l'ha scelto, è quel volto; altrimenti è il volto migliore, come per i gruppi da rivedere.
  - Le iniziali restano solo per le persone senza volti nella libreria aperta.
- **Scelta dal volto**: "Usa come copertina" compare anche nel menu contestuale di un volto (clic destro), oltre che in "Altro" con un solo volto selezionato. Vale sia in "Da rivedere" sia in "Persone".
  - Dopo la scelta, elenco, scheda e selettore mostrano subito il nuovo volto.
  - L'avviso offre "Annulla operazione".

## Capabilities

### New Capabilities

### Modified Capabilities
- `face-recognition`: copertina delle persone nell'elenco e nella scheda; "Usa come copertina" anche nel menu contestuale del volto.

## Impact

- Backend (`app/src-tauri/src/identities.rs`): `IdentityRow` riceve `cover` (volto di copertina della persona collegata), calcolato solo da `list_identities`. In `pc-store` una funzione pubblica per la copertina di una persona, con la stessa regola dei gruppi.
- UI (`modules/people`):
  - elenco delle persone con `Cover` al posto delle iniziali;
  - la scheda riceve la copertina dell'identità;
  - voce "Usa come copertina" nel menu contestuale dei volti;
  - binding rigenerati.
- Test: Rust (copertina scelta e automatica nell'elenco delle identità) e vitest (elenco, scheda, menu contestuale).
