# Tasks

Ogni gruppo si integra da solo con test verdi (`cargo test` con `CARGO_BUILD_JOBS=6` e `CARGO_PROFILE_DEV_DEBUG=0`, un solo processo cargo alla volta; `npm --prefix app/ui test`). D = `design.md`.

## 1. Preparazione e misura di partenza

- [ ] 1.1 Creare il branch `change/face-identity-aggregation` da `main` aggiornato, dopo il merge di `ui-usability-audit`, e verificare che `make test` passi prima di ogni modifica
- [x] 1.2 Generatore di dati sintetici (identità sull'ipersfera a 128-d con frammenti lontani, gemelli nella stessa foto, volti ponte, volti di qualità bassa) e metriche (F a coppie, BCubed F, gruppi per identità, quota di gruppi da un volto, precisione delle unioni automatiche); verificare con test unitari su casi calcolati a mano (D9)
- [x] 1.3 Esempio `cluster_bench` che misura il raggruppamento attuale sui dati sintetici e su un file JSONL locale; verificare che riporti i valori di partenza e salvarli in `design.md` come riferimento (D9)
- [x] 1.4 Comando di sviluppo che esporta in un file locale gli embedding delle persone con nome della libreria dell'utente, mai nel repository (escluso da git); verificare con un test che il file contenga solo embedding e id anonimi

## 2. Vincoli e primo passaggio

- [x] 2.1 Vincolo della stessa foto nel grafo e nella scelta dell'etichetta di Chinese Whispers, e nei suggerimenti; verificare con un test che due volti identici nella stessa foto restino in gruppi diversi e non vengano mai proposti insieme (spec "Volti della stessa foto")
- [x] 2.2 Chinese Whispers deterministico (ordine fisso, parità per id) e avviato dalle etichette precedenti; verificare con un test che due esecuzioni sullo stesso input diano lo stesso risultato e che un import di 20 volti non cambi i gruppi non toccati (D1)
- [x] 2.3 Identità dei gruppi conservata con sovrapposizione ≥ 50% (nome, copertina, id); verificare lo scenario "Import di poche foto" (D5)

## 3. Secondo passaggio e gruppi piccoli

- [x] 3.1 Collegamento robusto (mediana su al massimo 12×12 campioni scelti per qualità) e coppie candidate dal grafo (al massimo 8 per gruppo); verificare con test che un volto ponte non unisca due gruppi da 40 volti (D2)
- [x] 3.2 Unione agglomerativa con vicinanza reciproca, vincoli e margine per date lontane, con le unioni registrate come automatiche; verificare lo scenario "Frammenti uniti al secondo passaggio" e il caso dei 3 anni di distanza (D2)
- [x] 3.3 Abbinamento dei gruppi di al massimo 3 volti alle persone con nome, con margine verso la seconda candidata, nessuna unione per gruppi di sola qualità bassa e domande per i casi incerti; verificare gli scenari di "Abbinamento dei gruppi piccoli" (D3)
- [x] 3.4 Banco di prova: sui dati sintetici la BCubed F migliora rispetto al punto 1.3 e la precisione delle unioni automatiche resta ≥ 0,99; aggiungere il test di regressione alla CI (D9)
- [x] 3.5 Prestazioni: 100.000 volti sintetici raggruppati entro il budget di memoria del governatore e interrompibili; verificare con un test ignorato di default e una misura riportata in `design.md`

## 4. Soglie tarate

- [x] 4.1 Taratura dalle coppie della stessa foto (quantili 1 − 10⁻³ e 1 − 10⁻²), con minimi predefiniti, controllo sulle coppie confermate e origine delle soglie salvata; verificare con test su distribuzioni sintetiche e sul caso con meno di 200 coppie (D4)
- [x] 4.2 Valori predefiniti per SFace confermati con `cluster_bench` sulla libreria dell'utente (partenza: 0,55, 0,363 e margine 0,05, da PhotoPrism e dalla soglia attuale), scritti nel codice con la fonte; verificare con il banco di prova e annotare i risultati in `design.md` (D4)
- [x] 4.3 Impostazioni › Riconoscimento AI: sensibilità dell'unione automatica ("Più domande ↔ Più unioni automatiche") con "Applica" e anteprima del numero di unioni; verificare con i test di Impostazioni e lo scenario "Sensibilità ridotta"

## 5. Storico delle unioni

- [x] 5.1 Migrazione: tabelle `merge_log` e `review_question`, flag "confermato" e indice dei volti per foto; verificare con il test delle migrazioni su un database della versione 0.5
- [x] 5.2 Registrazione di ogni unione (manuale, automatica, coda) con istantanea dei volti, copertina e nome d'origine; "Annulla" che ripristina e, per le automatiche, aggiunge il vincolo; `undoPeople` come vista sullo storico; verificare con test d'integrazione gli scenari di "Storico delle unioni" (D7)
- [ ] 5.3 Persone › "Unioni recenti": elenco degli ultimi 90 giorni con copertine, origine, filtro per persona e "Annulla"; verificare con test vitest
- [ ] 5.4 Riepilogo della prima riaggregazione dopo l'aggiornamento ("N gruppi uniti automaticamente", collegamento allo storico); verificare con un test che compaia una sola volta

## 6. Coda "Da confermare"

- [x] 6.1 Calcolo delle domande dopo ogni analisi, con l'ordine misto (gruppi grandi per volti risolti attesi, poi piccoli per probabilità) e il raggruppamento per persona di destinazione; verificare con test che l'ordine riduca le domande per arrivare a BCubed F 0,95 nel banco di prova rispetto all'ordine per probabilità (D6)
- [x] 6.2 Propagazione: union-find per i Sì, vincoli per i No, domande superflue tolte, conflitti rilevati; verificare gli scenari di "Propagazione delle risposte"
- [ ] 6.3 Interfaccia della coda in "Da rivedere": domanda singola "È {nome}?" o "È Anna o Marta?", schede in blocco con caselle e "Conferma tutti", coda vuota con il conteggio dei gruppi rimasti, tutto da tastiera; verificare con test vitest gli scenari di "Coda Da confermare"

## 7. Misure

- [x] 7.1 Calcolo e salvataggio dopo ogni analisi di gruppi per persona con nome, quota di gruppi da un volto, unioni automatiche e annullate, soglie e loro origine; voce nel log di diagnostica; verificare con test Rust (D8)
- [x] 7.2 Diagnostica: riquadro "Raggruppamento dei volti" con le misure; verificare lo scenario "Frammentazione visibile"

## 8. Verifica d'insieme

- [ ] 8.1 `make test` in locale e `make test-linux` in Docker, con le correzioni necessarie
- [ ] 8.2 Aggiornare README (Persone: coda, storico, sensibilità) e `research.md` con i risultati del banco di prova
- [ ] 8.3 Prova sull'app reale (per l'utente): libreria di prova da `D:\Foto` con un catalogo isolato; confronto prima/dopo delle misure in Diagnostica, delle unioni automatiche e del numero di domande per sistemare i gruppi piccoli

## Workflow follow-up

- Aprire la PR, attendere la CI verde e fare il merge.
- Archiviare la change al momento della release.
