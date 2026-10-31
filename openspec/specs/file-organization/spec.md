# file-organization Specification

## Purpose
Materializza su disco cartelle organizzate secondo le catalogazioni di interesse (es. Luogo > Anno, Persona, Tipo soggetto) usando collegamenti simbolici, hard link o copie, senza mai alterare gli originali.

## Requirements

### Requirement: Struttura di cartelle da catalogazioni
L'utente SHALL poter scegliere un ambito (libreria, catalogo, selezione), una destinazione e una gerarchia di raggruppamento di uno o più livelli basata sulle catalogazioni (luogo, anno/mese, persona, tipo soggetto, locazione, tag, catalogo).

#### Scenario: Luogo > Anno
- **WHEN** l'utente organizza il catalogo "Viaggi" per Luogo e poi Anno in `E:\Organizzate`
- **THEN** viene creata la struttura `E:\Organizzate\<Luogo>\<Anno>\` contenente i file corrispondenti

#### Scenario: Valore mancante
- **WHEN** un file non ha luogo
- **THEN** il file viene collocato nella cartella "Senza luogo"

### Requirement: Metodo di collegamento
L'utente SHALL poter scegliere tra collegamento simbolico, hard link e copia; se il metodo scelto non è disponibile (permessi, file system, volumi diversi per hard link) il sistema SHALL avvisare prima di eseguire e proporre il metodo alternativo.

#### Scenario: Symlink non consentiti su Windows
- **WHEN** l'utente sceglie symlink su Windows senza Modalità sviluppatore o privilegi
- **THEN** il sistema lo segnala in anteprima e propone hard link (stesso volume) o copia

#### Scenario: Hard link tra volumi diversi
- **WHEN** destinazione e originali sono su volumi diversi e il metodo è hard link
- **THEN** il sistema segnala l'impossibilità e propone symlink o copia

### Requirement: Anteprima prima dell'esecuzione
Prima di creare file il sistema SHALL mostrare un'anteprima dell'albero di cartelle con i conteggi, lo spazio richiesto (per la copia) e gli eventuali conflitti.

#### Scenario: Anteprima
- **WHEN** l'utente sceglie "Anteprima"
- **THEN** vede l'albero di cartelle risultante con il numero di file per cartella senza che nulla sia scritto su disco

### Requirement: Gestione dei conflitti di nome
Se nella stessa cartella di destinazione confluiscono file con lo stesso nome, il sistema SHALL renderli univoci con un suffisso deterministico e MUST NOT sovrascrivere file esistenti nella destinazione.

#### Scenario: Due IMG_0001.JPG
- **WHEN** due file diversi chiamati `IMG_0001.JPG` finiscono nella stessa cartella
- **THEN** vengono creati `IMG_0001.JPG` e `IMG_0001 (2).JPG`

### Requirement: Nomi di cartella validi
I nomi delle cartelle generati dai valori di catalogazione SHALL essere resi validi per il file system di destinazione (caratteri non ammessi sostituiti, nomi riservati evitati, lunghezza limitata).

#### Scenario: Carattere non valido
- **WHEN** un valore di luogo contiene ":" su Windows
- **THEN** la cartella viene creata con il carattere sostituito

### Requirement: Annullamento di un'organizzazione
Ogni organizzazione eseguita SHALL essere registrata con l'elenco degli elementi creati; l'utente SHALL poter annullarla, rimuovendo solo i collegamenti, le copie e le cartelle create da quell'operazione.

#### Scenario: Annulla organizzazione
- **WHEN** l'utente annulla l'ultima organizzazione
- **THEN** vengono rimossi solo gli elementi creati da quell'operazione e gli originali restano intatti

### Requirement: Organizzazione come job
L'organizzazione SHALL essere eseguita come job con progresso e registrazione degli errori per singolo file.

#### Scenario: Errore su un file
- **WHEN** la creazione di un collegamento fallisce per un file
- **THEN** il job prosegue con gli altri file e l'errore è registrato in diagnostica
