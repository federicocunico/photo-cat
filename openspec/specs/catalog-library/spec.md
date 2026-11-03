# catalog-library Specification

## Purpose
Gestisce la libreria unica di tutti i media aggiunti e i cataloghi tematici (manuali e smart) con cui l'utente organizza, cerca e filtra i propri file.

## Requirements

### Requirement: Libreria unica dei media
Il sistema SHALL mantenere una libreria persistente e locale che contiene ogni file importato, una sola volta per percorso, con i suoi metadati tecnici, le date, le catalogazioni e lo stato di analisi.

#### Scenario: File importato appare in libreria
- **WHEN** un file supportato viene importato da una sorgente
- **THEN** il file compare in libreria con percorso completo, nome file e caratteristiche tecniche

#### Scenario: Persistenza tra riavvii
- **WHEN** l'utente chiude e riapre l'applicazione
- **THEN** la libreria mostra gli stessi file, catalogazioni e cataloghi di prima della chiusura

### Requirement: Modello di media estendibile
Ogni elemento della libreria SHALL avere un tipo di media; la prima versione supporta il tipo "foto" e il modello dati SHALL consentire di aggiungere altri tipi (es. video, documenti) e nuove catalogazioni senza invalidare i dati esistenti.

#### Scenario: Aggiornamento dello schema
- **WHEN** una nuova versione dell'applicazione introduce un nuovo campo di catalogazione
- **THEN** la libreria esistente viene migrata automaticamente e i dati precedenti restano disponibili

### Requirement: Cataloghi manuali
L'utente SHALL poter creare, rinominare ed eliminare cataloghi tematici manuali e aggiungervi o escludervi file; un file può appartenere a più cataloghi ed eliminare un catalogo non elimina i file. Escludere un file da un catalogo manuale lo toglie dal catalogo ma lo conserva tra le foto escluse di quel catalogo, da cui può essere reinserito.

#### Scenario: Aggiunta di file a un catalogo
- **WHEN** l'utente aggiunge 3 file al catalogo "Viaggio Toscana"
- **THEN** il catalogo mostra quei 3 file e il conteggio del catalogo è 3

#### Scenario: Eliminazione catalogo
- **WHEN** l'utente elimina un catalogo
- **THEN** i file restano in libreria e negli altri cataloghi

#### Scenario: Esclusione da un catalogo manuale
- **WHEN** il catalogo manuale "Viaggio Toscana" contiene 40 foto e l'utente ne esclude 6
- **THEN** il catalogo mostra 34 foto, la vista "Escluse" del catalogo mostra le 6 foto, e le 6 foto restano in libreria e sul disco

#### Scenario: Riaggiunta di una foto esclusa
- **WHEN** l'utente aggiunge di nuovo al catalogo una foto che ne era stata esclusa
- **THEN** la foto torna nel catalogo e non compare più tra le escluse

### Requirement: Cataloghi smart
L'utente SHALL poter definire cataloghi smart basati su regole combinate (AND/OR) su metadati, date, luogo, persone, catalogazioni AI e tag; il contenuto SHALL aggiornarsi automaticamente quando cambiano i dati dei file. Le foto escluse dall'utente da un catalogo smart MUST NOT comparirvi anche se soddisfano le regole.

#### Scenario: Regola smart
- **WHEN** l'utente crea un catalogo smart "Tipo soggetto = Ritratto AND Anno >= 2022"
- **THEN** il catalogo contiene esattamente i file che soddisfano entrambe le condizioni, tranne quelli esclusi dall'utente

#### Scenario: Aggiornamento automatico
- **WHEN** un nuovo file analizzato soddisfa le regole di un catalogo smart
- **THEN** il file compare nel catalogo senza azioni manuali

#### Scenario: Esclusione da un catalogo smart
- **WHEN** l'utente esclude 3 foto dal catalogo smart "Ritratti 2023"
- **THEN** il catalogo non mostra più le 3 foto, nemmeno dopo una rianalisi che le classifica ancora come ritratti

#### Scenario: Esclusione che sopravvive al cambio dei dati
- **WHEN** una foto esclusa da un catalogo smart smette di soddisfarne le regole e in seguito torna a soddisfarle
- **THEN** la foto continua a non comparire nel catalogo finché l'utente non la reinserisce

### Requirement: Foto escluse da un catalogo
Per ogni catalogo il sistema SHALL mostrare il numero di foto escluse e una vista che le elenca con la data di esclusione, da cui l'utente può reinserirne una o più. Nei cataloghi smart la vista SHALL mostrare per impostazione predefinita le escluse che oggi soddisfano le regole, con l'opzione di vederle tutte. Escludere e reinserire MUST NOT modificare i file sul disco né rimuovere foto dalla libreria.

#### Scenario: Revisione e reinserimento
- **WHEN** il catalogo "Viaggio Toscana" ha 6 foto escluse e l'utente apre "Escluse (6)", ne seleziona 2 e sceglie "Reinserisci nel catalogo"
- **THEN** le 2 foto tornano nel catalogo e la vista mostra "Escluse (4)"

#### Scenario: Esclusione rapida con annullamento
- **WHEN** l'utente guarda un catalogo, seleziona una foto e preme Canc
- **THEN** la foto viene esclusa dal catalogo senza dialogo di conferma, compare un avviso con "Annulla", e scegliendo "Annulla" la foto torna nel catalogo

#### Scenario: Canc fuori da un catalogo
- **WHEN** l'utente preme Canc nella libreria senza un catalogo aperto
- **THEN** le foto selezionate vanno nel Cestino (vedi "Cestino"), con un avviso con "Annulla operazione"; nessuna foto viene rimossa dalla libreria né dal disco

#### Scenario: Conteggi
- **WHEN** un catalogo ha 34 foto visibili e 6 escluse
- **THEN** la barra laterale mostra 34 per quel catalogo e l'intestazione del catalogo indica anche "6 escluse"

#### Scenario: Appartenenza in una regola smart
- **WHEN** una regola smart richiede "Catalogo = Viaggio Toscana" e una foto è esclusa da "Viaggio Toscana"
- **THEN** quella foto non soddisfa la condizione

### Requirement: Tag utente
L'utente SHALL poter assegnare e rimuovere tag liberi ai file; i tag utente SHALL essere distinti dalle catalogazioni automatiche e non essere mai sovrascritti da un'analisi. Ogni tag SHALL essere un'entità con nome univoco (senza distinzione di maiuscole) e colore, così che rinominarlo o unirlo a un altro mantenga le assegnazioni ai file.

#### Scenario: Rianalisi non tocca i tag
- **WHEN** un file con tag utente viene rianalizzato
- **THEN** i suoi tag utente restano invariati

#### Scenario: Stesso tag con maiuscole diverse
- **WHEN** l'utente aggiunge il tag "matrimonio" e in libreria esiste già "Matrimonio"
- **THEN** il file riceve il tag esistente "Matrimonio" e non ne viene creato uno nuovo

### Requirement: Ricerca e filtri
Il sistema SHALL permettere di filtrare la libreria o un catalogo per sorgente, cartella (con o senza sottocartelle), tipo soggetto, interno/esterno, persone, luogo, intervallo di date, formato e tag, e di cercare per testo su nome file, percorso, luogo, persone e tag.

#### Scenario: Filtri combinati
- **WHEN** l'utente seleziona i filtri "Esterno" e "Persona: Giulia"
- **THEN** vengono mostrati solo i file esterni in cui è presente Giulia

#### Scenario: Risposta su librerie grandi
- **WHEN** la libreria contiene 100.000 file e l'utente applica un filtro
- **THEN** il primo blocco di risultati viene mostrato entro 1 secondo su hardware consumer

#### Scenario: Filtro per cartella con sottocartelle
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone` includendo le sottocartelle
- **THEN** vengono mostrati i file di `Viaggio Giappone` e di tutte le sue sottocartelle (`D7000`, `Telefono`, …)

#### Scenario: Filtro per cartella esatta
- **WHEN** l'utente sceglie la cartella `D:\Foto\Viaggio Giappone\D7000` senza sottocartelle
- **THEN** vengono mostrati solo i file contenuti direttamente in quella cartella

### Requirement: Stato dei file sul disco
Il sistema SHALL segnalare i file della libreria non più presenti sul disco come "mancanti" senza rimuoverli automaticamente, e permettere all'utente di ricollegarli o rimuoverli. Il ricollegamento SHALL poter indicare la nuova posizione di una cartella e ritrovare i file per percorso relativo o per impronta del contenuto; la rimozione SHALL poter riguardare tutti i mancanti di una sorgente in un'unica azione confermata.

#### Scenario: File spostato fuori dal programma
- **WHEN** un file catalogato viene cancellato o spostato esternamente
- **THEN** al successivo controllo il file è marcato "mancante" e mantiene le sue catalogazioni

#### Scenario: Cartella spostata su un altro disco
- **WHEN** l'utente ha spostato `D:\Foto\Viaggio` in `E:\Archivio\Viaggio`, la libreria segna 400 file mancanti e l'utente sceglie "Ricollega…" indicando la nuova cartella
- **THEN** i 400 file tornano disponibili con le loro catalogazioni, i tag, i cataloghi e le persone, e il riepilogo indica quanti file sono stati ritrovati e quanti restano mancanti

#### Scenario: File rinominati nella nuova cartella
- **WHEN** nella cartella indicata per il ricollegamento alcuni file hanno un nome diverso ma lo stesso contenuto
- **THEN** quei file vengono ricollegati tramite impronta del contenuto

#### Scenario: Rimozione dei mancanti
- **WHEN** una sorgente ha 5 file mancanti e l'utente sceglie "Rimuovi mancanti…"
- **THEN** dopo una conferma che indica 5 file, quei file non sono più in libreria e gli altri file della sorgente restano invariati

### Requirement: Albero delle cartelle
Il sistema SHALL mostrare, sotto ogni sorgente, l'albero delle cartelle che contengono file della libreria con il numero di file per cartella (incluse le sottocartelle); l'albero SHALL caricarsi un livello alla volta per restare reattivo su librerie grandi.

#### Scenario: Espansione di una sorgente
- **WHEN** l'utente espande la sorgente `D:\Foto` che contiene `Viaggio Giappone` con 3 sottocartelle
- **THEN** compare `Viaggio Giappone` con il conteggio totale e, espandendola, le 3 sottocartelle con i rispettivi conteggi

#### Scenario: Seleziona tutto in una cartella
- **WHEN** l'utente sceglie una cartella nell'albero e poi "Seleziona tutto"
- **THEN** risultano selezionati tutti i file di quella cartella, pronti per un'azione bulk

### Requirement: Elenco completo dei valori dei filtri
Ogni lista di filtro della libreria (catalogazioni AI per gruppo, luoghi, anni, formati, tag) SHALL mostrare in forma compatta le voci con più foto, il numero totale di voci e un comando per vederle tutte. Ogni voce esistente nei risultati SHALL essere raggiungibile e selezionabile, senza limiti fissi al numero di voci, con ricerca testuale e ordinamento per numero di foto o alfabetico.

#### Scenario: Molte località
- **WHEN** la libreria contiene foto in 340 luoghi diversi
- **THEN** la lista Luoghi mostra le voci principali e "Mostra tutti (340)", e dall'elenco completo è possibile selezionare il luogo con meno foto

#### Scenario: Ricerca nell'elenco
- **WHEN** l'utente apre l'elenco completo dei luoghi e digita "sie"
- **THEN** compaiono solo i luoghi il cui nome contiene "sie" (senza distinzione di maiuscole e accenti), con i rispettivi conteggi

#### Scenario: Ordinamento alfabetico
- **WHEN** l'utente sceglie l'ordinamento alfabetico nell'elenco completo dei tag
- **THEN** i tag sono elencati in ordine alfabetico invece che per numero di foto

#### Scenario: Voce selezionata sempre visibile
- **WHEN** l'utente ha selezionato un luogo che non è tra le voci principali e richiude l'elenco completo
- **THEN** il luogo selezionato resta visibile e spuntato nella lista compatta

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 foto e 5.000 tag e l'utente apre l'elenco completo dei tag
- **THEN** le prime voci compaiono entro 1 secondo e lo scorrimento dell'elenco resta fluido

### Requirement: Conteggi dei filtri indipendenti dalla propria selezione
Le voci e i conteggi di una lista di filtro SHALL essere calcolati applicando tutti gli altri filtri attivi ma non la selezione della lista stessa, così che le altre voci della stessa lista restino visibili e selezionabili.

#### Scenario: Selezione di più luoghi
- **WHEN** l'utente seleziona il luogo "Siena" e la lista Luoghi conteneva anche "Firenze" con 80 foto
- **THEN** "Firenze" resta nella lista con 80 foto e può essere aggiunto alla selezione

#### Scenario: Altri filtri applicati
- **WHEN** è attivo il filtro "Esterno" e l'utente guarda la lista Luoghi
- **THEN** i conteggi dei luoghi considerano solo le foto esterne

### Requirement: Ordinamento per istante reale
La libreria e i cataloghi SHALL poter essere ordinati per istante reale di scatto (dopo le correzioni di fuso e scostamento) e il filtro per intervallo di date SHALL usare l'ora locale corretta di ciascuna foto.

#### Scenario: Ordinamento dopo una correzione
- **WHEN** l'utente applica uno scostamento di +2 minuti a un gruppo di sorgente e ordina la libreria per data di scatto
- **THEN** le foto di quel gruppo occupano la posizione corrispondente all'istante corretto

### Requirement: Filtro per data gerarchico
Il filtro per data della libreria SHALL essere una gerarchia anno → mese → giorno con una sola scelta per volta. Scelto un anno SHALL comparire sotto di esso l'elenco dei mesi di quell'anno con foto; scelto un mese, l'elenco dei giorni di quel mese con foto; ogni voce con il numero di foto. Mesi e giorni SHALL esistere solo dentro l'anno o il mese scelto, mai come filtro a sé. Date e conteggi SHALL usare l'ora locale corretta di ogni foto.

#### Scenario: Scelta del mese dentro l'anno
- **WHEN** la libreria contiene foto del 2023 scattate a marzo e a maggio e l'utente sceglie l'anno 2023
- **THEN** sotto "2023" compaiono "marzo" e "maggio" con i rispettivi conteggi, e scegliendo "maggio" la griglia mostra solo le foto di maggio 2023

#### Scenario: Scelta del giorno dentro il mese
- **WHEN** l'utente ha scelto maggio 2023 e in quel mese ci sono foto del 14 e del 20
- **THEN** sotto "maggio" compaiono i giorni 14 e 20 con i conteggi, e scegliendo il 14 la griglia mostra solo le foto del 14 maggio 2023

#### Scenario: Mesi non selezionabili senza anno
- **WHEN** nessun anno è scelto
- **THEN** la lista mostra solo gli anni e non è possibile filtrare per un mese o un giorno di tutti gli anni

#### Scenario: Risalire la gerarchia
- **WHEN** è scelto il 14 maggio 2023 e l'utente toglie la spunta al giorno, poi al mese, poi all'anno
- **THEN** il filtro diventa prima maggio 2023, poi l'anno 2023, poi nessun filtro per data

#### Scenario: Cambio di anno o di mese
- **WHEN** è scelto maggio 2023 e l'utente sceglie l'anno 2022 oppure il mese di marzo 2023
- **THEN** il filtro diventa rispettivamente l'anno 2022 intero oppure marzo 2023, senza conservare il mese o il giorno precedente

#### Scenario: Altri mesi restano selezionabili
- **WHEN** è attivo il filtro "Esterno", l'utente ha scelto maggio 2023 e marzo 2023 contiene 40 foto esterne
- **THEN** "marzo" resta nella lista con 40 foto e sceglierlo sostituisce maggio

#### Scenario: Ora locale corretta
- **WHEN** una foto ha istante reale 31 maggio 2023 23:30 UTC e il fuso dell'ora mostrata è +02:00
- **THEN** la foto viene contata nel 1° giugno 2023 e compare scegliendo giugno 2023, non maggio

#### Scenario: Foto senza data
- **WHEN** alcune foto non hanno data di scatto
- **THEN** non sono contate in nessun anno, mese o giorno e non compaiono quando è scelto un filtro per data

#### Scenario: Chip del filtro attivo
- **WHEN** è scelto il 14 maggio 2023 con l'interfaccia in italiano
- **THEN** tra i filtri attivi compare il chip "14 maggio 2023" e rimuoverlo toglie il filtro per data

### Requirement: Editor delle regole comprensibile
L'editor dei cataloghi smart SHALL presentare ogni condizione come campo, confronto e valore espressi nella lingua dell'interfaccia, e MUST NOT mostrare identificatori interni di campi o operatori. Per il valore SHALL offrire un selettore adatto al campo: elenco con ricerca, numero con unità o anno. L'editor SHALL mostrare un riepilogo della regola in una frase e il numero di foto corrispondenti, aggiornati durante la modifica.

#### Scenario: Nuova regola
- **WHEN** l'utente aggiunge una condizione e sceglie il campo "Persone nella foto"
- **THEN** i confronti offerti sono "è", "non è", "più di", "almeno", "meno di", "al massimo", e il valore è un numero

#### Scenario: Riepilogo della regola
- **WHEN** la regola è "Tipo soggetto è Ritratto e Anno è almeno 2022"
- **THEN** l'editor mostra la frase "Foto con tipo soggetto Ritratto, dal 2022 in poi" e il numero di foto corrispondenti

### Requirement: Conteggi nell'elenco dei cataloghi
Nell'elenco dei cataloghi ogni catalogo SHALL mostrare il tipo, il numero di foto e, se presenti, il numero di foto escluse, con accesso diretto alla vista delle escluse.

#### Scenario: Catalogo con esclusioni nell'elenco
- **WHEN** il catalogo "Viaggio Toscana" ha 312 foto e 6 escluse
- **THEN** l'elenco dei cataloghi mostra "Smart · 312 foto · 6 escluse", e il clic su "6 escluse" apre la vista delle foto escluse

### Requirement: Cataloghi di sistema
La libreria SHALL avere tre cataloghi di sistema creati automaticamente: "Cestino", "Duplicati" e "Qualità insufficiente", con il nome nella lingua dell'interfaccia. L'utente MUST NOT poterli eliminare né rinominare.

#### Scenario: Catalogo di sistema protetto
- **WHEN** l'utente prova a eliminare o rinominare il catalogo "Cestino"
- **THEN** l'operazione non è disponibile e il catalogo resta

### Requirement: Foto nascoste dai cataloghi di sistema
Le foto di un catalogo di sistema MUST NOT comparire in Libreria, negli altri cataloghi, nei filtri, nei conteggi e in "Seleziona tutto", salvo aprendo quel catalogo di sistema. I file sul disco MUST NOT essere modificati.

#### Scenario: Foto nascoste dalla vista principale
- **WHEN** una foto è nel catalogo "Duplicati"
- **THEN** non compare in "Tutte le foto", nei conteggi dei filtri né nei cataloghi dell'utente, e compare aprendo "Duplicati"

### Requirement: Ripristino da un catalogo di sistema
Dal catalogo di sistema aperto, "Ripristina" SHALL riportare le foto selezionate nella libreria, con tag, cataloghi e persone, e SHALL potersi annullare.

#### Scenario: Ripristino
- **WHEN** l'utente apre "Cestino", seleziona 2 foto e sceglie "Ripristina"
- **THEN** le 2 foto tornano in "Tutte le foto" con i loro tag, cataloghi e persone, e l'operazione si può annullare

### Requirement: Cestino
L'utente SHALL poter spostare nel Cestino le foto selezionate, senza dialogo di conferma, con un avviso con "Annulla operazione". Le foto nel Cestino conservano tag, cataloghi e persone.

#### Scenario: Eliminazione logica
- **WHEN** l'utente seleziona 3 foto e sceglie "Sposta nel cestino"
- **THEN** le 3 foto spariscono dalla Libreria, compaiono nel catalogo "Cestino" e i file restano sul disco

### Requirement: Duplicati messi da parte
L'utente SHALL poter mettere da parte i duplicati: per ogni gruppo di file della libreria con lo stesso contenuto resta in Libreria la copia importata per prima, e le altre vanno nel catalogo "Duplicati". L'operazione SHALL indicare quante copie ha messo da parte e SHALL potersi annullare.

#### Scenario: Copie identiche
- **WHEN** la libreria contiene 3 copie identiche dello stesso file e l'utente sceglie "Metti da parte i duplicati"
- **THEN** la prima copia importata resta in Libreria e le altre 2 vanno in "Duplicati", con l'avviso "2 duplicati messi da parte"

### Requirement: Foto di qualità insufficiente
L'utente SHALL poter avviare il controllo della qualità, come attività con avanzamento: per ogni foto senza misura, il sistema misura la nitidezza sulla miniatura e legge la risoluzione, e sposta nel catalogo "Qualità insufficiente" le foto molto sfocate o con meno di 0,3 megapixel. Le misure SHALL essere salvate, così un nuovo controllo esamina solo le foto nuove. Le foto ripristinate dall'utente MUST NOT tornare in "Qualità insufficiente" a un controllo successivo.

#### Scenario: Foto sfocata
- **WHEN** l'utente avvia "Controlla la qualità" e una foto è molto sfocata
- **THEN** al termine la foto è nel catalogo "Qualità insufficiente" e non compare più in Libreria

#### Scenario: Scelta dell'utente rispettata
- **WHEN** l'utente ripristina una foto da "Qualità insufficiente" e poi avvia di nuovo il controllo
- **THEN** la foto resta in Libreria
