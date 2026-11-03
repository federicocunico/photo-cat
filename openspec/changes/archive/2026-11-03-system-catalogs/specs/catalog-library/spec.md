# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

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
