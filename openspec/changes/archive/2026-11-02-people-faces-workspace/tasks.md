# Tasks

> **Note per l'agente implementatore — leggere prima di iniziare**
>
> - **Design dell'interfaccia**: il riferimento è `mockups/People.html` in questa cartella. Leggere prima `mockups/README.md`, che dice cosa è vincolante. Non esistono altre versioni del design a cui fare riferimento.
> - **Mai cancellare foto o file**: ignorare, scartare e unire agiscono solo sui dati dei volti. "Rimuovi volto" è l'unica azione che cancella dati (del volto, non della foto) e chiede conferma. Per le prove usare `D:\Foto` solo in lettura con una cartella dati separata (`PHOTO_CAT_DATA_DIR`), mai la cartella dati dell'utente.
> - **Coordinamento**:
>   - `large-library-stability`: paginazione dei volti, niente polling, comandi `async`;
>   - `loading-feedback`: `QueryView`, `BusyButton`, `Toast`;
>   - `similar-photo-stacks`: `pc-ai/src/quality.rs` e il requisito "Soglie dell'automatismo configurabili" in `ai-classification`;
>   - `catalog-exclusions`: numero di migrazione.
>
>   Prima di iniziare controllare cosa è già in `main` e riusarlo. Prima dell'archiviazione, allineare le delta ai requisiti già archiviati dagli altri change.
> - **Compatibilità**: identità, persone con nome, rifiuti e correzioni esistenti devono restare validi dopo la migrazione. Se un passaggio sembra richiedere di perderne una parte, fermarsi e chiedere.

## 1. Dati e qualità

- [x] 1.1 Migrazione di schema: `face.sharpness`, `face.frontality`, `face.size_px`, `face.quality_score`, `face.quality_override`, `person.ignored_at`, tabelle `person_not_same` e `person_suggestion`, indici per la paginazione (design §1–§3, Migration Plan). Verifica: test di migrazione dall'ultima versione con persone, identità collegate, rifiuti e volti manuali invariati.
- [x] 1.2 `pc-ai/src/quality.rs` (nitidezza; se esiste già da `similar-photo-stacks`, riusarla), frontalità dai punti di riferimento e qualità combinata in `face_analysis.rs`, soglia `ai.thresholds.face_min_sharpness` in `config.rs` (design §2, §6). Verifica: unit test con ritagli sintetici (nitido, sfocato con kernel gaussiano, piccolo, di profilo) e scenario "Volto mosso".
- [x] 1.3 Esclusione dei volti scartati dal raggruppamento e dai centroidi (salvo manuali e recuperati); gruppi ignorati protetti e attrattori; vincoli `person_not_same` in `cluster.rs`. Verifica: test in `crates/pc-ai/tests/` per "Nuove foto della persona ignorata", "Persone diverse" e "Nessuna persona creata automaticamente".
- [x] 1.4 Job di manutenzione `face_quality` per i volti esistenti (dai ritagli, senza modelli), annullabile e riprendibile, avviato una volta dopo la migrazione; estensione del job `retune` con `faces_quality`. Verifica: test su un catalogo con volti senza qualità, e test per "Soglia di nitidezza cambiata" senza esecuzione di modelli.
- [x] 1.5 Suggerimenti di unione da centroidi con le regole del design §3. Verifica: test per "Gruppi probabilmente uguali", per l'esclusione delle coppie "persone diverse" e per il limite di un suggerimento per gruppo.

## 2. Comandi

- [x] 2.1 `people_sections`, `list_groups` e `group_faces` paginati per cursore, con esclusione degli ignorati dalle query di Libreria (filtro, Inspector, facet, conteggi, FTS); rigenerare `bindings.ts`. Verifica: test Rust di paginazione senza duplicati né buchi e per "Persona che non interessa" (foto ancora in libreria, persona assente dal filtro).
- [x] 2.2 `correct_person` esteso (`MergeMany`, `Move`, `NotSame`, `Ignore`, `Restore`, `Recover`, `SetCover`), annullabile come le correzioni esistenti, ed evento `faces://changed`. Verifica: test per "Unione multipla", "Ripristino", "Recupero di un volto" e per l'evento emesso dopo ogni correzione.
- [x] 2.3 `locate_face` per volto o foto. Verifica: test che restituisce sezione, gruppo e indice corretti per un volto in una persona con 400 volti, in un gruppo da rivedere, in un gruppo ignorato e per un volto scartato.

## 3. Collegamento con le identità

- [x] 3.1 Rimuovere ogni creazione implicita di identità diversa dalla migrazione v5; il collegamento da "Chi è?" porta il gruppo in "Persone" e unisce se l'identità è già collegata. Verifica: test per gli scenari "Collegamento" e "Collegamento a un'identità già collegata" di `identity-registry`.

## 4. Interfaccia

- [x] 4.1 Struttura del modulo `modules/people/PeopleModule.tsx` a tre colonne con `GroupList` (sezioni, ricerca, ordinamento, virtualizzazione, selezione multipla), secondo `mockups/People.html`; rimozione della voce `identities` da `registry.ts` e migrazione della preferenza del modulo. Verifica: test con `ipcMock` per le sezioni e i conteggi, e per "Preferenza del modulo non più esistente".
- [x] 4.2 `FaceGrid` virtualizzata a pagine con selezione (clic, Ctrl, Maiusc, rettangolo), barra delle azioni (Sposta in…, Non è questa persona, Nuovo gruppo, Rimuovi volto con conferma), filtro "Solo scartati per qualità" con motivo in tooltip, banner del suggerimento di unione. Verifica: test con `ipcMock` per selezione e azioni, e per "Libreria grande" con 100.000 volti simulati (numero di nodi DOM limitato).
- [x] 4.3 `PersonCard`: "Chi è?" (riuso di `WhoIs`), copertina, unione, ignora; per le persone, `IdentityEditor` estratto da `IdentitiesPage.tsx` con riferimenti, alias, gruppo, soglia, unione ed esporta/importa (`Transfer`). Portare i test di `IdentitiesPage.test.tsx` sul nuovo modulo ed eliminare `PeoplePage.tsx` e `IdentitiesPage.tsx`. Verifica: i test portati passano, più il test per "Identità senza foto nella libreria".
- [x] 4.4 Dialogo di unione (anche con scelta del nome), "Sono persone diverse" annullabile, sezione Ignorati con "Ripristina" e "+N volti nuovi". Verifica: test per "Unione persone", "Persone diverse" e "Ripristino" lato UI.
- [x] 4.5 `openPeople(target)` in `store/layout.ts`, evidenziazione del volto e avviso "Torna alla foto"; chip dei volti nell'Inspector con "Apri in Persone"; stesso comando dai filtri e dalla Sequenza. Verifica: test per "Dall'Inspector", "Volto non riconosciuto" e "Volto ignorato o scartato".
- [x] 4.6 Scorciatoie da tastiera del mockup (disattivate nei campi di testo), aggiornamento su `faces://changed` senza perdere selezione e scorrimento, stati di caricamento/vuoto/errore. Verifica: test per "Revisione da tastiera" e "Aggiornamento dopo l'analisi".
- [x] 4.7 "Nitidezza minima dei volti" in Impostazioni › Soglie AI con anteprima. Verifica: test in `SettingsPage.test.tsx` per lo scenario "Nitidezza minima dei volti".
- [x] 4.8 Chiavi i18n it/en per tutto il modulo, e rimozione delle chiavi delle pagine eliminate non più usate. Verifica: test esistente sulle chiavi mancanti (o un nuovo test che confronta le chiavi it/en).

## 5. Verifica d'insieme

- [x] 5.1 Eseguire `make test` (fmt, clippy, test Rust e UI). Verifica: tutto verde.
- [x] 5.2 Taratura e prova sull'app reale con `D:\Foto` in sola lettura e una cartella dati separata:
  - importare e analizzare;
  - tarare la curva di nitidezza e la soglia predefinita, annotandole in Open Questions del design;
  - rivedere gruppi, dare nomi, unire i doppioni suggeriti, ignorare uno sconosciuto;
  - aprire in Persone da tre foto diverse.

  Verifica: tutti i passi riusciti, `D:\Foto` invariata (numero di file e date di modifica), nessun errore nuovo in Diagnostica.
- [x] 5.3 Aggiornare il `README.md` (modulo Persone, ignorati, qualità dei volti), allineare le delta e verificare che `openspec validate people-faces-workspace --strict` passi.

## Workflow follow-up

- Coordinare il merge con `large-library-stability`, `loading-feedback`, `similar-photo-stacks` e `catalog-exclusions`.
- Archiviare il change dopo la revisione.
