# Audit di usabilità ed estetica di photo-cat

Data: 2026-11-23.

**Base analizzata**: `main` 8f858bf (release 0.5.1 più #54), build release del 23 novembre.

**Metodo**:
- app reale avviata con un catalogo isolato (`PHOTO_CAT_DATA_DIR` e cartella WebView2 separate: il catalogo e le preferenze dell'utente non sono stati toccati);
- primo avvio completo, poi importazione di due cartelle di `D:\Foto` (Halloween 2026, 54 JPG; "Gioia cambia lavoro" 2025, 132 JPG): 186 foto, 7 gruppi di volti, 79 volti scartati, analisi AI completa. Dopo la prova le due cartelle risultano senza file creati o modificati;
- percorsi reali: dare un nome a un gruppo, passare alla sezione Persone, selezionare volti, menu contestuale, volti scartati, tema scuro, menu Aiuto e Informazioni;
- screenshot e misure sul DOM via Chrome DevTools Protocol, sempre a 1440×920 px: comandi visibili, disattivati, bersagli sotto 24 px, corpi di testo, monospazio, maiuscolo, posizione dei titoli, scorrimento;
- lettura del codice di `app/ui/src` per trovare la causa di ogni difetto.

Gli screenshot contengono foto e volti di persone reali. Per questo **non sono nel repository**: restano nella cartella temporanea della sessione. I mockup della proposta usano segnaposto (vedi `mockups/`).

**Legenda**: le priorità sono **A** = alta, **M** = media, **B** = bassa. I riferimenti `file:riga` sono relativi a `app/ui/src`. Gli identificativi (P1, S1…) sono usati in `design.md` e `tasks.md`.

---

## 1. Sintesi

L'app è completa e veloce. La revisione di novembre ha portato componenti condivisi, palette dei comandi, navigazione a gruppi e un linguaggio più chiaro. I problemi di oggi nascono soprattutto da quello che è stato aggiunto dopo, una change alla volta: unione multipla, assegnazione in blocco, Varianti RAW, Analisi AI.

1. **Persone è il punto più debole.** In una sola schermata convivono due selezioni indipendenti, ciascuna con il proprio "Seleziona tutto". Ci sono cinque verbi per spostare volti (Riassegna, Assegna, Unisci con, Unisci in, Sono la stessa persona). La barra dei volti ha 11 comandi su tre righe, 5 dei quali disattivati quando non c'è selezione. La parola "gruppo" ha tre significati diversi.
2. **Comandi sempre visibili invece di comandi che compaiono quando servono.** Persone, Sequenza e Tag mostrano tutti i pulsanti anche quando non si possono usare. In Libreria, invece, la barra della selezione compare solo con una selezione. Lo stesso principio non vale ovunque.
3. **Difetti funzionali visibili**:
   - dopo aver dato un nome, il gruppo successivo non è quello mostrato sotto;
   - la riga di una persona dice "non collegata" mentre la scheda dice "collegata";
   - "Crea un catalogo" risulta fatto senza aver creato cataloghi;
   - le fasi delle attività compaiono con il nome interno (`geo`, `clip`, `persons`, `faces`);
   - in Esporta scorre l'intera finestra.
4. **Estetica incoerente**:
   - 8 corpi di testo;
   - 7 stili diversi per le intestazioni di sezione;
   - titoli di pagina in tre posizioni diverse;
   - monospazio usato per contare;
   - etichette in maiuscolo;
   - chip colorati anche per i valori a zero.
5. **Mancano le convenzioni da app desktop.** Non c'è una barra dei menu. La versione si trova solo nell'ultima voce di un menu a icona, e la finestra "Informazioni" mostra solo versione e percorso dei dati: niente build, licenze di terze parti (che sono già nell'installazione, in `licenses/`), autori o copia delle informazioni per una segnalazione.
6. **Numeri da sviluppatore ancora in vista**: punteggi 0–1 sotto ogni volto e nell'Inspector, "Somiglianza minima 0.36" con il punto decimale, "Somiglianza 0,36" nei suggerimenti. La revisione precedente aveva rimandato questi punti a "una passata dedicata" (§9 di `ui-ux-overhaul/audit.md`): è questa.

---

## 2. Misure

Elementi visibili nella finestra, a 1440×920 px, sopra il contenuto del modulo (senza barra superiore e navigazione).

| Modulo | Comandi | Pulsanti | Disattivati | Corpi di testo | Testi in monospazio | Maiuscolo | Titolo pagina (x, y) |
|---|---:|---:|---:|---|---:|---|---|
| Libreria | 51 | 22 | 0 | 11, 12, 13, 14, 16 | 10 | — | nessuno |
| Sequenza | 36 | 29 | 3 | 11, 12, 14, 16, 18, 20 | 29 | — | 224, 67 |
| Cataloghi | 9 | 9 | 0 | 11, 12, 14, 16, 20 | 1 | — | **248, 97** |
| Persone (nessun gruppo aperto) | 21 | 13 | 2 | 12, 14, 20 | 10 | — | 224, 67 |
| Tag | 31 | 27 | 2 | 11, 12, 13, 14, 18, 20 | 8 | 4 | 224, 67 |
| AI | 35 | 35 | 0 | 12, 13, 14, 16, 20 | 2 | — | 224, 67 |
| Importa | 24 | 12 | 1 | 12, 13, 14, 16, 20 | 3 | — | 224, 67 |
| Esporta | 34 | 7 | 1 | 12, 14, 16, 20 | 1 | — | 224, 67 (**scorre il documento**) |
| Attività | 3 | 3 | 0 | 11, 12, 13, 14, 16, 17, 20 | 6 | 7 | **256, 91** |
| Impostazioni | 20 | 16 | 1 | 12, 13, 14, 20 | 2 | — | 224, 67 |

Intestazioni di sezione trovate: 11/600 maiuscolo, 14/700, 16/600, 16/700, 17/700, 18/700, 20/600.

In Persone, aprendo un gruppo, la sola barra dei volti aggiunge 11 comandi, di cui 5 disattivati finché non si seleziona un volto. L'intestazione dell'elenco occupa **411 px su 779** della colonna (53%) prima del primo gruppo.

### Dopo (task 8.1, 23/11/2026)

Stesso script d'inventario, sulla build release della change e su una copia del catalogo di prova isolato (186 foto da `D:\Foto`), a 1440×900 px. Con la barra dei menu la finestra parte 18 px più in alto, quindi il titolo sta a y = 65 invece di 67.

| Modulo | Comandi | Pulsanti | Disattivati | Corpi di testo | Testi in monospazio | Maiuscolo | Titolo pagina (x, y) |
|---|---:|---:|---:|---|---:|---|---|
| Libreria | 48 | 21 | 0 | 12, 14, 16, 20 | 0 | — | 224, 65 |
| Sequenza | 32 | 9 | 3 | 12, 14, 16, 20 | 6 (tasti 1–6) | — | 224, 65 |
| Cataloghi | 12 | 9 | 0 | 12, 14, 16, 20 | 0 | — | 224, 65 |
| Persone (nessun gruppo aperto) | 15 | 8 | **0** | 12, 14, 20 | 0 | — | 224, 65 |
| Tag | 30 | 26 | 1 | 12, 14, 16, 20 | 6 (tasti 1–6) | — | 224, 65 |
| Analisi AI | 35 | 35 | 0 | 12, 14, 16, 20 | 0 | — | 224, 65 |
| Importa | 24 | 12 | 1 | 12, 14, 16, 20 | 2 (percorsi) | — | 224, 65 |
| Esporta | 34 | 7 | 1 | 12, 14, 16, 20 | 0 | — | 224, 65 (la finestra non scorre) |
| Attività | 5 | 5 | 0 | 12, 14, 16, 20 | 0 | — | 224, 65 |
| Impostazioni | 19 | 16 | 0 | 12, 14, 20 | 1 (percorso) | — | 224, 65 |

- Il titolo è nello stesso punto in tutti i moduli. I corpi di testo sono solo 12, 14, 16 e 20. Non c'è testo in maiuscolo. Il monospazio resta solo per tasti e percorsi.
- `html` e `body` hanno `overflow: hidden` e il documento è alto quanto la finestra (900 px), anche in Esporta.
- I pulsanti disattivati confermano qualcosa che non c'è ancora:
  - Sequenza, nel pannello "Dispositivi e orari": "Allinea le due foto selezionate", "Annulla modifiche" e "Applica (0 gruppi)", finché non si selezionano due foto o non si cambia un orario;
  - Tag, Importa ed Esporta: "Aggiungi" accanto a un campo ancora vuoto.
- Persone con un gruppo aperto ha 40 comandi, 15 pulsanti e nessun pulsante disattivato. La barra dei volti compare solo con dei volti selezionati.
- Il primo gruppo dell'elenco sta a **91 px su 757** dall'alto della colonna (12%). Sopra ci sono solo la ricerca e "Ordina e filtra".

---

## 3. Persone

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| P1 | La barra dei volti ha 11 comandi che vanno su **tre righe** anche a 1440 px: "Riassegna a…" più "Riassegna", "Non è questa persona", "Nuovo gruppo con i selezionati", "Rimuovi volto", "Seleziona tutto", "Inverti selezione", "Ordina per", "Mostra: Tutti / Solo scartati per qualità". Senza selezione 5 sono disattivati. Azioni sulla selezione, comandi di selezione e comandi di vista stanno nella stessa fila, senza gerarchia. | `modules/people/FaceGrid.tsx:567-720` | A |
| P2 | **Cinque verbi per spostare volti**: "Riassegna a…" + "Riassegna" (volti, select più pulsante); "Assegna a…" (gruppi in blocco, e volti singoli); "Unisci con" (scheda, select più pulsante); "Unisci in…" (identità in blocco); "Sono la stessa persona" (suggerimento). Chi usa il modulo deve capire che sono la stessa operazione su oggetti diversi. | `FaceGrid.tsx:583-616,722-765`, `GroupList.tsx:693-712`, `PersonCard.tsx:241-260` | A |
| P3 | L'intestazione dell'elenco usa 411 px: schede su due righe ("Ignorati" va a capo), tre chip con "Ordina:" ripetuto, "Mostra solo con poche foto", "Seleziona tutto / Deseleziona", un pulsante principale e "1 selezionato". Il primo gruppo compare a metà della colonna. | `GroupList.tsx:606-720` | A |
| P4 | **Due selezioni indipendenti nella stessa schermata**, entrambe con "Seleziona tutto": i gruppi nell'elenco ("1 selezionato") e i volti nella griglia ("1 selezionati"). Un clic su un gruppo lo apre e lo seleziona anche per le azioni in blocco: aprire e selezionare sono la stessa cosa. | `GroupList.tsx:687-717`, `FaceGrid.tsx:665-670` | A |
| P5 | **Dopo aver dato un nome, il gruppo che si apre non è quello sotto.** L'elenco mostra prima i gruppi suggeriti, poi gli altri. `onCatalogChange` passa però le righe nell'ordine del backend, e sia l'avanzamento automatico sia le frecce su e giù usano quell'ordine. Verificato: dopo aver nominato il primo suggerito (26 foto) si è aperto il gruppo da 24 foto in "Altri gruppi", saltando il suggerito da 17 foto subito sotto. | `GroupList.tsx:257-293`, `PeopleModule.tsx:152-157,215` | A |
| P6 | **"Gruppo" ha tre significati**: gruppo di volti (l'elemento in revisione), la sezione "Gruppo" della scheda (che contiene l'unione) e il campo "Gruppo" dell'identità (Famiglia, Amici). Anche "identità" e "persona" si mescolano: nella sezione "Persone" si trovano "Nuova identità", "Crea l'identità «Gioia»", "Elimina identità…" e "Unisci in…". | `GroupList.tsx:614-638`, `IdentityEditor.tsx`, `PersonCard.tsx:241` | A |
| P7 | **La riga di una persona contraddice la scheda**: l'elenco dice "Gioia · non collegata · nessuno · 26 foto", la scheda "collegata a un gruppo di volti · 26 foto". Causa: quando manca il campo Gruppo (la cerchia), la riga mostra il testo "non collegata"; "nessuno" è il numero di volti di riferimento, senza etichetta. | `GroupList.tsx:776-780`, `i18n/modules.ts:101-102` | A |
| P8 | Suggerimento d'unione ambiguo: "Somiglia a «Persona senza nome»" senza la miniatura dell'altro gruppo e con "Somiglianza 0,36". La tendina "Unisci con un altro gruppo o persona…" (testo troncato) elenca sei voci identiche "Persona senza nome · N foto". Si decide senza vedere i volti. | `FaceGrid.tsx:722-765`, `PersonCard.tsx:241-260` | M |
| P9 | Punteggi grezzi senza etichetta sotto ogni volto ("2025 · 0,77"). Nell'editor della persona compare "Somiglianza minima 0.36", con il punto decimale, mentre altrove si usa la virgola e le Impostazioni parlano di "sensibilità" (requisito "Impostazioni" di desktop-shell). | `FaceGrid.tsx` (riquadro del volto), `IdentityEditor.tsx:247-256` | M |
| P10 | Scheda della persona: modulo con "Salva" esplicito a metà (altrove il salvataggio è immediato); "Apri in Persone" mentre si è già in Persone; barra di scorrimento orizzontale; "Elimina identità…" attaccato a "Unisci con…". | `IdentityEditor.tsx:244-430` | M |
| P11 | Stati vuoti doppi: griglia e scheda ripetono "Scegli un gruppo o una persona nell'elenco.". Aprendo "Volti scartati per qualità" la scheda continua a dire "Scegli un gruppo…", anche se una voce è aperta. | `FaceGrid.tsx:537-543`, `PersonCard.tsx:197-201` | M |
| P12 | Nomi troncati: tutti i gruppi diventano "Persona senza …", perché il conteggio "26 foto" in monospazio occupa la colonna. La riga del suggerimento "probabilmente la st…" non si legge. | `GroupList.tsx:869-882`, `people.css` | M |
| P13 | Intestazione del gruppo ridondante: "Persona senza nome · 26 foto", poi un chip "26 foto", "qualità buona: 26" (blu) e "scartati per qualità: 0" (giallo anche a zero). | `FaceGrid.tsx:549-565` | M |
| P14 | La sezione "Copertina" spiega come trovare un comando nascosto ("Seleziona un volto › «Usa come copertina»"), che manca dal menu contestuale del volto. | `PersonCard.tsx:41-60,226-236` | M |
| P15 | Chip di ordinamento e filtri per cerchia sono pulsanti fatti a mano con stili inline, invece del controllo segmentato condiviso. | `GroupList.tsx:618-675` | B |
| P16 | Dopo aver dato un nome compare un avviso generico, "Salvataggio completato", senza il nome scelto e senza "Annulla operazione". | `i18n/feedback.ts:12` | B |
| P17 | "1 selezionati": manca la forma singolare. | `i18n/features.ts:129` | B |

**Cosa funziona**: la scheda "Chi è?" propone le persone mentre si scrive; il menu contestuale del volto è completo; le scorciatoie (N, M, I e le frecce) sono rapide; l'elenco virtualizzato è fluido.

**Riferimenti**:
- Google Foto e Apple Foto chiedono una decisione per volta ("È la stessa persona?", "È Anna?") con le due facce affiancate.
- In Immich, scrivere il nome di una persona esistente propone l'unione (già citato nell'audit di `ui-ux-overhaul`).

---

## 4. Shell, navigazione e informazioni sull'app

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| S1 | **Richiesta degli utenti: versione e informazioni esplicite.** La versione si legge solo in Aiuto (icona "?") › ultima voce › "Informazioni su photo-cat", che mostra versione e percorso dei dati. Mancano build (commit, data, configurazione), sistema e acceleratore, autori, repository, licenza del progetto, **licenze di terze parti** (i file sono già installati in `licenses/` ma l'app non li mostra) e un comando per copiare tutto in una segnalazione. Impostazioni non ha una sezione "Informazioni". | `components/shell/TopBar.tsx:103-190`, `app/src-tauri/src/commands.rs:17-30` | A |
| S2 | **Nessuna barra dei menu.** Per un'app desktop di catalogazione (Lightroom Classic, digiKam, Capture One, Photo Mechanic) i menu File / Modifica / Vista / Aiuto sono la mappa dei comandi e mostrano le scorciatoie accanto. Oggi i comandi globali sono sparsi tra palette, menu "?", menu contestuali e pulsanti di modulo. | `TopBar.tsx`, `commands/registry.ts` | A |
| S3 | La navigazione ha un gruppo "AI" con una sola voce, anch'essa "AI": l'etichetta ripete la voce. "Varianti RAW (Fujifilm X-T5)" va su due righe. Le etichette dei gruppi sono in maiuscolo. | `modules/registry.ts:22-55`, `ModuleNav.tsx` | M |
| S4 | **In Esporta scorre l'intera finestra**: la barra di scorrimento è quella del documento e la barra superiore si sposta di 10 px. Nelle altre pagine scorre solo il contenuto. | `modules/organize/OrganizePage.tsx`, `components/shell/shell.css` | M |
| S5 | La barra di stato si sposta e si deforma: in Attività, durante il caricamento, compare a metà schermo subito sotto lo skeleton; in Sequenza va su due righe ("Nessun errore" a capo), perché ospita anche le scorciatoie del modulo e il set attivo. | `ReadyApp.tsx`, `SequencePage.tsx` | M |
| S6 | Titoli di pagina in tre posizioni: (224, 67) quasi ovunque, (248, 97) in Cataloghi, (256, 91) in Attività. Varianti RAW ha i comandi a destra del titolo, Cataloghi il pulsante "Nuovo catalogo" isolato in alto a destra, Libreria nessun titolo. | `ModulePage`, `CatalogsPage.tsx`, `JobsPage.tsx` | M |
| S7 | Tre comandi diversi per chiudere i pannelli in Libreria: "<" in cima ai Cataloghi, ">" nell'Inspector e il pulsante pannello nell'intestazione. A questi si aggiunge "<" in fondo alla navigazione. | `Sidebar.tsx`, `Inspector.tsx`, `LibraryHeader.tsx` | B |
| S8 | Accanto alla pillola delle attività compare un secondo cerchio che gira (richieste in corso): due spinner vicini con significati diversi. | `TopBar.tsx` (`PendingIndicator`) | B |
| S9 | I suggerimenti al primo uso coprono comandi: in Persone la barra dei volti, in Sequenza la casella "Raggruppa…", in Cataloghi la prima scheda (Cestino). | `components/guide/Coachmark.tsx` | M |

---

## 5. Libreria

| # | Osservazione | Dove | Pr. |
|---|---|---|---|
| L1 | "Primi passi" occupa 140 px sopra la griglia, con la chiusura "×" in basso a destra (di solito è in alto). **"Crea un catalogo" risulta fatto senza aver creato cataloghi**: la condizione conta anche i tre cataloghi di sistema (Cestino, Duplicati, Qualità insufficiente). | `modules/library/FirstSteps.tsx:46-56` | M |
| L2 | La riga "+ Nuovo catalogo" ha un conteggio "0", come se fosse un catalogo. | `modules/library/Sidebar.tsx:161-167` | M |
| L3 | Le miniature hanno due badge non spiegati, un numero in alto a sinistra (i volti) e "AI" in alto a destra, e sotto ogni foto il nome del file in monospazio maiuscolo. | `modules/library/Grid.tsx` | M |
| L4 | Inspector: "Catalogazioni AI" mostra punteggi grezzi (0,34 · 0,89 · 0,31) con barre. "Persone presenti: 2" ma un solo chip, "volto scartato · Apri in Persone". Le stelle sono caselle da 24 px poco riconoscibili. | `Inspector.tsx`, `InspectorQuality.tsx` | M |
| L5 | Ogni sezione di filtro ha l'icona "apri in finestra" (23×32 px, sotto i 24 px richiesti) senza testo; la riga del Luogo ha una matita. | `Sidebar.tsx` | B |

---

## 6. Altri moduli

### Primo avvio
- **M**: tre titoli impilati di dimensione quasi uguale ("Benvenuto in photo-cat", i passi, "Benvenuto").
- **M**: "Lo faccio dopo" compare anche all'ultimo passo, dove non c'è nulla da rimandare.
- **M**: il passo 3 dice insieme "I modelli si scaricano in background" e "I modelli AI sono già pronti".
- **B**: i controlli segmentati di lingua e tema sono larghi quanto la finestra, con le opzioni ammassate a sinistra.
- **B**: il campo della cartella ha il segnaposto in monospazio.

### Importa
- **B**: "Errori 0" in rosso.
- **B**: al termine compare l'avviso "Operazione completata", che non dice quale operazione.
- **B**: conteggi e percorsi in monospazio.

### Attività
- **A**: fasi con il nome interno "geo", "clip", "persons", "faces". Mancano in `jobs.phases`, quindi si vede la chiave grezza, contro il requisito "Nessun identificatore interno" (`i18n/it.ts:146`, `modules/jobs/JobsPage.tsx:57`).
- **M**: barre indeterminate mentre il conteggio dice "59 / 98" e la percentuale "—".
- **M**: "Analisi AI · Completata · 0 / 0 · 100%" per un'attività che non aveva nulla da fare.
- **M**: intestazioni di tabella in maiuscolo; il riquadro Risorse è duplicato in AI › Panoramica.

### Sequenza
- **M**: griglia a 4 colonne fisse con circa 180 px vuoti a destra.
- **M**: nome della fotocamera troncato in ogni miniatura ("Fujifilm …").
- **M**: etichetta "JPGS · 132 foto".
- **M**: "Set attivo: Base" ripetuto nella barra del modulo e nella barra di stato.
- **M**: "Esporta sequenza…" sta nel pannello "Dispositivi e orari".

### Cataloghi
- **M**: titolo più in basso e più a destra che nelle altre pagine.
- **M**: le schede dei cataloghi di sistema mostrano "0 foto" e due comandi ciascuna.

### Tag
- **M**: 4 tag su 6 hanno lo stesso viola (Preferite, Da stampare, Da scartare, Lavoro), quindi il colore non li distingue.
- **M**: "Set attivo" è un pulsante principale disattivato quando il set è già attivo, e sembra un errore.
- **M**: la terza colonna resta vuota ("Scegli un tag…") e le intestazioni sono in maiuscolo.

### AI
- **M**: la Mappa delle foto, che è un modo di sfogliare la libreria, è nascosta in AI › Mappa.
- **M**: le barre di copertura cliccabili sono alte 21 px.

### Esporta
- **M**: una pagina lunga con quattro riquadri e un solo "Ambito" in cima, che vale per tutti.
- **M**: 21 campi da esportare, tutti spuntati.
- Scorre l'intera finestra (vedi S4).

### Varianti RAW
- **B**: il nome della voce contiene il modello della fotocamera.
- **B**: i comandi dell'intestazione stanno in alto a destra, diversamente dalle altre pagine.

### Impostazioni
- **M**: "Cache delle anteprime di Varianti RAW" sta in Generale, che secondo il requisito "Impostazioni" contiene solo tema, lingua, doppio clic e percorso dei dati.
- **M**: manca una sezione "Informazioni" (vedi S1).

---

## 7. Estetica e coerenza visiva

| # | Osservazione | Pr. |
|---|---|---|
| E1 | **Scala tipografica non rispettata**: 8 corpi (11, 12, 13, 14, 16, 17, 18, 20) e 7 stili di intestazione. L'11 px è usato molto (54 testi in Sequenza, 32 in Libreria) ed è troppo piccolo per leggere comodamente su un monitor a 1,5×. | A |
| E2 | **Monospazio fuori luogo**: si usa per contare ("6 gruppi", "26 foto"), per date, nomi di file, percorsi, segnaposto e RAM. Il monospazio serve per codice e per allineare colonne di cifre: qui aggiunge rumore e cambia la voce dell'interfaccia. | M |
| E3 | Etichette in maiuscolo spaziato (SFOGLIA, ORGANIZZA, SET DI TAG, FASE, MODELLI RESIDENTI) accanto a etichette in minuscolo con lo stesso ruolo. | M |
| E4 | Chip colorati per i conteggi (blu "26 foto", giallo "scartati: 0"): il colore segnala urgenza anche quando non c'è nulla da fare. | M |
| E5 | Stati selezionati scritti con stili inline (bordi e sfondi) in `GroupList` e `FaceGrid` invece che con le classi dei componenti condivisi: lo stesso stato ha tre aspetti (elenco, chip, pillole). | B |
| E6 | Avvisi generici: "Operazione completata", "Salvataggio completato". Non dicono che cosa è successo né offrono "Annulla operazione" quando si potrebbe. | M |
| E7 | Pannelli con tre sfondi diversi nello stesso modulo (intestazione grigia, elenco grigio chiaro, griglia bianca, scheda grigia). Il confine tra intestazione della pagina e contenuto cambia da modulo a modulo. | B |

---

## 8. Difetti funzionali (da correggere comunque)

| # | Difetto | Causa | Rif. |
|---|---|---|---|
| F1 | Dopo aver dato un nome, e con le frecce, si apre un gruppo diverso da quello visibile sotto | ordine del backend usato al posto di quello mostrato | P5 |
| F2 | "non collegata · nessuno" per una persona collegata | campo cerchia vuoto mostrato come "non collegata"; conteggio senza etichetta | P7 |
| F3 | "Crea un catalogo" spuntato senza cataloghi dell'utente | conta i cataloghi di sistema | L1 |
| F4 | Fasi `geo`, `clip`, `persons`, `faces` mostrate col nome interno | chiavi mancanti in `jobs.phases` | §6 Attività |
| F5 | Esporta fa scorrere la finestra intera | contenitore di scorrimento sbagliato | S4 |
| F6 | Barra di stato a metà schermo durante il caricamento di Attività; su due righe in Sequenza | altezza della pagina e contenuto della barra | S5 |
| F7 | "1 selezionati" | manca la forma singolare | P17 |
| F8 | "Somiglianza minima 0.36" con il punto decimale | `toFixed` invece del formato della lingua | P9 |
| F9 | Riga "+ Nuovo catalogo" con conteggio 0 | `count={0}` sulla riga d'azione | L2 |

---

## 9. Principi per la correzione

1. **Una cosa, un nome.** Glossario esteso a Persone: *persona* (con nome, vale in tutte le librerie), *persona senza nome* (gruppo di volti da rivedere), *cerchia* (Famiglia, Amici). I verbi sono *Sposta in…* per i volti e *Unisci a…* per gruppi e persone. "Identità" e "gruppo" spariscono dai testi.
2. **I comandi compaiono quando servono.** Senza selezione nessun pulsante d'azione disattivato in fila. Con una selezione, una barra delle azioni prende il posto dell'intestazione, come già succede in Libreria. I comandi di vista (ordina, mostra) stanno sempre a destra, separati.
3. **Una selezione per schermata.** In Persone la selezione multipla dell'elenco passa da caselle visibili al passaggio del mouse o da Ctrl/Maiusc, con una barra in fondo all'elenco. Un clic semplice apre e basta.
4. **Una decisione alla volta.** Il suggerimento diventa la prima cosa della scheda ("È Gioia?", con i due volti affiancati e Sì / No). Niente punteggi: "somiglianza alta / media".
5. **Barre che non vanno a capo.** Una barra mostra le azioni principali; quelle che non entrano vanno nel menu "Altro". Nessun testo esce dai pulsanti, anche a 1024 px.
6. **Convenzioni desktop.** Barra dei menu letta dal registro dei comandi; finestra "Informazioni" completa e sezione Impostazioni › Informazioni.
7. **Una scala, un'intestazione.** Quattro corpi di testo (12, 14, 16, 20), un solo stile per le intestazioni di sezione, titolo di pagina sempre nello stesso punto, monospazio solo per percorsi e codice, niente maiuscolo nelle etichette.
8. **Stabilità della navigazione.** Nessuno spostamento di voci né cambio delle scorciatoie Ctrl+1…9. Si toglie solo il gruppo "AI" con una voce sola e si accorcia il nome "Varianti RAW".
