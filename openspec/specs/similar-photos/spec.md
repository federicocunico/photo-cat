# similar-photos Specification

## Purpose
Raggruppa le foto quasi identiche (raffiche, scatti ripetuti, piccole variazioni della stessa inquadratura) per rendere la libreria più consultabile e aiutare l'utente a scegliere le foto migliori, senza mai cancellare nulla dal disco.

## Requirements

### Requirement: Rilevamento dei gruppi di foto simili
Il sistema SHALL raggruppare in locale le foto quasi identiche, considerando candidate le coppie con contenuto semantico molto vicino e confermandole con una misura di somiglianza strutturale (SSIM) sulle miniature, sopra una soglia configurabile. Una foto SHALL appartenere al massimo a un gruppo; i duplicati esatti SHALL far parte dello stesso gruppo.

#### Scenario: Raffica
- **WHEN** l'utente importa una raffica di 8 scatti dello stesso soggetto fatti in 3 secondi
- **THEN** al termine dell'analisi le 8 foto formano un unico gruppo

#### Scenario: Stesso luogo, inquadratura diversa
- **WHEN** due foto ritraggono lo stesso monumento da punti di vista diversi
- **THEN** le due foto non vengono raggruppate

#### Scenario: Foto senza data
- **WHEN** due copie quasi identiche di una foto non hanno data di scatto
- **THEN** vengono comunque raggruppate in base al contenuto

#### Scenario: Soglia più severa
- **WHEN** l'utente aumenta la soglia di somiglianza dei gruppi e salva
- **THEN** i gruppi vengono ricalcolati senza rianalizzare le immagini con i modelli, e le foto meno simili escono dai gruppi

### Requirement: Foto principale
Per ogni gruppo il sistema SHALL indicare una foto principale, proponendo quella con nitidezza ed esposizione migliori; l'utente SHALL poter scegliere un'altra foto principale, e la sua scelta MUST prevalere sulle analisi successive.

#### Scenario: Principale proposta
- **WHEN** un gruppo contiene una foto mossa e una nitida dello stesso soggetto
- **THEN** la foto nitida è indicata come principale

#### Scenario: Scelta dell'utente
- **WHEN** l'utente imposta come principale un'altra foto del gruppo e poi l'analisi viene ripetuta
- **THEN** la principale resta quella scelta dall'utente

### Requirement: Correzioni dei gruppi
L'utente SHALL poter togliere foto da un gruppo, unire foto selezionate in un gruppo e sciogliere un gruppo; le correzioni MUST essere rispettate dalle analisi successive.

#### Scenario: Foto tolta da un gruppo
- **WHEN** l'utente toglie una foto da un gruppo e poi la libreria viene rianalizzata
- **THEN** quella foto non torna nel gruppo

#### Scenario: Unione manuale
- **WHEN** l'utente seleziona 3 foto non raggruppate e sceglie "Raggruppa come simili"
- **THEN** le 3 foto formano un gruppo che resta tale anche dopo una nuova analisi

#### Scenario: Gruppo sciolto
- **WHEN** l'utente scioglie un gruppo di 5 foto
- **THEN** le 5 foto tornano singole e non vengono raggruppate di nuovo tra loro automaticamente

### Requirement: Visualizzazione raggruppata
Con l'opzione "Raggruppa foto simili" attiva, la griglia di Libreria, cataloghi e Sequenza SHALL mostrare ogni gruppo come una sola miniatura (la principale) con il numero di foto del gruppo, espandibile sul posto. Conteggi, filtri e "Seleziona tutto" SHALL continuare a riferirsi alle singole foto. Selezionare una miniatura di gruppo chiusa SHALL selezionare tutte le foto del gruppo.

#### Scenario: Gruppo chiuso
- **WHEN** l'opzione è attiva e un gruppo di 8 foto rientra nei risultati
- **THEN** la griglia mostra una sola miniatura con l'indicazione "8" e il conteggio dei risultati include le 8 foto

#### Scenario: Espansione
- **WHEN** l'utente espande il gruppo
- **THEN** le 8 foto compaiono una accanto all'altra, con la principale evidenziata

#### Scenario: Gruppo parzialmente nei risultati
- **WHEN** un filtro attivo include solo 3 delle 8 foto di un gruppo
- **THEN** la miniatura del gruppo indica "3" e mostra la principale se è tra le 3, altrimenti la migliore delle 3

### Requirement: Aiuto alla scelta
Il sistema SHALL offrire il filtro "Solo foto con simili" e l'azione "Seleziona tutte tranne la principale" su una selezione o sui risultati correnti, da combinare con le azioni bulk esistenti. Nessuna funzione dei gruppi di foto simili MUST cancellare file dal disco.

#### Scenario: Pulizia di un catalogo
- **WHEN** l'utente filtra "Solo foto con simili" nel catalogo "Vacanze", sceglie "Seleziona tutte tranne la principale" e poi "Escludi dal catalogo"
- **THEN** nel catalogo resta solo la foto principale di ogni gruppo e i file sul disco sono intatti

#### Scenario: Inspector
- **WHEN** l'utente seleziona una foto che appartiene a un gruppo
- **THEN** l'Inspector mostra le altre foto del gruppo e indica se la foto è la principale

### Requirement: Analisi incrementale dei gruppi
L'analisi dei gruppi SHALL essere incrementale e riprendibile: le foto nuove o modificate vengono confrontate solo con le candidate vicine, senza ricalcolare l'intera libreria. Con 100.000 foto l'analisi SHALL avere memoria limitata dal governor delle risorse.

#### Scenario: Nuovo import
- **WHEN** vengono importate 200 foto in una libreria di 100.000 già raggruppata
- **THEN** vengono calcolati solo i confronti che coinvolgono le 200 foto e i gruppi esistenti non toccati restano invariati

#### Scenario: Interruzione
- **WHEN** l'app viene chiusa durante l'analisi dei gruppi
- **THEN** alla ripresa vengono elaborate solo le foto non ancora completate
