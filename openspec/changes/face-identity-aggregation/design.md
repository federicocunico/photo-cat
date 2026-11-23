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

## Esito dell'implementazione

### Banco di prova: valori di partenza e risultati (task 1.3, 3.4, 4.2)

`cargo run -p pc-ai --example cluster_bench` (soglie 0,363 / 0,55 / margine 0,05). Dati sintetici con le distribuzioni misurate sulla libreria di prova (coseni della stessa persona: mediana 0,46 contro 0,49 reale; della stessa foto: mediana 0,11 contro 0,14 reale): 1.081 volti, 120 identità; l'utente ha già dato un nome alle 12 identità più grandi, confermandone il 30% dei volti del periodo con più foto.

| Misura | Prima (CW, una soglia) | Dopo (due passaggi) |
| --- | --- | --- |
| BCubed F (P / R) | 0,844 (0,743 / 0,976) | 0,950 (0,965 / 0,934) |
| F a coppie | 0,824 | 0,933 |
| Gruppi per identità | 1,13 | 1,28 |
| Quota di gruppi da un volto | 5,0% | 17,7% |
| Unioni automatiche (precisione) | — | 0 (1,00) |
| Domande nella coda | — | 8 |

- Il guadagno viene dalla precisione: prima il centroide delle persone con nome (a 0,363) e la propagazione delle etichette con nome in Chinese Whispers attiravano volti di altre persone. Il prezzo è un richiamo più basso: più volti restano da soli in "Da rivedere".
- Con le distribuzioni realistiche il collegamento mediano tra due gruppi della stessa persona sta intorno a 0,46–0,49: a 0,55 le unioni automatiche sono rare (0 nel banco). Abbassando "unisci da solo" restano corrette (0,40: 1 unione, 0,38: 2 unioni, precisione 1,00), ma senza prove sulla libreria reale i valori predefiniti restano **0,55 / 0,363 / 0,05**. Vanno confermati sulla libreria dell'utente (task 8.3).
- Libreria di prova copiata (197 volti, una sola persona con nome, 26 volti): prima BCubed F 1,000, dopo 0,922 (2 volti restano da soli, nessuna domanda). Non è una misura utile: le etichette di quella persona vengono dal raggruppamento di prima, che quindi è avvantaggiato per costruzione. Servono persone curate a mano (task 8.3).
- Coda, utente perfetto: sul banco l'ordine misto non chiede mai più domande dell'ordine per probabilità. Con frammenti lontani nel tempo (coseno 0,68–0,8 dal centro) arriva a metà del guadagno possibile con 3–4 domande invece di 7–9; con seme 1 arriva a BCubed F 0,95 con 8 domande invece di 9. Nella maggior parte delle configurazioni realistiche, però, la coda da sola non porta la BCubed F a 0,95: il test del task 6.1 verifica 0,95 su un caso dove è raggiungibile e "metà del guadagno" su tre configurazioni.

### Scostamenti dal design

1. **Le persone protette non attirano volti nel primo passaggio.** Le etichette fisse (con nome, ignorate, con volti confermati, con vincoli) non votano in Chinese Whispers. Con la propagazione, un volto simile ad "Anna" e a "Marta" finiva nella più votata, contro lo scenario "Due candidate vicine". I volti nuovi raggiungono le persone protette dall'abbinamento per volto, dal secondo passaggio o dai gruppi piccoli (D3), oppure diventano domande.
2. **Abbinamento per volto alle persone con nome** (al salvataggio e, in lettura, per i volti senza persona): il centroide serve solo da filtro (≥ 0,8 × soglia). Il volto deve avere un collegamento robusto (mediana con i 12 volti migliori della persona) ≥ alla soglia di somiglianza, e staccare la seconda persona di 0,05. Resta la soglia di somiglianza del requisito "Identificazione delle persone", che non cambia; i volti di riferimento delle identità dell'app usano ancora la loro soglia.
3. **Il riabbinamento in lettura riguarda solo i volti senza persona.** Prima ogni analisi riabbinava al centroide tutti i volti non manuali, anche quelli già in un gruppo. Ora i gruppi esistenti passano dal secondo passaggio e da D3.
4. **"Almeno metà dei suoi volti" (D5)** si conta sui volti che avevano già una persona: i volti nuovi non tolgono l'id al gruppo in cui entrano.
5. **Incrementale e completo.** Il ricalcolo è completo se cambiano la versione dell'algoritmo (quindi alla prima analisi dopo l'aggiornamento), la somiglianza, la sensibilità o le soglie di qualità dei volti (chiave `faces.cluster_run` in `settings_kv`). Negli altri casi Chinese Whispers visita solo i volti nuovi e i loro vicini, e il secondo passaggio e D3 considerano solo i gruppi toccati. Una Rianalisi senza cambi di soglie resta incrementale; `cluster::run_with(.., Some(Mode::Full))` è pronto per un comando esplicito.
6. **Sensibilità.** Il tasso obiettivo è 10^(−4 + 2s). Il valore predefinito di "unisci da solo" è 0,55 + 0,2 × max(0; 0,5 − s), cioè 0,65 verso "Più domande", e non scende mai sotto 0,55. La metà destra della scala cambia qualcosa solo se la taratura sulla libreria supera 0,55. "Chiedi" non scende sotto la soglia di somiglianza impostata; se serve, "unisci da solo" sale a "chiedi" + 0,05. L'origine è "tarate" solo se la libreria ha spostato almeno una soglia.
7. **Domande**: per ogni gruppo senza nome al massimo una verso una persona con nome ("È A?" o "È A o B?") e una verso un altro gruppo, solo tra gruppi vicini nel grafo e sopra "chiedi". Probabilità = (collegamento − chiedi) / (unisci da solo − chiedi). Le coppie oltre "unisci da solo" ma non reciproche, o frenate dalle date lontane, diventano domande.
8. **Secondo passaggio**: coda di priorità dalla coppia migliore, con reciprocità e vincoli ricontrollati all'estrazione; dopo ogni unione si rimettono in coda le coppie del gruppo nuovo e dei vicini. I gruppi di al massimo 3 volti verso persone con nome sono lasciati a D3. Il margine verso la seconda persona vale anche per i gruppi più grandi diretti a una persona con nome.
9. **Qualità bassa** (D3): qualità combinata < 0,4.
10. **Flag "confermato" e indice per foto** (task 5.1): il flag è `assigned_by = 'manual'`, che ricevono anche i volti passati da una risposta della coda; l'indice per foto è `face_media` (v1). Nessuna colonna nuova.
11. **Storico**: si conservano 90 giorni, quindi i conteggi delle unioni in Diagnostica valgono per 90 giorni. "Annulla" di una voce che è anche l'ultima operazione annullabile usa Ctrl+Z (stato completo); altrimenti riporta solo i volti ancora nella destinazione e toglie la voce dal registro di Ctrl+Z. Una risposta "Sì" annullata non rimette la domanda: tornerà alla prossima analisi.
12. **Prestazioni (task 3.5)**: test ignorato `centomila_volti_entro_il_budget` (103.152 volti sintetici, 10.000 identità, ricalcolo completo), eseguito una volta con il profilo di sviluppo (dipendenze ottimizzate, crate del progetto senza ottimizzazioni): 407,6 s; memoria in più al picco 436 MB contro i 520 MB stimati per il governatore (5 KiB per volto + 16 MB, stima alzata da 4 KiB); annullamento dopo 2 s con risposta in 3,3 ms. Il tempo in build di rilascio non è stato misurato: va confermato che resti nell'ordine del primo passaggio.

### Risposte alle Open Questions

- Soglie e margine: restano 0,55 / 0,363 / 0,05 (vedi sopra).
- Anni per il margine aggiuntivo: 3 (`aggregate::FAR_MS`). Non verificato sulla libreria di prova: l'esportazione non include le date (solo id anonimi ed embedding).
- Campioni per coppia: 12×12, mantenuto. Sul banco (1.081 volti, build di debug) il raggruppamento nuovo richiede 1,4 s contro 0,8 s di prima; 32×32 non è stato provato.

### Comandi per le interfacce rinviate (task 5.3, 5.4, 6.3)

- "Unioni recenti": `listMerges(person, limit)` (ultimi 90 giorni, con copertine come id di volto per `face://`, origine e `undoable`) e `undoMerge(id)`.
- Riepilogo della prima riaggregazione: `aggregationSummary()` (una sola volta finché non si chiama) e `dismissAggregationSummary()`.
- Coda "Da confermare": `reviewQueue(limit)` (elementi singoli o in blocco verso la stessa persona, domande aperte, gruppi senza nome per la coda vuota); `answerQuestion(question, answer, overrideConflict)` con `answer` = `yes`, `no`, `choose {person}`, `neither`; `answerQuestions(person, confirm, reject, overrideConflict)` per "Conferma tutti". Un conflitto torna in `AnswerOutcome.conflict` senza modifiche: per togliere il vincolo si ripete la risposta con `overrideConflict = true`.
- Le risposte e le unioni emettono `faces_changed`; le probabilità servono solo all'ordine e alle etichette, non vanno mostrate come numeri.
