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
