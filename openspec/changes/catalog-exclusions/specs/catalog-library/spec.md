# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
- **THEN** nessuna foto viene esclusa né rimossa

#### Scenario: Conteggi
- **WHEN** un catalogo ha 34 foto visibili e 6 escluse
- **THEN** la barra laterale mostra 34 per quel catalogo e l'intestazione del catalogo indica anche "6 escluse"

#### Scenario: Appartenenza in una regola smart
- **WHEN** una regola smart richiede "Catalogo = Viaggio Toscana" e una foto è esclusa da "Viaggio Toscana"
- **THEN** quella foto non soddisfa la condizione
