# Proposal

## Why

Nelle librerie reali la stessa persona finisce divisa in **molti gruppi piccoli** di 1, 2 o 3 foto. Unirli a mano è lungo, e ogni unione fa sparire il gruppo dall'elenco senza lasciare traccia. Le cause sono nel raggruppamento di oggi:
- un unico passaggio di Chinese Whispers su un grafo kNN, con una sola soglia globale (0,363, quella di verifica di SFace);
- suggerimenti d'unione calcolati confrontando i centroidi.

La ricerca (`research.md`, dalla letteratura 2017–2026 e dai prodotti) dice che con una soglia unica la frammentazione è il risultato atteso, non un errore di taratura. Su MS1M Chinese Whispers ottiene la F a coppie più bassa tra i metodi a soglia (53,9). Apple Foto risolve con due passaggi: il primo preciso, a gruppi piccoli; il secondo unisce i gruppi confrontando i loro membri. Immich non mostra come persone i gruppi con meno di 3 volti. La letteratura sulla revisione umana indica come scegliere le domande e propagare le risposte, per ottenere il massimo di unioni a ogni decisione.

## What Changes

- **Raggruppamento in due passaggi**:
  - il primo resta Chinese Whispers su kNN, ma deterministico e avviato dalle etichette precedenti, per gruppi stabili;
  - il secondo unisce i gruppi vicini con un collegamento robusto: la mediana delle somiglianze tra campioni dei membri, non i centroidi. Lo frena un controllo di vicinanza reciproca, così un singolo volto "ponte" non basta a unire due persone.
- **Vincoli rigidi**:
  - due volti della stessa foto non sono mai la stessa persona (nuovo);
  - "Non è questa persona" e "Persone diverse" restano vincoli;
  - due persone con nome non vengono mai unite automaticamente;
  - i volti confermati dall'utente non si spostano.
- **Abbinamento dei gruppi piccoli alle persone note**: un gruppo di al massimo 3 volti passa da solo a una persona con nome solo se il punteggio è alto **e** stacca chiaramente la seconda candidata. Altrimenti diventa una domanda.
- **Soglie tarate sulla libreria**:
  - due soglie, "unisci da solo" (molto severa) e "chiedi" (più larga);
  - partono da valori predefiniti e si adattano alla libreria usando le coppie di volti della stessa foto (sicuramente persone diverse) e le persone confermate dall'utente;
  - la sensibilità è regolabile in Impostazioni.
- **Coda "Da confermare"** in Persone › Da rivedere:
  - domande ordinate per volti risolti a ogni risposta;
  - risposte in blocco, ad esempio "12 gruppi probabilmente di Gioia" con tutti i volti visibili, Conferma tutti o togli quelli sbagliati;
  - ogni Sì si propaga per transitività e ogni No diventa un vincolo.
- **Storico delle unioni**: un gruppo unito, a mano o da solo, non sparisce senza traccia. "Unioni recenti" mostra che cosa è finito dove, con "Annulla" per ciascuna unione.
- **Misure**: in Diagnostica il numero medio di gruppi per persona con nome, la quota di gruppi da un volto, le unioni automatiche e quante sono state annullate. Un banco di prova riproducibile con dati etichettati serve a tarare e a evitare regressioni.

Nessun file viene toccato. Le persone con nome e le correzioni dell'utente restano invariate.

## Capabilities

### New Capabilities

Nessuna: il comportamento appartiene a `face-recognition`.

### Modified Capabilities

- `face-recognition`:
  - modificato "Raggruppamento in persone": due passaggi, collegamento robusto, vincoli, soglie tarate;
  - nuovi requisiti:
    - "Volti della stessa foto";
    - "Abbinamento dei gruppi piccoli";
    - "Soglie tarate sulla libreria";
    - "Coda Da confermare";
    - "Propagazione delle risposte";
    - "Gruppi stabili tra le analisi";
    - "Storico delle unioni";
    - "Misure del raggruppamento".

## Impact

- **Rust**:
  - `crates/pc-ai/src/cluster.rs`: primo passaggio deterministico, secondo passaggio, abbinamento dei gruppi piccoli;
  - `crates/pc-store/src/people.rs`: vincoli della stessa foto, suggerimenti con collegamento robusto, storico delle unioni con annullamento, stato "confermato" dei volti;
  - nuovo modulo di taratura delle soglie;
  - nuova migrazione per lo storico e lo stato dei volti;
  - metriche in `pc-diag`.
- **UI**:
  - Persone: coda "Da confermare" con risposte in blocco e "Unioni recenti";
  - Impostazioni › Riconoscimento AI: sensibilità dell'unione automatica;
  - Diagnostica: misure.
- **Prestazioni**: il secondo passaggio confronta solo gruppi già vicini nel grafo, con al massimo 32×32 campioni per coppia. Il costo resta nell'ordine del primo passaggio fino a circa 100.000 volti.
- **Dipendenze**: nessuna nuova. Il cambio del modello degli embedding (ad esempio AuraFace, Apache-2.0) e i metodi appresi (GCN, transformer) sono fuori da questa change: vedi `design.md`.
- **Prerequisito**: `ui-usability-audit`, per la nuova struttura di Persone.
