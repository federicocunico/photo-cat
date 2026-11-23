# face-recognition Specification

## Purpose
Rileva i volti nelle foto, li raggruppa per somiglianza in persone e permette all'utente di dare un nome alle persone e correggere i raggruppamenti, mantenendo i dati biometrici solo in locale.

## Requirements

### Requirement: Rilevamento volti
Il sistema SHALL rilevare i volti presenti in ogni foto analizzata, memorizzandone posizione e qualità; i volti sotto una dimensione o qualità minima configurabile SHALL essere ignorati per il riconoscimento. La qualità di un volto SHALL combinare confidenza del rilevatore, dimensione, nitidezza del ritaglio e frontalità; la confidenza minima e la nitidezza minima sono soglie impostate dall'utente.

#### Scenario: Volti rilevati
- **WHEN** viene analizzata una foto con 3 volti ben visibili
- **THEN** la foto ha 3 volti associati con le rispettive aree

#### Scenario: Volto troppo piccolo
- **WHEN** un volto è sotto la dimensione minima
- **THEN** il volto non viene usato per il raggruppamento

#### Scenario: Soglia di rilevamento alzata
- **WHEN** una foto ha volti con confidenza 0,95 e 0,85 e l'utente porta la soglia di rilevamento volti a 0,9
- **THEN** solo il volto con confidenza 0,95 viene usato per il raggruppamento

#### Scenario: Volto mosso
- **WHEN** una foto contiene un volto ben inquadrato ma mosso, con nitidezza sotto la soglia minima
- **THEN** il volto viene salvato come "scartato per qualità" con motivo "sfocato" e non viene usato per il raggruppamento né per i suggerimenti

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

### Requirement: Identificazione delle persone
L'utente SHALL poter assegnare a un gruppo di volti un'identità dell'archivio dell'app, scegliendone una esistente o creandone una nuova; solo questa scelta dell'utente trasforma un gruppo in una persona. I nuovi volti sufficientemente simili a una persona collegata o ai volti di riferimento di un'identità SHALL essere associati automaticamente a quella identità, con la soglia di somiglianza dell'identità.

#### Scenario: Nome assegnato e propagato
- **WHEN** l'utente collega un gruppo all'identità "Giulia" e poi importa nuove foto in cui Giulia compare
- **THEN** le nuove foto riportano "Persone: Giulia"

#### Scenario: Volto sotto soglia
- **WHEN** un nuovo volto ha somiglianza con "Giulia" inferiore alla soglia dell'identità (o, se non impostata, a quella configurata)
- **THEN** il volto non viene associato automaticamente a "Giulia"

#### Scenario: Riconoscimento in una libreria nuova
- **WHEN** l'identità "Giulia" ha volti di riferimento e l'utente analizza una libreria nuova in cui Giulia compare
- **THEN** i volti di Giulia vengono raggruppati in una persona collegata all'identità "Giulia" senza che l'utente la nomini

#### Scenario: Suggerimento in "Chi è?"
- **WHEN** l'utente scrive "Giu" nel campo "Chi è?" di un gruppo senza nome
- **THEN** il sistema propone le identità il cui nome o alias contiene "Giu", ordinate per somiglianza con i volti del gruppo

#### Scenario: Nessuna persona creata automaticamente
- **WHEN** l'analisi trova un gruppo di 300 volti che non somiglia a nessuna identità
- **THEN** il gruppo resta in "Da rivedere" e non compare tra le Persone finché l'utente non gli dà un nome

### Requirement: Correzione dei raggruppamenti
L'utente SHALL poter unire due o più gruppi o persone in un'unica operazione, separare volti da una persona, spostare volti in un altro gruppo o persona, indicare "non è questa persona", indicare che due gruppi sono "persone diverse" e rimuovere un volto; le correzioni MUST essere rispettate nei raggruppamenti successivi e nei suggerimenti.

#### Scenario: Volto rifiutato
- **WHEN** l'utente indica che un volto non appartiene a "Marco"
- **THEN** quel volto non viene più associato a "Marco" anche dopo un nuovo raggruppamento

#### Scenario: Unione persone
- **WHEN** l'utente unisce "Senza nome 12" con "Giulia"
- **THEN** tutti i volti risultano di "Giulia" e "Senza nome 12" non esiste più

#### Scenario: Unione multipla
- **WHEN** l'utente seleziona nell'elenco tre gruppi senza nome e la persona "Marco" e sceglie "Unisci"
- **THEN** tutti i volti dei tre gruppi risultano di "Marco" e i tre gruppi non esistono più

#### Scenario: Persone diverse
- **WHEN** l'utente indica che due gruppi suggeriti come simili sono persone diverse
- **THEN** il suggerimento non viene più proposto e i due gruppi non vengono uniti dai raggruppamenti successivi

### Requirement: Persone ignorate
L'utente SHALL poter ignorare un gruppo di volti o una persona: un gruppo ignorato MUST NOT comparire in "Da rivedere", nei filtri e nei conteggi delle persone della libreria, nell'Inspector e nei suggerimenti, e i nuovi volti simili SHALL esservi associati senza essere proposti. L'utente SHALL poter ripristinare un gruppo ignorato. Ignorare MUST NOT rimuovere foto dalla libreria né file dal disco.

#### Scenario: Persona che non interessa
- **WHEN** l'utente ignora un gruppo di 140 volti di uno sconosciuto ricorrente
- **THEN** il gruppo non compare più in "Da rivedere" né nel filtro Persone della libreria, e le 140 foto restano in libreria

#### Scenario: Nuove foto della persona ignorata
- **WHEN** dopo averlo ignorato vengono importate 6 nuove foto con quello sconosciuto
- **THEN** i 6 nuovi volti finiscono nel gruppo ignorato e la sezione "Ignorati" indica "+6 volti nuovi"

#### Scenario: Ripristino
- **WHEN** l'utente ripristina un gruppo dalla sezione "Ignorati"
- **THEN** il gruppo torna in "Da rivedere" con tutti i suoi volti

### Requirement: Volti scartati per qualità
I volti scartati per qualità SHALL restare consultabili, per gruppo e complessivamente, con il motivo dello scarto (sfocato, troppo piccolo, poco frontale, confidenza bassa), e l'utente SHALL poter recuperarli, rendendoli disponibili per il raggruppamento. Copertine e volti di riferimento proposti SHALL essere scelti tra i volti di qualità migliore.

#### Scenario: Recupero di un volto
- **WHEN** l'utente seleziona un volto scartato come sfocato e sceglie "Recupera"
- **THEN** il volto entra nel raggruppamento al successivo aggiornamento e non viene più scartato per qualità

#### Scenario: Copertina
- **WHEN** un gruppo contiene volti nitidi e volti mossi
- **THEN** la copertina del gruppo è un volto nitido

#### Scenario: Soglia di nitidezza cambiata
- **WHEN** l'utente abbassa la nitidezza minima dei volti
- **THEN** i volti scartati che ora superano la soglia entrano nel raggruppamento senza rianalizzare le foto

### Requirement: Apertura diretta di una persona
Da ogni punto dell'app che mostra una persona o un volto (Inspector, filtri, Sequenza, cataloghi) il comando "Apri in Persone" SHALL aprire il modulo Persone direttamente sulla persona o sul gruppo di quel volto. Quando il comando parte da una foto, il volto di quella foto SHALL essere evidenziato e portato in vista.

#### Scenario: Dall'Inspector
- **WHEN** l'utente seleziona una foto in Libreria e sceglie "Apri in Persone" sul chip "Giulia"
- **THEN** si apre la persona "Giulia" con il volto di quella foto evidenziato e visibile, anche se la persona ha 400 volti

#### Scenario: Volto non riconosciuto
- **WHEN** la foto ha un volto in un gruppo senza nome e l'utente sceglie "Apri in Persone" su quel volto
- **THEN** si apre il gruppo in "Da rivedere" con il volto evidenziato, pronto per "Chi è?"

#### Scenario: Volto ignorato o scartato
- **WHEN** il volto della foto appartiene a un gruppo ignorato o è scartato per qualità
- **THEN** si apre rispettivamente la sezione "Ignorati" sul gruppo, o la vista dei volti scartati, con il volto evidenziato

### Requirement: Privacy dei dati biometrici
I dati dei volti (ritagli ed embedding, inclusi i volti di riferimento delle identità) MUST restare esclusivamente sul computer dell'utente; l'utente SHALL poter disattivare il riconoscimento volti e cancellare tutti i dati dei volti in un'unica azione.

#### Scenario: Cancellazione dati volti
- **WHEN** l'utente sceglie "Elimina tutti i dati dei volti"
- **THEN** volti, embedding, persone e volti di riferimento delle identità vengono eliminati, le identità restano con nome, alias e gruppo, e il riconoscimento resta disattivato finché l'utente non lo riattiva

#### Scenario: Riconoscimento disattivato
- **WHEN** il riconoscimento volti è disattivato
- **THEN** l'analisi non calcola embedding facciali ma continua a contare le persone presenti

### Requirement: Dal volto alla foto
Il clic destro su un volto nella griglia di Persone SHALL aprire un menu che agisce sulla selezione corrente (o sul solo volto se non selezionato) con "Apri la foto", "Mostra in Libreria", "Apri con ▸" e "Mostra nella cartella", oltre alle azioni sui volti già disponibili. Il doppio clic e Invio su un volto SHALL aprire la foto nel visore. Il menu SHALL essere accessibile da tastiera.

#### Scenario: Apri la foto
- **WHEN** l'utente fa clic destro su un volto di "Giulia" e sceglie "Apri la foto"
- **THEN** si apre nel visore la foto da cui è preso il volto, con il riquadro del volto evidenziato

#### Scenario: Mostra in Libreria
- **WHEN** l'utente sceglie "Mostra in Libreria" su un volto
- **THEN** si apre la Libreria nella cartella della foto, con la foto selezionata e portata in vista

#### Scenario: Apri con un editor esterno su più volti
- **WHEN** l'utente seleziona 4 volti di 3 foto diverse e sceglie "Apri con ▸ Photoshop"
- **THEN** le 3 foto, ciascuna una sola volta, vengono passate a Photoshop

#### Scenario: Doppio clic
- **WHEN** l'utente fa doppio clic su un volto
- **THEN** la foto si apre nel visore e chiudendolo si torna alla stessa posizione della griglia dei volti

#### Scenario: Foto mancante sul disco
- **WHEN** la foto del volto è segnata mancante e l'utente sceglie "Apri con ▸"
- **THEN** compare un avviso che il file non è disponibile, con il percorso, e nessuna applicazione viene avviata

### Requirement: Ordinamento dei volti per data
La griglia dei volti di una persona o di un gruppo SHALL poter essere ordinata per qualità (predefinito), per data di scatto dalla più vecchia o per data di scatto dalla più recente. Negli ordinamenti per data SHALL mostrare un'intestazione per anno con il numero di volti, e i volti senza data SHALL comparire in fondo sotto "Senza data". La scelta SHALL essere ricordata.

#### Scenario: Persona per anno
- **WHEN** l'utente apre "Nonno Piero" con volti dal 1958 al 2004 e sceglie "Data: dalla più vecchia"
- **THEN** i volti sono in ordine cronologico con le intestazioni "1958", "1959", … e il numero di volti di ciascun anno

#### Scenario: Date impostate a mano
- **WHEN** una scansione ha data manuale "1962" e l'ordinamento è per data
- **THEN** il suo volto compare sotto l'intestazione "1962"

#### Scenario: Scelta ricordata
- **WHEN** l'utente sceglie "Data: dalla più recente", chiude l'app e apre un'altra persona
- **THEN** i volti sono ordinati dalla data più recente

#### Scenario: Persona con molti volti
- **WHEN** la persona ha 3.000 volti e l'utente cambia ordinamento
- **THEN** la griglia si aggiorna caricando i dati a pagine senza blocchi percepibili, e la selezione esistente resta valida

### Requirement: Assegnazione in blocco dei gruppi
In "Da rivedere" l'utente SHALL poter filtrare i gruppi per numero di foto ("1 foto", "fino a 2 foto", "fino a N foto") e selezionarli tutti; in "Singoli" SHALL poter selezionare tutti i volti. Con "Assegna a…" SHALL poter spostare i volti selezionati in una persona o identità scelta con ricerca, anche non selezionata, oppure in una nuova persona con nome, in un'unica operazione con un solo "Annulla", anche con centinaia di gruppi.

#### Scenario: Gruppi con una foto assegnati a una persona
- **WHEN** l'utente filtra "1 foto" in "Da rivedere", preme Ctrl+A sui 420 gruppi e sceglie "Assegna a…" › "Nonno Piero"
- **THEN** tutti i volti dei 420 gruppi risultano di "Nonno Piero", i 420 gruppi non esistono più e un avviso offre "Annulla"

#### Scenario: Annulla
- **WHEN** dopo l'assegnazione di 420 gruppi l'utente sceglie "Annulla"
- **THEN** i 420 gruppi tornano com'erano, con i loro volti, e "Nonno Piero" ha di nuovo solo i suoi volti

#### Scenario: Identità non ancora collegata
- **WHEN** l'utente assegna 12 gruppi all'identità "Zia Anna", che non ha ancora una persona in questa libreria
- **THEN** i 12 gruppi diventano un'unica persona collegata a "Zia Anna"

#### Scenario: Nuova persona
- **WHEN** l'utente assegna 5 gruppi a "Nuova persona…" e scrive "Vicino di casa"
- **THEN** i 5 gruppi diventano un'unica persona "Vicino di casa"

#### Scenario: Volti singoli
- **WHEN** in "Singoli" l'utente preme Ctrl+A sui 230 volti e sceglie "Assegna a…" › "Sconosciuti"
- **THEN** i 230 volti risultano di "Sconosciuti" e un avviso offre "Annulla"

#### Scenario: Correzioni rispettate
- **WHEN** un volto di un gruppo assegnato era stato rifiutato per la persona di destinazione
- **THEN** quel volto non viene assegnato alla destinazione e resta in un gruppo da rivedere

### Requirement: Comandi di correzione dei volti
Le correzioni SHALL usare due soli verbi: "Sposta in…" per volti selezionati e "Unisci a…" per gruppi o persone. Le azioni sui volti SHALL comparire in una barra che sostituisce l'intestazione dei volti solo quando c'è almeno un volto selezionato: "Sposta in…", "Non è questa persona" e un menu "Altro" con "Nuovo gruppo con questi volti", "Usa come copertina" (un solo volto) e "Rimuovi volti…".

#### Scenario: Volti selezionati
- **WHEN** l'utente seleziona tre volti di un gruppo
- **THEN** al posto dell'intestazione compare "3 selezionati" con "Sposta in…", "Non è questa persona", "Altro" e "Deseleziona", su una riga

#### Scenario: Copertina dal volto
- **WHEN** l'utente seleziona un solo volto e apre "Altro"
- **THEN** il menu contiene "Usa come copertina", e il volto diventa la copertina del gruppo

#### Scenario: Unire un gruppo
- **WHEN** l'utente sceglie "Unisci a…" dalla scheda di un gruppo senza nome e sceglie "Gioia"
- **THEN** tutti i volti del gruppo passano a Gioia e un avviso dice "Gruppo unito a Gioia" con "Annulla operazione"

### Requirement: Selettore della destinazione
"Sposta in…" e "Unisci a…" SHALL aprire lo stesso selettore con ricerca, che elenca prima le persone con nome e poi i gruppi senza nome. Ogni voce SHALL mostrare un volto di copertina, il nome e il numero di foto. Il selettore SHALL permettere di creare una persona nuova con il nome scritto nella ricerca.

#### Scenario: Gruppi senza nome distinguibili
- **WHEN** l'utente apre "Unisci a…" e ci sono sei gruppi senza nome
- **THEN** ogni gruppo senza nome compare con il proprio volto di copertina e il numero di foto, così che le voci si distinguono a colpo d'occhio

#### Scenario: Persona nuova dal selettore
- **WHEN** l'utente scrive "Marta" nel selettore e nessuna persona ha quel nome
- **THEN** il selettore propone "Nuova persona «Marta»", che crea la persona e vi sposta i volti

### Requirement: Suggerimento di identità nella scheda
Quando un gruppo senza nome somiglia a una persona o a un altro gruppo, la scheda SHALL mostrare in cima la domanda "È {nome}?", oppure "È la stessa persona?" se l'altro gruppo non ha nome, con i due volti di copertina affiancati e i comandi "Sì" e "No". La somiglianza SHALL essere indicata in parole (alta, media), senza valori numerici. Nell'elenco il gruppo SHALL riportare "forse {nome}".

#### Scenario: Conferma del suggerimento
- **WHEN** l'utente apre un gruppo che somiglia a Gioia e sceglie "Sì"
- **THEN** il gruppo viene unito a Gioia, si apre il gruppo successivo dell'elenco e l'avviso offre "Annulla operazione"

#### Scenario: Rifiuto del suggerimento
- **WHEN** l'utente sceglie "No" alla domanda "È Gioia?"
- **THEN** il suggerimento non viene più proposto per quei due elementi, come con "Persone diverse"

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
