# Proposal

## Why

L'issue #51 segnala che dopo l'analisi automatica dei volti restano centinaia di persone e gruppi con una o due foto. Oggi per riassegnarle bisogna aprirle una per una:

- nella sezione "Persone" si seleziona una sola identità alla volta, e l'unione di identità si fa solo dall'editor, una per volta;
- in "Da rivedere" si possono unire più gruppi, ma solo tra quelli selezionati: non si possono mandare in blocco a una persona esistente, né isolare i gruppi più piccoli.

## What Changes

- **Persone: unione di più identità**
  - selezione multipla nell'elenco delle identità (clic, Ctrl/Maiusc+clic, Ctrl+A, Esc);
  - filtro "con al massimo N foto";
  - azione "Unisci in…" che porta tutte le identità selezionate in un'identità di destinazione, scelta con ricerca tra tutte, in un'unica operazione.
  - Le persone della libreria collegate vengono unite nella persona della destinazione. Prima dell'unione, che non si può annullare, compare una conferma con i numeri (identità, foto, volti di riferimento).
- **Da rivedere e Singoli: assegnazione in blocco**
  - filtro per numero di foto ("1 foto", "fino a 2 foto", "fino a N foto") e "Seleziona tutto" sull'elenco filtrato;
  - azione "Assegna a…" che sposta tutti i volti dei gruppi selezionati in una persona o identità scelta con ricerca, anche se non è tra quelle selezionate, oppure in una nuova persona con nome.
  - È un'unica operazione con un solo "Annulla" e funziona con centinaia di gruppi.

## Capabilities

### New Capabilities

Nessuna.

### Modified Capabilities

- `identity-registry`: nuovo requisito "Unione di più identità".
- `face-recognition`: nuovo requisito "Assegnazione in blocco dei gruppi".

## Impact

- **Rust**:
  - `pc-store`: in `app_store/identities.rs` la `merge_many` delle identità, in una transazione; in `people.rs` l'assegnazione di più gruppi a una persona, riusando `merge_many` e la registrazione dell'annullamento esistente;
  - comandi Tauri in `app/src-tauri/src/{identities,people}.rs` e binding.
- **UI** (modulo Persone):
  - `GroupList.tsx`: selezione multipla nella sezione Persone e filtri per numero di foto;
  - dialoghi "Unisci in…" e "Assegna a…" con ricerca, riusando la ricerca di `WhoIs`;
  - i18n it/en.
- **Compatibilità**: nessuna migrazione dello schema.
