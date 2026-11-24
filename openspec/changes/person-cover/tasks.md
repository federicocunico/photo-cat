# Tasks

D = `design.md`. Build con `CARGO_BUILD_JOBS` ≤ 4 e `CARGO_PROFILE_DEV_DEBUG=0`, un solo processo cargo alla volta.

## 1. Backend

- [x] 1.1 `pc-store::people::person_cover` (volto scelto o migliore) e `IdentityRow.cover` calcolato da `list_identities`; test Rust su copertina automatica, scelta e persona senza volti; binding rigenerati (D1)

## 2. Interfaccia

- [x] 2.1 Elenco di Persone › Persone con `Cover` al posto delle iniziali, iniziali solo senza volti; la scheda mostra la copertina dell'identità; verificare con vitest gli scenari di "Copertina delle persone" (D2)
- [x] 2.2 "Usa come copertina" nel menu contestuale di un solo volto, con avviso annullabile; verificare con vitest lo scenario "Copertina dal menu contestuale" (D3)

## 3. Verifica

- [ ] 3.1 `make test` in locale, `make test-linux` in Docker e build NSIS locale
- [ ] 3.2 Prova sull'app reale (per l'utente): scegliere la copertina di una persona dal clic destro e vederla nell'elenco e nella scheda
