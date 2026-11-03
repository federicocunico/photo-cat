# Tasks

## 1. Backend

- [x] 1.1 Schema v10: `catalog.system`, indice unico, i tre cataloghi; tabella `media_quality`. Test di migrazione.
- [x] 1.2 `Catalog.system` nel modello e nei binding; `delete`/`rename` rifiutano i cataloghi di sistema.
- [x] 1.3 `compile_filter` nasconde le foto dei cataloghi di sistema, salvo aprendo quel catalogo. Test su pagina, conteggio e facet.
- [x] 1.4 Azione bulk `remove_from_system` (Ripristina), annullabile.
- [x] 1.5 `collect_duplicates` e comando Tauri. Test.
- [x] 1.6 Job `photo_quality` e comando per avviarlo. Test con una foto nitida e una sfocata.

## 2. Interfaccia

- [x] 2.1 Cataloghi di sistema in barra laterale e in Cataloghi, con nome tradotto, senza Elimina/Rinomina.
- [x] 2.2 "Sposta nel cestino" nella barra delle azioni; Canc fuori da un catalogo sposta nel cestino. Avviso con "Annulla operazione".
- [x] 2.3 Nel catalogo di sistema aperto, "Ripristina" al posto di "Escludi".
- [x] 2.4 "Metti da parte i duplicati" e "Controlla la qualità" (menu della Libreria e palette).

## 3. Verifica

- [x] 3.1 Typecheck, lint, test UI, `cargo test`, `make test-linux`.
