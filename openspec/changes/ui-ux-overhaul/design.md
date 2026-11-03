# Design

## Context

L'analisi completa è in `audit.md`, le schermate di riferimento in `mockups/` e sul canvas di design https://claude.ai/artifact/FoQtdU21aWGGp1dZJK3aHz.

**Situazione di partenza (`main` f40beb5)**:
- React 18, zustand, TanStack Query e Virtual, i18next; nessuna libreria di componenti;
- colori come token in `theme/tokens.ts`, applicati come `--pc-*`; spaziature, raggi e tipografia sono valori fissi;
- 23 icone fatte a mano in `components/icons.tsx`, più glifi di testo;
- un solo componente di avviso, `components/feedback/Toast.tsx`, usato da `catalogActions`.

**Change che arrivano prima di questa**:
- `loading-feedback`:
  - porta `components/feedback/*`: Spinner, Skeleton, QueryView, RefreshBar, BusyButton, Toast/useToast;
  - porta l'indicatore delle richieste in corso e la schermata di avvio.
- `people-faces-workspace`:
  - unisce Persone e Identità in `modules/people/PeopleModule.tsx`, con tastiera e virtualizzazione;
  - aggiunge `openPeople` / `peopleTarget` in `store/layout.ts`, con migrazione del modulo persistito.
- `catalog-exclusions`: già integrata, ma ancora da archiviare. I suoi requisiti di `catalog-library` e `bulk-operations` sono la base di questa change.

**Vincoli**:
- app desktop offline (WebView2 / WebKitGTK), italiano come lingua di riferimento e inglese allineato (`keys.test.ts`);
- contrasto verificato da `tokens.test.ts`;
- preferenze dell'interfaccia rispecchiate in `ui-prefs.json`;
- CI Linux obbligatoria (`make test-linux`).

## Goals / Non-Goals

**Goals**:
- architettura dell'informazione a gruppi e stabile;
- un solo sistema di componenti e regole di interazione;
- Libreria centrata sul contenuto, con barra delle azioni sulla selezione e visualizzatore;
- palette dei comandi, scorciatoie scopribili, aiuto contestuale;
- Impostazioni a sezioni e senza duplicati;
- linguaggio comprensibile e coerente;
- WCAG 2.2 AA.

**Non-Goals**:
- nuove funzioni di dominio. Fanno eccezione il visualizzatore interno, che riusa lo schermo intero di Sequenza, e il registro "annulla" globale, che riusa le operazioni già annullabili;
- ridisegnare il flusso del modulo Persone di `people-faces-workspace`: si adatta solo ai componenti condivisi e al glossario;
- cambiare il formato dei dati, del catalogo o dei file;
- un tema "di marca" diverso dall'attuale: si mantengono palette e font, rendendo più sobrie la grafica di contorno e la gerarchia.

## Decisions

### D1. Questa change viene dopo tutte le altre
Si implementa solo dopo l'integrazione e l'archiviazione di `loading-feedback` e `people-faces-workspace`, e l'archiviazione di `catalog-exclusions`.

**Perché**: quasi ogni file UI è toccato anche da quelle change (Grid, Inspector, BulkBar, TopBar, Settings, Persone). Farla prima vorrebbe dire rifare il lavoro. Lo insegna anche Google Foto: una sola riorganizzazione, poi stabilità.

**Come**: il gruppo 0 di `tasks.md` riallinea le delta al testo archiviato. In particolare:
- "Indicatori globali" parte dal testo di `loading-feedback`;
- "Moduli di navigazione" parte dal testo di `people-faces-workspace`.

Si fa con `openspec show desktop-shell --type spec` dopo l'archiviazione. Se una di quelle change cambia il proprio piano, si aggiorna questa delta prima dell'implementazione.

### D2. Navigazione a gruppi, Modelli e Diagnostica dentro Impostazioni
Ordine e gruppi:
- **Sfoglia**: Libreria, Sequenza;
- **Organizza**: Cataloghi, Persone, Tag;
- **Importa ed esporta**: Importa, Esporta;
- in fondo: Attività, Impostazioni.

Le voci di primo livello passano da 11 a 9; le scorciatoie sono Ctrl+1…7 per le voci dei gruppi.

`ModuleId` mantiene `models` e `diagnostics` come *destinazioni* (deep link), ma non come voci: `openModule({ module: "settings", section: "models" })`. La migrazione di `store/layout.ts` (persist `version` +1) porta `models` → `settings/models` e `diagnostics` → `settings/diagnostics`, e si somma a quella di Identità.

**Alternative scartate**:
- *Nascondere i moduli non usati* (PhotoPrism): rimandato, perché con 9 voci il beneficio è modesto.
- *Attività dentro Impostazioni*: scartato. Lo stato del sistema deve restare visibile (Nielsen #1) e Fluent ammette voci di navigazione in fondo al pannello.

### D3. Componenti: primitive accessibili esistenti, stile proprio
Si adottano primitive senza stile per i comportamenti difficili:
- `@radix-ui/react-dialog`, `-alert-dialog`, `-dropdown-menu`, `-context-menu`, `-popover`, `-tooltip`, `-tabs`, `-toggle-group` (licenza MIT);
- `cmdk` per la palette (MIT).

Lo stile resta interamente nostro: token e classi in `components/ui/*`.

**Perché**:
- focus trap, ritorno del focus, sottomenu e typeahead sono la fonte dei difetti A dell'audit;
- riscriverli bene costa più che importarli;
- sono librerie mature, offline e piccole (tree-shaking per primitiva).

**Licenze**: le nuove dipendenze vanno aggiunte a `assets/licenses` con lo stesso processo usato per le altre.

**Alternative**:
- *React Aria Components*: più completo (anche per le griglie), ma più pesante e con un modello diverso.
- *Tutto fatto in casa*: possibile, ma richiede test di accessibilità propri per ogni primitiva.

Se l'utente preferisce non aggiungere dipendenze, D3 diventa "fatto in casa" con gli stessi componenti e gli stessi test (vedi Open Questions).

**Icone**: `lucide-react` (ISC), con tratto e stile coerenti con le icone attuali, al posto delle icone fatte a mano e dei glifi ▸ ▾ ↻ ✎ × ↑ −. `components/icons.tsx` diventa un adattatore, così si cambia set in un punto solo.

Il `Toast` di `loading-feedback` resta l'unico, esteso con pila, chiusura e azione (un unico `ToastHost`).

### D4. Token aggiuntivi
In `theme/tokens.ts`, oltre ai colori:

| Token | Valori |
|---|---|
| `space` | 4, 8, 12, 16, 24, 32, 48 |
| `radius` | 4, 6, 10, pill |
| `font` (dimensione / interlinea / peso) | 11 / 12 / 13 / 14 / 16 / 20 / 26 |
| `elevation` | piatta, pannello flottante, dialogo |
| `motion` | 120 ms e 200 ms, con `prefers-reduced-motion` |
| `z` | contenuto < pannelli flottanti < avvisi < menu e popover < dialoghi |

Il tema `system` segue `prefers-color-scheme` ed è il valore predefinito solo per le nuove installazioni. Le installazioni esistenti mantengono il tema salvato.

Si aggiungono a `contrastPairs` le coppie 3:1 di bordo, icona e focus. Il test di contrasto copre entrambi i temi.

### D5. Libreria: una barra, poi la barra delle azioni
- `LibraryHeader` sostituisce `CatalogHeader`, `CatalogFeedback` e `Toolbar`:
  - percorso (Tutte le foto › catalogo › Escluse);
  - conteggio (con "in calcolo…" di `loading-feedback`);
  - chip dei filtri;
  - menu "Ordina" e "Vista" (dimensione e raggruppamento dei simili);
  - interruttore dell'ispettore.
- "Solo foto con simili" passa al pannello Filtri, come faccetta booleana con il proprio chip.
- `SelectionBar` sostituisce `BulkBar` quando `selCount > 0`:
  - comandi diretti: Tag, Catalogo, Persona, Luogo, Apri con, Esporta;
  - menu "Altro";
  - ogni comando apre un popover con ricerca, riusando `CategoryPicker`, `PlaceEditor` e il selettore dei tag;
  - le azioni restano quelle di `bulk-operations`, nessuna nuova azione di dominio;
  - "Seleziona tutte le foto del filtro" usa la selezione per filtro di `large-library-stability`.
- Visualizzatore: si estrae lo schermo intero di `SequencePage` (`:784`) in `components/viewer/Viewer.tsx`, condiviso tra Libreria e Sequenza. Usa `thumb://` alla risoluzione massima già disponibile; non serve un nuovo protocollo, salvo il punto 1 delle Open Questions.
- Griglia: un modello di focus "roving" sul virtualizzatore. Il focus ha un indice proprio, distinto dalla selezione, e lo scorrimento lo segue.
- Inspector:
  - sezioni con `Disclosure`, il cui stato va nello store `layout` persistito;
  - modifica sul campo con `InlineEdit` (valore → editor → Invio/Esc);
  - con selezione multipla mostra i valori comuni o "Valori diversi" e applica con un'operazione bulk.

### D6. Annullamento globale
Uno store `undo` registra l'ultima operazione annullabile, qualunque modulo l'abbia avviata: bulk, esclusioni, tag (rinomina, unione, eliminazione), persone (unione, separazione, ignora), luoghi e categorie. Il registro contiene:
- l'etichetta in parole;
- la funzione che chiama il comando di annullamento già esistente (`undo_bulk` e simili).

Ctrl+Z e l'avviso chiamano lo stesso registro. Lo storico in Attività elenca le operazioni annullabili recenti.

Lato backend non serve nulla di nuovo se ogni operazione espone già il suo annullamento. Se se ne trova una che non lo espone, va elencata in `tasks.md` 3.6 prima di procedere.

### D7. Palette dei comandi e scorciatoie
- Un registro dei comandi (`commands/registry.ts`): id, etichetta i18n, gruppo, scorciatoia, condizione di disponibilità (ad esempio "c'è una selezione") ed esecuzione.
- Navigazione, menu, palette e foglio delle scorciatoie leggono tutti dallo stesso registro, così non possono divergere.
- Le entità (persone, tag, cataloghi, luoghi) arrivano dalle query esistenti, con limite e debounce.
- I nomi dei file si cercano con il filtro di testo esistente, mostrando i primi 20 risultati.
- Un unico `useHotkeys` globale ignora i campi di testo, salvo Ctrl+K ed Esc, e sostituisce i gestori sparsi (tasti di Sequenza, tasti dei tag).

### D8. Guida nell'app
- Lo stato dei "Primi passi" e dei suggerimenti già visti vive in un nuovo store persistito `photo-cat.guide`, quindi passa da `ui-prefs.json` e sopravvive alle reinstallazioni.
- Le voci dei "Primi passi" si completano osservando eventi già presenti:
  - prima sorgente aggiunta;
  - modelli verificati;
  - prima identità collegata;
  - primo catalogo;
  - primo tag assegnato con un tasto.
- Suggerimenti: componente `Coachmark` ancorato, non modale, al massimo uno visibile alla volta.
- Nessun tour a schede: secondo NN/g i tutorial a schede non migliorano il successo e peggiorano la percezione della difficoltà.
- Onboarding: si riscrive `Onboarding.tsx` in 3 passi con `FolderPicker`.
  - La verifica di sistema diventa un riepilogo testuale calcolato da `systemInfo`.
  - Il download dei modelli parte in background.
  - Alla fine si apre la Libreria, non Attività.
  - Si mantiene il vincolo di `loading-feedback`: l'onboarding compare dopo la schermata di avvio.

### D9. Impostazioni a sezioni e modo di salvare
- `SettingsPage` diventa un layout con sotto-navigazione: Generale, Libreria e importazione, Riconoscimento AI, Modelli AI e sistema, Applicazioni esterne, Diagnostica, Avanzate.
- La ricerca filtra sulle etichette i18n dei campi, porta alla sezione ed evidenzia il campo.
- Salvataggio:
  - le preferenze semplici si salvano subito, con debounce di 400 ms, `set_config` e una spunta "Salvato" accanto al campo;
  - soglie, tassonomia e percorso dei dati restano in stato locale "da applicare", con barra "Ripristina / Applica" e anteprima (riusando l'anteprima delle soglie già presente);
  - si rimuovono i duplicati in `ModelsPage` e `DiagnosticsPage`, il cui contenuto diventa le sezioni corrispondenti.
- Sensibilità: la UI mostra `1 − soglia` o la soglia in percentuale, a seconda del verso del parametro, con le etichette "Più risultati" e "Più precisi". Il verso di ogni soglia va definito in una tabella in `tuning.ts`, per non confondere confidenza e distanza. Il valore grezzo resta in "Avanzate".

### D10. Glossario e i18n
Il glossario dell'audit (§6) diventa `i18n/GLOSSARY.md` più un test: un elenco di termini vietati per lingua, ad esempio "job" nelle stringhe `it` e i codici operatore. Le chiavi prese in prestito da altri moduli (`people.save`, `library.facets.close`) si sostituiscono con chiavi `common.*`, e si rimuovono le chiavi inutilizzate indicate nell'audit. Le date passano per `Intl.DateTimeFormat` nella lingua attiva.

### D11. Verifica dell'accessibilità
- Test vitest per ogni primitiva: focus, Esc, ritorno del focus, ruoli.
- Un test che scorre il DOM delle pagine renderizzate e segnala:
  - bersagli interattivi sotto 24 px (con jsdom misurando lo stile calcolato su `min-width` e `min-height` delle classi);
  - controlli annidati;
  - pulsanti con sola icona senza nome.
- Prova manuale da tastiera sull'app reale, con la lista in `tasks.md`.

## Risks / Trade-offs

- **[Ampiezza: quasi tutti i file UI]**:
  - fasi integrabili una alla volta (fondamenta → shell → Libreria → palette → guida → impostazioni → moduli → pulizia), ciascuna con i test verdi e la CI Linux;
  - nessuna fase cambia dati.
- **[Abitudini degli utenti che già usano l'app]**:
  - la navigazione cambia una volta sola;
  - nella prima versione un suggerimento "Dove sono finiti Modelli AI e Diagnostica?" compare una sola volta;
  - le scorciatoie esistenti (1–9, A, Spazio in Sequenza) restano invariate.
- **[Nuove dipendenze (D3)]**: librerie piccole e senza rete. In alternativa, primitive fatte in casa con gli stessi test.
- **[Il visualizzatore interno può essere lento con i RAW]**: mostra subito la miniatura più grande disponibile, poi l'anteprima piena se esiste; per i RAW senza anteprima incorporata resta la miniatura (vedi Open Questions).
- **[Il salvataggio immediato può creare scritture troppo frequenti del config]**: debounce e scrittura atomica già presente in `settings`.
- **[Le delta di `desktop-shell` partono da testi ancora in movimento]**: gruppo 0 obbligatorio, e `openspec validate --strict` prima di implementare.

## Migration Plan

1. Gruppo 0: verificare i prerequisiti e riallineare le delta.
2. Integrare una fase alla volta su `feat/ui-ux-overhaul`, con un merge in `main` per fase o per gruppi di fasi, dopo la CI.
3. Migrazioni degli store persistiti, con `version` +1 e un test per ogni migrazione:
   - `layout`: modulo, sezione delle Impostazioni, nuove preferenze dei pannelli;
   - `guide` (nuovo).
4. Il tema `system` è il valore predefinito solo se non c'è un tema salvato.
5. Rollback: ogni fase è un merge a sé stante. Nessun dato del catalogo cambia, quindi tornare a una versione precedente non richiede migrazioni inverse, salvo ignorare le nuove chiavi di `ui-prefs.json`.

## Open Questions

1. **Visualizzatore**: per i RAW senza anteprima incorporata basta la miniatura più grande esistente, o serve una decodifica a piena risoluzione?

   *Proposta*: la miniatura più grande ora, la decodifica piena in una change successiva.
2. **Dipendenze (D3)**: va bene aggiungere Radix, `cmdk` e `lucide-react`?

   *Proposta*: sì.
3. **Nome del modulo Esporta**: tenere "Esporta" con le sezioni "Esporta dati", "Organizza in cartelle" e "Sequenza", o separare "Organizza"?

   *Proposta*: un solo modulo "Esporta", perché in entrambi i casi l'utente porta foto fuori dal catalogo. L'export della sequenza resta raggiungibile anche da Sequenza.
4. **"Smart" o "dinamico"**: tenere "Smart", come Apple Foto e Lightroom in italiano?

   *Proposta*: sì.

### Decisioni (3 novembre 2026)

Confermate dall'utente prima dell'implementazione:

1. **Visualizzatore**: per i RAW senza anteprima incorporata si usa la miniatura più grande esistente; la decodifica a piena risoluzione va in una change successiva.
2. **Dipendenze**: si aggiungono le primitive Radix, `cmdk` e `lucide-react`.
3. **Esporta**: un solo modulo "Esporta" con le sezioni "Esporta dati", "Organizza in cartelle" e "Sequenza".
4. **Termine**: resta "Smart".
