# Tasks

Ogni gruppo si integra da solo, con i propri test verdi (`npm --prefix app/ui test`, `typecheck`, `lint`, e `cargo test` quando tocca Rust). Riferimenti: P/S/L/E/F = `audit.md`, D = `design.md`.

## 1. Preparazione

- [x] 1.1 Creare il branch `change/ui-usability-audit` da `main` aggiornato, dopo il merge di `unified-selection` (ordine consigliato), e verificare che `make test` passi prima di ogni modifica
- [x] 1.2 Aggiungere `@radix-ui/react-menubar` alle dipendenze dell'interfaccia e verificare che `npm ci` e `npm --prefix app/ui run build` passino

## 2. Difetti e testi (F1–F9)

- [x] 2.1 Persone: `GroupList` passa a `onCatalogChange` le righe nell'ordine mostrato (D5); verificare con un test vitest che, con suggeriti da 26 e 17 foto e altri gruppi da 41 e 24, dopo il nome al primo suggerito si apra quello da 17, e che la freccia giù dall'ultimo suggerito apra il primo di "Altri gruppi" (F1)
- [x] 2.2 Riga della persona: mostrare "cerchia · N foto" oppure solo "N foto", senza "non collegata" né "nessuno"; verificare con un test su una persona collegata senza cerchia (F2)
- [x] 2.3 Primi passi: la voce "catalogo" conta solo i cataloghi senza `system`, e la chiusura va in alto a destra; verificare con un test che i soli cataloghi di sistema non completino la voce (F3, L1)
- [x] 2.4 Fasi: test Rust che scrive `app/ui/src/test/fixtures/phases.json` con tutti i nomi di fase emessi, e traduzioni it/en mancanti (`geo`, `clip`, `persons`, `faces` e altre); verificare con un test vitest che ogni nome abbia una traduzione (F4)
- [x] 2.5 Plurali e decimali: forme `_one`/`_other` per ogni chiave con `{{count}}` (tra cui `people.selected`) e `fmtDec` per i decimali; verificare con un test che scorre le chiavi i18n e con "1 selezionato" in `FaceGrid` (F7, F8)
- [x] 2.6 Sidebar: la riga "+ Nuovo catalogo" senza conteggio; verificare con il test della Sidebar (F9, L2)
- [x] 2.7 Avvisi specifici: eliminare `feedback.done` e `feedback.saved` e dare a ogni `notify` un testo con azione e oggetto ("Gruppo chiamato Gioia", "Importazione di Halloween completata: 54 foto"), con "Annulla operazione" dove l'operazione si annulla; verificare con un test che non restino usi delle chiavi generiche (E6, P16)

## 3. Shell e fondamenta visive

- [x] 3.1 Token tipografici 12/14/16/20 e stile unico `.section-title` (D10); riportare 11, 13, 17 e 18 px ai token in tutti i CSS; verificare con un nuovo test di stile che fallisce su `font-size` fuori scala e su `text-transform: uppercase` (E1, E3)
- [x] 3.2 Monospazio solo per percorsi e codici: togliere la classe `mono` da conteggi, date, nomi di file e segnaposto, con un elenco consentito nel test di stile; verificare che il test passi e che in Persone, Libreria e Attività i conteggi usino il carattere del testo (E2)
- [x] 3.3 Chip colorati solo per stati, mai per uno zero: in `FaceGrid`, Importa ("Errori 0") e Attività; verificare con i test dei componenti (E4, P13)
- [x] 3.4 `ModulePage` con `subtitle`, `tabs` e `actions`; adottarla in Cataloghi, Attività, Libreria e Varianti RAW; verificare con un test che il titolo di ogni modulo stia nello stesso contenitore e con la misura del titolo (x, y) uguale in tutti i moduli, usando lo script di inventario dell'audit (S6)
- [x] 3.5 Un solo contenitore scorrevole per modulo, con `html/body` senza scorrimento; verificare con un test di layout su Esporta e, sull'app reale, che la finestra non abbia barra di scorrimento propria (F5, S4)
- [x] 3.6 Barra di stato su una riga, che resta in fondo anche durante i caricamenti; Sequenza non vi aggiunge scorciatoie né set attivo; verificare con i test di `StatusBar` e di Sequenza (F6, S5)
- [x] 3.7 Navigazione: gruppo `ai` unito in coda a `organize`, etichetta "Analisi AI", voce "Varianti RAW" con la fotocamera nel sottotitolo, etichette dei gruppi senza maiuscolo; verificare con un test che Ctrl+1…9 aprano gli stessi moduli di prima (D9, S3)
- [x] 3.8 Un solo comando per richiudere ogni pannello, nell'intestazione della colonna; togliere i doppioni "<" e ">" in Libreria; verificare con i test di `Panels` e `LibraryHeader` (S7)
- [x] 3.9 Coachmark in linea, con `placement` vincolato, che non copre comandi (D12); verificare con un test che il suo rettangolo non intersechi i pulsanti di Persone, Sequenza, Cataloghi e Tag (S9)
- [x] 3.10 Stati selezionati dalle classi condivise (`data-selected`, `aria-current`) al posto degli stili inline in `GroupList` e `FaceGrid`; verificare con `rg "style=\{\{\s*border"` senza risultati nel modulo Persone (E5, P15)

## 4. Barra dei menu

- [x] 4.1 Registro dei comandi: campi `menu` e `disabledReason` (D6), con i comandi globali che mancano (Importa una cartella…, Esporta…, Apri una cartella di RAW…, Apri un progetto FujiView…, Mostra la cartella dei dati, Esci, Rianalizza le foto…, Metti da parte i duplicati, Controlla la qualità, Mappa delle foto, Schermo intero, Copia le informazioni di sistema); verificare con un test che ogni voce dei requisiti di `application-menu` esista nel registro con lo stesso nome della palette
- [x] 4.2 Scorciatoie Ctrl+O e Ctrl+E; verificare con i test del registro che non entrino in conflitto con altre scorciatoie e che compaiano nel foglio delle scorciatoie
- [x] 4.3 Componente `MenuBar` (Radix Menubar) nella `TopBar` con File, Modifica, Vista, Vai, Strumenti e Aiuto, voci disattivate con il motivo; verificare con test vitest dell'apertura, delle scorciatoie accanto alle voci e del motivo di una voce disattivata
- [x] 4.4 Tastiera: Alt premuto e rilasciato da solo, oppure F10, porta il focus sul primo menu; frecce, Invio, Esc e lettere di accesso; verificare con test che Alt+1 non apra il menu e che Esc riporti il focus dov'era
- [x] 4.5 Rimuovere `HelpMenu` dalla `TopBar` e portarne le voci nel menu Aiuto, insieme a "Copia le informazioni di sistema"; verificare con il test del menu Aiuto (in-app-guidance)
- [x] 4.6 macOS: comando `set_app_menu`, menu nativo di Tauri con il menu dell'app (Informazioni, Impostazioni… ⌘,, Esci ⌘Q), evento `menu://command` inoltrato al registro, ricostruzione al cambio di lingua; `MenuBar` nascosto su macOS; verificare con un test Rust sulla costruzione dell'elenco delle voci e con un test vitest sull'inoltro dell'evento
- [x] 4.7 Aggiornare il README (Uso dell'app: barra dei menu, Alt/F10, nuove scorciatoie) e verificare che la tabella delle scorciatoie coincida con il foglio delle scorciatoie

## 5. Informazioni su photo-cat

- [x] 5.1 `build.rs`: `PC_BUILD_COMMIT`, `PC_BUILD_DATE` (da `SOURCE_DATE_EPOCH` se presente) e `PC_BUILD_PROFILE`, con commit vuoto senza git; aggiungere `repository` in `[workspace.package]`; verificare con un test Rust che `app_info` riporti versione, commit o "commit non disponibile", data e configurazione
- [x] 5.2 `app_info` esteso con sistema, architettura, versione del motore della finestra, acceleratore attivo, autori, repository e licenza; rigenerare `bindings.ts`; verificare con il test del comando e con `npm run typecheck`
- [x] 5.3 `scripts/licenses.mjs` (sostituisce `ui-licenses.mjs`): dipendenze Rust da `cargo metadata`, dipendenze di produzione dell'interfaccia e `assets/licenses/manual.json` per modelli e librerie native, uniti in `assets/licenses/third-party.json`; fallisce per una licenza non riconosciuta; verificare eseguendo lo script e con un test sul caso di licenza mancante
- [x] 5.4 CI: il job Linux rigenera `third-party.json` e fallisce se differisce da quello nel repository; verificare con `make test-linux`
- [x] 5.5 Comando Rust `third_party_licenses` che legge il file dalle risorse installate; verificare con un test d'integrazione su un file di esempio
- [x] 5.6 Componente `AboutPanel` con le schede "Informazioni" e "Licenze di terze parti" (ricerca, gruppi, testo completo, nota sulle sorgenti per LGPL), "Apri" accanto al percorso dei dati e "Copia informazioni" senza il percorso; verificare con test vitest della ricerca "LGPL" e del testo copiato
- [x] 5.7 Finestra Informazioni dal menu Aiuto e sezione Impostazioni › Informazioni con lo stesso `AboutPanel`; verificare con i test di Impostazioni (ricerca "versione" porta alla sezione) e del menu Aiuto

## 6. Persone

- [x] 6.1 Componente condiviso `ActionBar` con priorità e menu "Altro", senza andare a capo (D2); verificare con test vitest a larghezze 1024 e 1440 che nessun pulsante abbia testo che esce e che le azioni in eccesso finiscano in "Altro"
- [x] 6.2 Portare la `SelectionBar` della Libreria su `ActionBar`; verificare che i test esistenti della Libreria passino senza cambiare comportamento
- [x] 6.3 Intestazione della pagina con le schede "Da rivedere / Persone / Ignorati" (`Tabs`) e intestazione dell'elenco con ricerca e menu "Ordina e filtra" (ordinamento, poche foto, cerchia); verificare con un test che sopra il primo gruppo ci siano solo ricerca e menu (P3)
- [x] 6.4 Selezione dell'elenco con il modello comune: clic seleziona e apre, Ctrl/Maiusc+clic e casella per la selezione multipla, barra in fondo all'elenco con "Unisci a…", "Ignora" e "Deseleziona" solo con due o più voci; togliere i pulsanti "Seleziona tutto/Deseleziona/Inverti"; aggiornare `BulkPeople.test.tsx` e verificare gli scenari di "Apertura e selezione in Persone" (P4)
- [x] 6.5 Volti: sopra la griglia solo "Ordina" e "Vista"; con volti selezionati, `ActionBar` con "Sposta in…", "Non è questa persona" e "Altro" (Nuovo gruppo con questi volti, Usa come copertina con un solo volto, Rimuovi volti…); aggiornare `FaceGrid*.test.tsx` e verificare gli scenari di "Comandi di correzione dei volti" (P1, P14)
- [x] 6.6 `PersonPicker` in modalità `destination` (persone, poi gruppi senza nome, con copertina e foto, e "Nuova persona «…»") per "Sposta in…" e "Unisci a…"; rimuovere le select "Riassegna a…" e "Unisci con…"; l'unione di identità mantiene anteprima e conferma; verificare con i test del selettore e di `PeopleCorrections` (P2, P8)
- [x] 6.7 Scheda del gruppo: "È {nome}?" o "È la stessa persona?" in cima con le due copertine, Sì/No e somiglianza in parole; poi "Chi è?"; poi Mostra in Libreria, Unisci a…, Ignora; stati vuoti non duplicati, con la scheda giusta per volti singoli e volti scartati; verificare con gli scenari di "Suggerimento di identità nella scheda" (P8, P11)
- [x] 6.8 Scheda della persona: intestazione con copertina, "Mostra in Libreria" e menu ⋯ (Unisci a…, Esporta, Elimina persona…); schede "Volti nella libreria" e "Volti di riferimento"; dettagli salvati subito con "Salvato" (Nome, Nome nei file esportati, Cerchia, Altri nomi, Riconoscimento come sensibilità); niente "Apri in Persone" né barra di scorrimento orizzontale; aggiornare `IdentityEditor.test.tsx` (P10, P9)
- [x] 6.9 Volti senza punteggi: anno e score fuori dal riquadro, icona con spiegazione solo per la qualità bassa; intestazione del gruppo con "N foto · N volti" senza chip; nomi dell'elenco non troncati dal conteggio; verificare con test dei componenti e con lo script di inventario (nessun testo "0,xx" in Persone) (P9, P12, P13)
- [x] 6.10 Glossario di Persone in it/en (persona, persona senza nome, cerchia, Sposta in…, Unisci a…) ed elenco di parole vietate nel test del glossario; verificare che il test passi (P6)

## 7. Altri moduli

- [x] 7.1 Primo avvio: un solo titolo per passo, "Lo faccio dopo" solo dove rimanda qualcosa, testo coerente sui modelli già pronti, controlli segmentati larghi quanto il contenuto; verificare con i test di `Onboarding`
- [x] 7.2 Attività: barre determinate quando il conteggio è noto, nessuna attività "Completata 0 / 0" mostrata come lavoro, intestazioni senza maiuscolo; verificare con i test di `JobsPage`
- [x] 7.3 Sequenza: griglia che riempie la larghezza, nome della fotocamera non ripetuto in ogni miniatura (iniziale e colore restano), etichetta "JPGS" in parole, "Esporta sequenza…" nell'intestazione della pagina; verificare con i test di Sequenza
- [x] 7.4 Tag: colori predefiniti distinti per i tag del set Base, "Set attivo" come stato e non come pulsante disattivato, intestazioni senza maiuscolo; verificare con i test di `TagsPage`
- [x] 7.5 Libreria: badge delle miniature con etichetta accessibile e descrizione al passaggio del mouse; Inspector con le catalogazioni AI in parole e i volti con nome o "persona senza nome"; verificare con i test di `Grid` e `Inspector` (L3, L4)
- [x] 7.6 Impostazioni: cache delle anteprime di Varianti RAW in Avanzate; verificare con il test della ricerca "cache"

## 8. Verifica d'insieme

- [x] 8.1 Ripetere le misure dell'audit con lo script di inventario sull'app di prova: titoli nella stessa posizione, corpi solo 12/14/16/20, nessun maiuscolo, nessun pulsante d'azione disattivato in Persone senza selezione; riportare i valori in `audit.md` §2 come "dopo"
- [x] 8.2 `make test` in locale e `make test-linux` in Docker, con le correzioni necessarie
- [x] 8.3 Build NSIS locale (`npm --prefix app/ui run tauri -- build --ci --bundles nsis`) e verifica che `licenses/third-party.json` sia nell'installer
- [ ] 8.4 Prova sull'app reale (per l'utente): primo avvio, revisione di Persone da tastiera, Sposta in e Unisci a, barra dei menu con Alt e F10, Informazioni e Copia informazioni, tema chiaro e scuro, finestra a 1024 px

## Workflow follow-up

- Aprire la PR, attendere la CI verde e fare il merge, come da flusso abituale.
- Archiviare la change al momento della release (`openspec archive ui-usability-audit --yes`).
