# Design

## Context

Motivazione in `proposal.md`, fonti e numeri in `research.md`.

Il raggruppamento di oggi (`crates/pc-ai/src/cluster.rs`):
- **Grafo**: per ogni modello di embedding (SFace, 128-d), un indice HNSW (16 vicini, `ef` 100). Ogni volto tiene al massimo 16 archi verso vicini con coseno ≥ `face_similarity` (predefinito 0,363), esclusi gli archi tra persone segnate "Persone diverse".
- **Chinese Whispers**: 30 iterazioni con voti pesati dal coseno. Le etichette di persone con nome, volti manuali e rifiuti sono fisse; i volti rifiutati non votano per la persona che li ha rifiutati.
- **Prima del raggruppamento**: i volti non manuali vengono abbinati alle identità con volti di riferimento (`named_match_refs`).
- **Dopo**: le etichette nuove riprendono l'id di persona più frequente tra i membri; poi si ricalcolano i centroidi (`refresh_centroids_with`) e i suggerimenti d'unione (`refresh_merge_suggestions`). I suggerimenti confrontano i centroidi con soglia `similarity × 0,8`.
- **Cosa manca**: nessun vincolo per i volti della stessa foto; l'annullamento delle correzioni esiste (`undoPeople`), ma non uno storico consultabile.

Vincoli:
- solo CPU;
- librerie da 1.000 a circa 100.000 volti;
- budget di memoria del governatore;
- analisi incrementali a ogni importazione;
- nessun dato biometrico fuori dal computer.

## Goals / Non-Goals

**Goals:**
- Ridurre la frammentazione: meno gruppi per persona e meno gruppi da un volto, senza aumentare le unioni sbagliate.
- Rendere la revisione dei gruppi piccoli una questione di poche decisioni, ciascuna delle quali risolve molti volti.
- Rendere visibile e annullabile ogni unione.
- Misurare: banco di prova riproducibile e metriche in Diagnostica.

**Non-Goals:**
- **Cambiare modello di embedding.** AuraFace (Apache-2.0, ResNet100) è un candidato, ma richiede di ricalcolare tutti gli embedding e di ritarare le soglie, e pesa di più: change a parte, da valutare con il banco di prova di questa.
- **Metodi appresi** (GCN, transformer). Richiedono addestramento con identità etichettate e GPU, e guadagnano circa un punto su FaceMap.
- **Sostituire il primo passaggio con FaceMap/Infomap.** È il miglior metodo non supervisionato in letteratura (F a coppie 94,2 e BCubed 92,6 su MS1M), ma lo si valuta solo se, dopo questa change, le misure mostrano ancora troppa frammentazione.
- **Abiti e corporatura** (come Apple, nello stesso evento): rinviati.

## Decisions

### D1. Primo passaggio: preciso, deterministico, stabile
Chinese Whispers resta, con tre modifiche:
- **ordine di visita fisso** per id del volto e parità risolte per id dell'etichetta, così lo stesso input dà sempre lo stesso risultato (la letteratura segnala che CW è casuale e dipende dall'ordine);
- **partenza dalle etichette precedenti** dei volti già assegnati, al posto di un'etichetta propria per ciascuno: un'analisi incrementale cambia solo ciò che i nuovi volti toccano;
- **vincoli della stessa foto**: niente archi tra volti della stessa foto. Mentre sceglie l'etichetta, un volto non può prendere un'etichetta che contiene già un altro volto della sua foto: una mappa etichetta → foto, aggiornata a ogni cambio.

La soglia del grafo resta la soglia "chiedi" (D4): il primo passaggio deve produrre gruppi piccoli ma puri, come il primo passaggio di Apple ("alta precisione, molti gruppi piccoli").

### D2. Secondo passaggio: unione dei gruppi con collegamento robusto
- **Coppie candidate**: le coppie di gruppi collegate da almeno un arco del grafo kNN; per ogni gruppo, al massimo gli 8 gruppi con più archi in comune.
- **Collegamento**: la mediana dei coseni tra campioni dei due gruppi. Al massimo 12 volti per gruppo, scelti per qualità (i più nitidi e grandi prima), quindi al massimo 144 coppie. La mediana resiste ai volti ponte meglio del massimo (collegamento singolo) e costa molto meno della media su tutte le coppie: è la scelta di Apple (mediana su campioni). Il centroide di oggi tende a dare punteggi più alti delle coppie, e con la stessa soglia unirebbe troppo.
- **Reciprocità**: si uniscono A e B solo se B è tra i 3 gruppi più vicini di A e viceversa (vicini reciproci, come nel re-ranking k-reciproco), il collegamento supera la soglia "unisci da solo" e nessun vincolo li separa. Vincoli: volti della stessa foto, "Persone diverse", rifiuti, due persone con nome.
- **Ordine di unione**: agglomerativo, dalla coppia migliore. Dopo ogni unione si ricalcolano solo le coppie del gruppo nuovo. Ci si ferma quando nessuna coppia supera la soglia.
- **Domande**: le coppie tra le soglie "chiedi" e "unisci da solo" diventano domande (D6).
- **Date lontane**: se le foto più vicine nel tempo dei due gruppi distano più di 3 anni, l'unione automatica richiede un margine in più (+0,05). La verifica peggiora molto con il passare degli anni, soprattutto nei bambini (`research.md`). Sotto quel margine la coppia diventa una domanda.

### D3. Gruppi piccoli verso persone con nome
Per un gruppo senza nome di al massimo 3 volti si calcola il collegamento (D2) verso le persone con nome raggiungibili nel grafo.
- **Unione automatica** se il migliore supera "unisci da solo" **e** stacca il secondo di almeno 0,05: è la regola "rispetto al secondo", comune nel riconoscimento a insieme aperto e consigliata dalla ricerca.
- **Domanda** se supera solo "chiedi", oppure se due candidati sono vicini ("È Anna o Marta?").
- **Qualità**: un gruppo i cui volti sono tutti di qualità bassa non viene mai unito da solo.

### D4. Soglie tarate senza etichette
**Coppie "sicuramente diverse"**: tutte le coppie di volti validi della stessa foto. È un'idea emersa dalla ricerca come deduzione, senza un articolo che la applichi: la si verifica con il banco di prova (D9).
- con almeno 200 coppie, "unisci da solo" è il quantile 1 − 10⁻³ dei loro coseni e "chiedi" il quantile 1 − 10⁻²;
- "unisci da solo" non scende mai sotto il valore predefinito del modello e "chiedi" resta sotto "unisci da solo";
- **valori predefiniti di partenza per SFace**: "unisci da solo" a coseno 0,55 e "chiedi" a 0,363. Il primo viene da PhotoPrism, che usa lo stesso modello e ha tarato le soglie su circa 88.000 volti di una libreria reale: distanza euclidea d'accettazione 0,95 su embedding normalizzati, pari a un coseno di circa 0,55. Il secondo è la soglia di verifica di oggi. Margine verso la seconda candidata: 0,05 in partenza, contro 0,01 di PhotoPrism. I tre valori si confermano con il banco di prova prima dell'integrazione.

**Coppie "sicuramente uguali"**: quando esistono volti confermati della stessa persona, servono solo da controllo. Se più del 20% supera appena "chiedi", Diagnostica segnala che le soglie sono probabilmente troppo severe; le soglie non si abbassano da sole.

**Sensibilità**: la scala in Impostazioni ("Più domande ↔ Più unioni automatiche") sposta il tasso obiettivo di unioni sbagliate tra 10⁻⁴ e 10⁻². Si applica con "Applica" e anteprima, come le altre soglie che ricalcolano dati.

### D5. Stato dei volti e stabilità
- **Volti confermati**: il flag `manual` esistente diventa "confermato" e lo ricevono anche i volti passati da una risposta della coda. Un'analisi non li sposta mai.
- **Identità dei gruppi**: dopo ogni analisi, ogni gruppo nuovo riprende l'id della persona da cui proviene almeno metà dei suoi volti (oggi basta la maggioranza relativa), con nome e copertina. Gli altri ricevono un id nuovo. Così l'elenco non si rimescola a ogni importazione.
- **Analisi incrementale**: i nuovi volti si abbinano prima ai gruppi esistenti (D3 per i gruppi piccoli, D2 per i gruppi nuovi); un ricalcolo completo avviene solo con un cambio di soglie o su richiesta (Rianalisi).

### D6. Coda "Da confermare"
Le domande si salvano in una tabella `review_question` (tipo: gruppo↔persona, gruppo↔gruppo, gruppo↔{persona A, persona B}; collegamento; volti coinvolti), ricalcolata a ogni analisi.

**Ordine** (dalla letteratura sulla risoluzione di entità con risposte umane):
1. prima le domande che coinvolgono i gruppi senza nome più grandi, ordinate per volti risolti attesi (probabilità × volti): l'ordine "per nodi" che non degrada con cluster di dimensioni sbilanciate;
2. poi i gruppi piccoli per probabilità decrescente, l'ordine migliore quando i cluster hanno 3 o meno elementi.

**Domande in blocco**: le domande gruppo↔persona con la stessa persona si raggruppano in una scheda ("12 gruppi probabilmente di Gioia"), con tutti i volti, caselle selezionate e "Conferma tutti". Si chiede per gruppo, non per volto, perché i gruppi sono già insiemi coerenti.

**Propagazione**:
- i Sì passano per una *union-find*;
- un No diventa un vincolo (`not_same` o rifiuto) e toglie le domande tra componenti separate da quel No;
- un Sì tra componenti già separate da un No è un conflitto, che si mostra (spec "Propagazione delle risposte").

**Interfaccia**: la coda è una voce in testa all'elenco di "Da rivedere" e si apre nei riquadri di Persone riorganizzati da `ui-usability-audit`, con la stessa domanda "È {nome}?".

### D7. Storico delle unioni
Una nuova tabella `merge_log`:
- `id`, `at`, `origin` (manuale, automatica, coda), `target_person`, `faces`;
- `snapshot`: JSON con gli id dei volti e la persona di provenienza, nome e copertina del gruppo d'origine.

"Annulla":
- riporta i volti alla persona di provenienza, ricreandola se serve con lo stesso id;
- per le unioni automatiche aggiunge un vincolo `not_same`, così l'unione non si ripete.

Il registro di annullamento di oggi (`undoPeople`) diventa una vista sulle ultime voci di `merge_log` e degli altri tipi di correzione. In Persone, "Unioni recenti" elenca le voci degli ultimi 90 giorni, con filtro per persona.

### D8. Misure in Diagnostica
Dopo ogni analisi si salvano:
- **gruppi per persona con nome**: 1 più i gruppi senza nome con una domanda aperta verso quella persona, in media;
- **quota di gruppi senza nome con un volto**;
- **unioni automatiche e annullate**, da `merge_log`;
- **soglie in uso e loro origine**.

L'evento si registra anche nel log di diagnostica, così si confrontano versioni diverse.

### D9. Banco di prova
Un esempio Rust (`crates/pc-ai/examples/cluster_bench.rs`) legge un file JSONL di embedding con identità e calcola:
- F a coppie, BCubed F (la metrica ufficiale di IJB-B, che pesa ogni volto allo stesso modo);
- gruppi per identità e quota di gruppi da un volto;
- precisione delle unioni automatiche e numero di domande per arrivare a una BCubed F di 0,95 simulando un utente perfetto.

Le fonti dei dati:
- **dati sintetici** per la CI: identità come nuvole sull'ipersfera, con frammenti più lontani, gemelli e volti ponte; i test di regressione verificano che il nuovo raggruppamento migliori la BCubed F del vecchio e non peggiori la precisione;
- **libreria dell'utente**: un comando di sviluppo esporta gli embedding delle persone con nome in un file locale, che resta sul computer e non entra mai nel repository. Serve a fissare i valori predefiniti di D4 prima dell'integrazione.

## Risks / Trade-offs

- [Unioni sbagliate: un errore d'unione costa più di un gruppo in più] → soglia "unisci da solo" al quantile 10⁻³, reciprocità, vincoli della stessa foto, nessuna unione automatica tra persone con nome, storico con "Annulla" che diventa un vincolo, quota di annullamenti misurata in Diagnostica.
- [Le coppie della stessa foto non rappresentano tutti gli sconosciuti (parenti, età simile)] → spostano la soglia verso l'alto, cioè la rendono più prudente, non più permissiva; il valore predefinito resta comunque il minimo.
- [Prima analisi dopo l'aggiornamento: molte unioni insieme] → la prima riaggregazione mostra un riepilogo ("52 gruppi uniti automaticamente") con il collegamento allo storico, dove si può annullare voce per voce; le persone con nome non vengono mai unite tra loro.
- [Costo del secondo passaggio con 100.000 volti] → al massimo 8 gruppi candidati per gruppo e 144 coppie campione per coppia di gruppi: circa 30.000 gruppi × 8 × 144 prodotti scalari a 128 dimensioni, pochi secondi su CPU. La stima di memoria del governatore aggiunge la tabella delle coppie; il passaggio è interrompibile come il primo.
- [L'unica taratura pubblicata di SFace per il raggruppamento è quella di PhotoPrism: 0,363 viene dalla verifica e la sua origine non è documentata] → si parte dai valori di PhotoPrism e si confermano con il banco di prova sulla libreria dell'utente prima dell'integrazione; nel codice si scrivono con il valore misurato e la fonte.
- [Coda troppo lunga in librerie grandi] → si mostrano le domande dei 50 elementi più grandi; il resto arriva man mano che si risponde.

## Migration Plan

1. Migrazione del database: tabelle `merge_log` e `review_question`, il flag "confermato" dove serve, indice per foto sui volti. I dati esistenti non cambiano.
2. Alla prima analisi dopo l'aggiornamento si esegue una riaggregazione completa in background, con il riepilogo descritto nei rischi.
3. Rollback: la versione precedente ignora le tabelle nuove. Le unioni automatiche già fatte si annullano dallo storico prima del downgrade, se necessario.

## Open Questions

- Valori predefiniti delle due soglie e del margine per SFace: si fissano con il banco di prova (task 2.x), senza cambiare l'approccio.
- Con quanti anni di distanza scatta il margine aggiuntivo: 3 è il valore di partenza, da verificare sulla libreria di prova. `research.md` suggerisce circa 2 anni per i bambini.
- Quanti campioni per coppia di gruppi: 12×12 per stare nel budget, contro i 32×32 indicati da `research.md`. Si sceglie con il banco di prova, misurando qualità e tempo.
