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
Il sistema SHALL raggruppare automaticamente i volti simili in gruppi e mostrare per ciascuno il numero di foto; i gruppi senza nome SHALL apparire nella sezione "Da rivedere". Due volti SHALL essere considerati della stessa persona solo se la loro somiglianza raggiunge la soglia di somiglianza configurata. Il sistema SHALL suggerire le coppie di gruppi o persone probabilmente uguali, così che l'utente possa unirle ed evitare doppioni.

#### Scenario: Stessa persona in più foto
- **WHEN** la stessa persona compare in 20 foto
- **THEN** i suoi volti sono raggruppati in un unico gruppo con conteggio 20

#### Scenario: Soglia più severa
- **WHEN** l'utente alza la soglia di somiglianza e salva
- **THEN** il raggruppamento viene ricalcolato dai volti salvati e i gruppi con volti poco simili si separano, mentre le persone con nome e le correzioni manuali restano invariate

#### Scenario: Gruppi probabilmente uguali
- **WHEN** la stessa persona è finita in due gruppi diversi perché fotografata a distanza di anni
- **THEN** in "Da rivedere" compare il suggerimento "Probabilmente la stessa persona" per quei due gruppi, con la loro somiglianza

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
