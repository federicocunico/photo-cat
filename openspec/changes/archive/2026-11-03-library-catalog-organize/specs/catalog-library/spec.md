# Spec Delta

## MODIFIED Requirements

### Requirement: Cestino
L'utente SHALL poter spostare nel Cestino le foto selezionate, con un avviso con "Annulla operazione". Spostare nel Cestino SHALL togliere le foto da tutti i cataloghi manuali dell'utente. Se almeno una foto appartiene a un catalogo manuale, il sistema SHALL chiedere conferma prima, elencando i cataloghi interessati e quante foto perde ciascuno; se nessuna foto appartiene a un catalogo manuale, MUST NOT comparire alcun dialogo. Le foto nel Cestino conservano tag e persone. "Annulla operazione" SHALL riportare le foto anche nei cataloghi da cui erano state tolte. Nel dialogo di conferma il focus iniziale SHALL essere sul pulsante di conferma, così Invio conferma.

#### Scenario: Eliminazione logica
- **WHEN** l'utente seleziona 3 foto che non appartengono a nessun catalogo manuale e sceglie "Sposta nel cestino"
- **THEN** senza dialogo le 3 foto spariscono dalla Libreria, compaiono nel catalogo "Cestino" e i file restano sul disco

#### Scenario: Conferma per le foto nei cataloghi
- **WHEN** l'utente seleziona 5 foto, 3 delle quali sono nel catalogo "Mare" e 1 in "Montagna", e sceglie "Sposta nel cestino"
- **THEN** un dialogo indica che "Mare" perderà 3 foto e "Montagna" 1 foto; confermando, le 5 foto vanno nel Cestino e "Mare" e "Montagna" non le contengono più

#### Scenario: Conferma con Invio
- **WHEN** compare il dialogo di conferma dello spostamento nel Cestino e l'utente preme Invio
- **THEN** le foto vanno nel Cestino come scegliendo il pulsante di conferma

#### Scenario: Annullamento della conferma
- **WHEN** compare il dialogo di conferma e l'utente sceglie "Annulla"
- **THEN** nessuna foto viene spostata e i cataloghi restano invariati

#### Scenario: Annullare lo spostamento nel Cestino
- **WHEN** dopo aver confermato lo spostamento delle 5 foto l'utente sceglie "Annulla operazione"
- **THEN** le 5 foto tornano in Libreria, "Mare" contiene di nuovo le sue 3 foto e "Montagna" la sua

#### Scenario: Cataloghi smart
- **WHEN** l'utente sposta nel Cestino una foto che soddisfa le regole del catalogo smart "Ritratti"
- **THEN** il dialogo non elenca "Ritratti" e la foto non compare più in "Ritratti" finché resta nel Cestino

### Requirement: Ripristino da un catalogo di sistema
Dal catalogo di sistema aperto, "Ripristina" SHALL riportare le foto selezionate nella libreria, con tag, cataloghi e persone, e SHALL potersi annullare. Per il Cestino le foto tornano con tag e persone ma senza i cataloghi manuali da cui erano state tolte entrando nel Cestino.

#### Scenario: Ripristino
- **WHEN** l'utente apre "Duplicati", seleziona 2 foto e sceglie "Ripristina"
- **THEN** le 2 foto tornano in "Tutte le foto" con i loro tag, cataloghi e persone, e l'operazione si può annullare

#### Scenario: Ripristino dal Cestino
- **WHEN** una foto era in "Mare" prima di essere spostata nel Cestino, e l'utente la ripristina dal Cestino
- **THEN** la foto torna in "Tutte le foto" con tag e persone, ma non in "Mare"

### Requirement: Albero delle cartelle
Il sistema SHALL mostrare, sotto ogni sorgente, l'albero delle cartelle che contengono file della libreria con il numero di file per cartella (incluse le sottocartelle); l'albero SHALL caricarsi un livello alla volta per restare reattivo su librerie grandi. Nella barra laterale della Libreria la sezione delle cartelle SHALL stare dopo i cataloghi e i filtri, chiusa per impostazione predefinita, e SHALL offrire il comando per gestire le cartelle nel modulo Importa.

#### Scenario: Espansione di una sorgente
- **WHEN** l'utente espande la sorgente `D:\Foto` che contiene `Viaggio Giappone` con 3 sottocartelle
- **THEN** compare `Viaggio Giappone` con il conteggio totale e, espandendola, le 3 sottocartelle con i rispettivi conteggi

#### Scenario: Seleziona tutto in una cartella
- **WHEN** l'utente sceglie una cartella nell'albero e poi "Seleziona tutto"
- **THEN** risultano selezionati tutti i file di quella cartella, pronti per un'azione bulk

#### Scenario: Primo avvio della Libreria
- **WHEN** l'utente apre la Libreria per la prima volta
- **THEN** la barra laterale mostra in alto i cataloghi, poi i filtri, e in fondo la sezione delle cartelle chiusa; aprendola compaiono le sorgenti e "Gestisci cartelle", che apre il modulo Importa

## ADDED Requirements

### Requirement: Aggiunta a un catalogo per trascinamento
L'utente SHALL poter trascinare foto dalla griglia su un catalogo manuale della barra laterale per aggiungervele, con un avviso con "Annulla operazione". Trascinando una foto selezionata si spostano tutte le foto selezionate; trascinando una foto non selezionata si sposta solo quella, senza cambiare la selezione. Durante il trascinamento il sistema SHALL mostrare quante foto si spostano ed evidenziare il catalogo sotto il puntatore.

#### Scenario: Trascinamento della selezione
- **WHEN** l'utente seleziona 12 foto e ne trascina una sul catalogo manuale "Mare"
- **THEN** durante il trascinamento compare "12 foto", "Mare" si evidenzia e, al rilascio, "Mare" contiene le 12 foto e compare "12 foto aggiunte a «Mare»" con "Annulla operazione"

#### Scenario: Trascinamento di una foto non selezionata
- **WHEN** sono selezionate 3 foto e l'utente trascina una quarta foto non selezionata su "Mare"
- **THEN** solo la quarta foto viene aggiunta a "Mare" e la selezione resta di 3 foto

#### Scenario: Selezione per filtro
- **WHEN** l'utente sceglie "Seleziona tutto" su una cartella con 2.340 foto e trascina una foto selezionata su "Mare"
- **THEN** le 2.340 foto vengono aggiunte a "Mare"

#### Scenario: Catalogo che non accetta foto
- **WHEN** l'utente trascina foto sopra un catalogo smart o sopra "Duplicati"
- **THEN** il catalogo non si evidenzia come destinazione, il cursore indica che il rilascio non è possibile e al rilascio non cambia nulla

#### Scenario: Foto già nel catalogo
- **WHEN** l'utente trascina su "Mare" 5 foto, 2 delle quali sono già in "Mare"
- **THEN** "Mare" contiene le 5 foto una volta sola

#### Scenario: Rilascio fuori dai cataloghi
- **WHEN** l'utente rilascia le foto trascinate fuori da un catalogo, oppure preme Esc durante il trascinamento
- **THEN** il trascinamento termina senza modifiche e la selezione resta invariata

#### Scenario: Clic senza trascinamento
- **WHEN** l'utente preme e rilascia il pulsante su una miniatura muovendo il puntatore di meno di 5 px
- **THEN** il gesto vale come clic e la selezione si comporta come un clic

### Requirement: Nuovo catalogo dalle foto trascinate
La sezione dei cataloghi della barra laterale SHALL offrire "Nuovo catalogo", che crea un catalogo manuale con un nome scritto sul posto; rilasciando foto trascinate su "Nuovo catalogo" il sistema SHALL chiedere il nome e creare il catalogo con quelle foto.

#### Scenario: Catalogo creato dal trascinamento
- **WHEN** l'utente trascina 4 foto su "Nuovo catalogo", scrive "Compleanno" e preme Invio
- **THEN** compare il catalogo manuale "Compleanno" con le 4 foto

#### Scenario: Nome già usato
- **WHEN** l'utente scrive come nome di un nuovo catalogo il nome di un catalogo esistente
- **THEN** il sistema lo indica accanto al campo e non crea un secondo catalogo con lo stesso nome

#### Scenario: Creazione annullata
- **WHEN** l'utente rilascia foto su "Nuovo catalogo" e poi preme Esc nel campo del nome
- **THEN** nessun catalogo viene creato

### Requirement: Spostamento nel Cestino per trascinamento
Rilasciare foto trascinate sul catalogo "Cestino" della barra laterale SHALL equivalere a "Sposta nel cestino" su quelle foto, compresa la conferma per le foto nei cataloghi manuali.

#### Scenario: Trascinamento sul Cestino
- **WHEN** l'utente trascina 2 foto che non sono in nessun catalogo manuale sul Cestino
- **THEN** le 2 foto vanno nel Cestino con un avviso con "Annulla operazione"
