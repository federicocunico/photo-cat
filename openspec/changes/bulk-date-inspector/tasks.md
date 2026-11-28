# Tasks

D = `design.md`. Issue #68. Solo interfaccia: le azioni bulk `set_manual_date` e `clear_manual_date` esistono già.

## 1. Editor della data

- [x] 1.1 `initialDateState(count, common)` condivisa. `DateEditor` e `ManualDateDialog` partono dalla data manuale comune e dalla sua precisione; altrimenti con "giorno", campi vuoti per più foto e valori di oggi per una sola. Tutte le precisioni restano selezionabili e "Imposta" resta disattivato finché la data non è completa. Verificare con vitest: dialogo del menu con 60 foto (parte vuoto su "giorno" e imposta `value: "1987-03-03"`, `precision: "day"`), con 1 foto (invariato) e con data manuale comune (D3)
- [x] 1.2 `CapturedRow` accetta `dates` `undefined` ("Valori diversi") e la modalità multi. In multi "Ripristina data del file" compare se almeno una foto ha una data manuale o se i valori non sono noti. Verificare con vitest la resa di "Valori diversi" e del ripristino (D2)

## 2. Inspector con più foto

- [x] 2.1 `MultiInspector` mostra "Data di scatto" con il valore comune calcolato come in D1 (data manuale o data del file) e "Valori diversi" per selezioni per filtro o oltre `COMMON_LIMIT`. Verificare con vitest (`Inspector.test.tsx`) gli scenari "Data completa su più foto dall'Inspector" (azione `set_manual_date` con `value: "1976-08-14"` e `precision: "day"` sul target della selezione), "Solo anno su più foto" (`value: "1976"`, `precision: "year"`), "Date diverse nella selezione", "Data manuale comune" e "Ripristino su più foto" (D1)

## 3. Verifica

- [x] 3.1 `make test` in locale, lint UI a parte, `make test-linux` in Docker
- [ ] 3.2 Prova sull'app reale (per l'utente): selezionare un gruppo di scansioni, impostare una data completa dall'Inspector e dal menu contestuale, poi solo l'anno, e annullare

## Workflow follow-up

- Chiudere la issue #68 al rilascio, indicando anche la voce già presente nel menu contestuale.
