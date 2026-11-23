# Spec Delta

## MODIFIED Requirements

### Requirement: Raggruppamento in persone
Il sistema SHALL raggruppare automaticamente i volti simili in gruppi e mostrare per ciascuno il numero di foto; i gruppi senza nome SHALL apparire nella sezione "Da rivedere". Il raggruppamento SHALL avvenire in due passaggi: un primo passaggio preciso, poi un'unione dei gruppi vicini che confronta i loro volti, non i centroidi. Due gruppi SHALL essere uniti automaticamente solo se superano la soglia "unisci da solo" e sono vicini l'uno dell'altro in modo reciproco. Il sistema SHALL suggerire le coppie probabilmente uguali che superano la soglia "chiedi".

#### Scenario: Stessa persona in più foto
- **WHEN** la stessa persona compare in 20 foto
- **THEN** i suoi volti sono raggruppati in un unico gruppo con conteggio 20

#### Scenario: Soglia più severa
- **WHEN** l'utente alza la soglia di somiglianza e salva
- **THEN** il raggruppamento viene ricalcolato dai volti salvati e i gruppi con volti poco simili si separano, mentre le persone con nome e le correzioni manuali restano invariate

#### Scenario: Gruppi probabilmente uguali
- **WHEN** la stessa persona è finita in due gruppi diversi perché fotografata a distanza di anni e i due gruppi superano solo la soglia "chiedi"
- **THEN** in "Da rivedere" compare il suggerimento "Probabilmente la stessa persona" per quei due gruppi

#### Scenario: Frammenti uniti al secondo passaggio
- **WHEN** il primo passaggio produce per la stessa persona un gruppo di 30 volti e quattro gruppi di 1–2 volti, tutti oltre la soglia "unisci da solo" e reciprocamente vicini al gruppo grande
- **THEN** al termine dell'analisi esiste un solo gruppo di 36 volti, e lo storico delle unioni registra le quattro unioni automatiche

#### Scenario: Un volto ponte non basta
- **WHEN** due persone diverse hanno ciascuna un gruppo di 40 volti e un solo volto ambiguo è molto simile a entrambi
- **THEN** i due gruppi non vengono uniti, perché il confronto usa la mediana delle somiglianze tra i membri e non il volto più vicino

## ADDED Requirements

### Requirement: Volti della stessa foto
Due volti rilevati nella stessa foto MUST NOT essere assegnati automaticamente alla stessa persona o allo stesso gruppo, né proposti come la stessa persona. Lo stesso vale per le coppie segnate "Persone diverse" o "Non è questa persona". Solo una correzione esplicita dell'utente può assegnarli alla stessa persona.

#### Scenario: Gemelli nella stessa foto
- **WHEN** una foto contiene due gemelli e i loro volti superano la soglia "unisci da solo"
- **THEN** i due volti restano in gruppi diversi e nessun suggerimento li propone come la stessa persona

### Requirement: Abbinamento dei gruppi piccoli
Un gruppo senza nome con al massimo 3 volti SHALL essere assegnato automaticamente a una persona con nome solo se il punteggio verso quella persona supera la soglia "unisci da solo" e supera di un margine fisso quello della seconda persona più simile. Altrimenti, se supera la soglia "chiedi", SHALL diventare una domanda nella coda "Da confermare". Due persone con nome MUST NOT essere unite automaticamente.

#### Scenario: Candidata chiara
- **WHEN** un gruppo di 2 volti è molto simile a "Gioia" e molto meno simile a ogni altra persona
- **THEN** i 2 volti passano a Gioia e l'unione compare nello storico

#### Scenario: Due candidate vicine
- **WHEN** un gruppo di 1 volto è simile quasi allo stesso modo a "Anna" e a "Marta"
- **THEN** il volto non viene assegnato e compare nella coda con la domanda "È Anna o Marta?"

### Requirement: Soglie tarate sulla libreria
Le soglie "unisci da solo" e "chiedi" SHALL partire da valori predefiniti e adattarsi alla libreria. Si usano le somiglianze tra volti della stessa foto, che sono sempre persone diverse, e, quando disponibili, quelle tra volti confermati della stessa persona. La soglia "unisci da solo" MUST NOT scendere sotto il valore predefinito più severo previsto per il modello. L'utente SHALL poter regolare la sensibilità in Impostazioni › Riconoscimento AI, da "Più domande" a "Più unioni automatiche".

#### Scenario: Libreria con pochi dati
- **WHEN** la libreria ha meno di 200 coppie di volti nella stessa foto
- **THEN** le soglie restano ai valori predefiniti e Diagnostica lo indica

#### Scenario: Sensibilità ridotta
- **WHEN** l'utente sposta la sensibilità verso "Più domande" e applica
- **THEN** le unioni automatiche successive richiedono somiglianze più alte e le coppie escluse compaiono come domande

### Requirement: Coda Da confermare
"Da rivedere" SHALL offrire una coda "Da confermare" con le domande del sistema. Le domande SHALL essere ordinate in modo da risolvere più volti per risposta: prima i gruppi grandi senza nome, poi quelli piccoli in ordine di probabilità. I gruppi piccoli probabilmente della stessa persona SHALL essere proposti insieme, con tutti i volti visibili, "Conferma tutti" e la possibilità di togliere quelli sbagliati prima di confermare.

#### Scenario: Conferma in blocco
- **WHEN** la coda propone "12 gruppi probabilmente di Gioia" e l'utente toglie 2 gruppi e sceglie "Conferma tutti"
- **THEN** i 10 gruppi rimasti passano a Gioia con un'unica operazione annullabile, e i 2 gruppi tolti ricevono il vincolo "Non è questa persona" verso Gioia

#### Scenario: Coda vuota
- **WHEN** non ci sono domande aperte
- **THEN** la coda dice che non c'è nulla da confermare e indica quanti gruppi restano senza nome in "Da rivedere"

### Requirement: Propagazione delle risposte
Ogni "Sì" SHALL unire i due elementi e rendere superflue le domande ormai risolte per transitività. Ogni "No" SHALL diventare un vincolo e togliere le domande che contraddice. Una risposta in conflitto con un vincolo esistente SHALL essere mostrata come conflitto, con la possibilità di annullare il vincolo precedente.

#### Scenario: Domande risolte per transitività
- **WHEN** la coda contiene "A è Gioia?" e "A è lo stesso gruppo di B?", e l'utente risponde Sì alla seconda e poi alla prima
- **THEN** A e B passano entrambi a Gioia, e la domanda "B è Gioia?" non viene più proposta

#### Scenario: Conflitto
- **WHEN** l'utente aveva indicato che C non è Gioia e ora conferma che C è lo stesso gruppo di un gruppo già unito a Gioia
- **THEN** l'app mostra il conflitto e chiede se annullare il "Non è Gioia" precedente, senza unire nulla finché l'utente non sceglie

### Requirement: Gruppi stabili tra le analisi
Una nuova analisi SHALL mantenere identità, nome e copertina dei gruppi esistenti quando contengono in gran parte gli stessi volti. I volti confermati dall'utente (assegnati a mano o tramite una risposta della coda) MUST NOT essere spostati da un'analisi. I nuovi volti SHALL essere assegnati ai gruppi esistenti senza ricalcolare quelli non coinvolti.

#### Scenario: Import di poche foto
- **WHEN** l'utente importa 20 foto nuove in una libreria con 300 gruppi
- **THEN** i nuovi volti entrano nei gruppi esistenti o in gruppi nuovi, e nessun altro gruppo cambia identità, copertina o posizione nell'elenco

### Requirement: Storico delle unioni
Ogni unione, manuale o automatica, SHALL essere registrata in uno storico consultabile da Persone › "Unioni recenti", con data, origine (manuale, automatica, coda), gruppi coinvolti con la copertina, persona di destinazione e numero di volti. Ogni voce SHALL offrire "Annulla", che riporta i volti nei gruppi di prima e impedisce che la stessa unione automatica venga ripetuta.

#### Scenario: Ritrovare un gruppo unito
- **WHEN** l'utente cerca un gruppo che ha unito ieri e non lo trova più in "Da rivedere"
- **THEN** in "Unioni recenti" trova la voce "Gruppo di 3 volti → Gioia", con la copertina del gruppo e il comando "Annulla"

#### Scenario: Unione automatica annullata
- **WHEN** l'utente annulla un'unione automatica
- **THEN** il gruppo torna com'era e le analisi successive non lo uniscono di nuovo alla stessa persona

### Requirement: Misure del raggruppamento
Diagnostica SHALL mostrare dopo ogni analisi:
- il numero medio di gruppi per persona con nome;
- la quota di gruppi con un solo volto;
- il numero di unioni automatiche e quante sono state annullate;
- le soglie in uso e da dove vengono (predefinite o tarate).

#### Scenario: Frammentazione visibile
- **WHEN** l'utente apre Diagnostica dopo un'analisi
- **THEN** vede ad esempio "1,4 gruppi per persona con nome · 18% di gruppi con un volto · 52 unioni automatiche, 1 annullata"
