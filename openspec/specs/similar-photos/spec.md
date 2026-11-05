# similar-photos Specification

## Purpose
Raggruppa le foto quasi identiche (raffiche, scatti ripetuti, piccole variazioni della stessa inquadratura) per rendere la libreria più consultabile e aiutare l'utente a scegliere le foto migliori, senza mai cancellare nulla dal disco.

## Requirements

### Requirement: Rilevamento dei gruppi di foto simili
Il sistema SHALL raggruppare in locale le foto quasi identiche. SHALL considerare candidate le coppie con impronta percettiva vicina, gli scatti a pochi secondi l'uno dall'altro con impronta compatibile e, quando l'analisi AI è disponibile, le coppie con contenuto semantico molto vicino. Ogni candidata SHALL essere confermata con una misura di somiglianza strutturale (SSIM) sulle miniature, sopra una soglia configurabile. Una foto SHALL appartenere al massimo a un gruppo; i duplicati esatti SHALL far parte dello stesso gruppo. Il rilevamento MUST funzionare anche senza modelli AI installati.

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

#### Scenario: Nessun modello AI installato
- **WHEN** l'utente importa una raffica di 5 scatti senza aver installato i modelli AI
- **THEN** al termine dell'import le 5 foto formano un unico gruppo

#### Scenario: Scatti vicini nel tempo ma diversi
- **WHEN** due foto dello stesso dispositivo sono scattate a 2 secondi di distanza, la seconda dopo aver girato la fotocamera verso un soggetto diverso
- **THEN** le due foto non vengono raggruppate

#### Scenario: Copia ridimensionata in un'altra cartella
- **WHEN** la libreria contiene una foto e una sua copia ridimensionata e ricompressa, in cartelle diverse e senza data
- **THEN** le due foto vengono raggruppate anche se l'analisi AI non è stata eseguita

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
L'analisi dei gruppi SHALL essere incrementale e riprendibile: le foto nuove o modificate vengono confrontate solo con le candidate vicine, senza ricalcolare l'intera libreria. Con 100.000 foto l'analisi SHALL avere memoria limitata dal governor delle risorse. L'analisi dei gruppi SHALL partire automaticamente al termine di ogni import che aggiunge o modifica foto.

#### Scenario: Nuovo import
- **WHEN** vengono importate 200 foto in una libreria di 100.000 già raggruppata
- **THEN** vengono calcolati solo i confronti che coinvolgono le 200 foto e i gruppi esistenti non toccati restano invariati

#### Scenario: Interruzione
- **WHEN** l'app viene chiusa durante l'analisi dei gruppi
- **THEN** alla ripresa vengono elaborate solo le foto non ancora completate

#### Scenario: Avvio dopo l'import
- **WHEN** termina un import che ha aggiunto 50 foto
- **THEN** l'analisi dei gruppi per quelle foto viene accodata senza azioni dell'utente

### Requirement: Impronta percettiva all'import
Il sistema SHALL calcolare per ogni foto un'impronta percettiva di 64 bit durante l'import, dalla miniatura, senza modelli AI e senza ulteriori letture dell'originale. L'impronta SHALL essere stabile rispetto a ridimensionamento, ricompressione e piccole variazioni di esposizione, e SHALL essere ricalcolata quando il contenuto del file cambia.

#### Scenario: Import di una foto
- **WHEN** termina l'import di una foto con miniatura
- **THEN** la foto ha un'impronta percettiva salvata nel catalogo

#### Scenario: Stessa foto ricompressa
- **WHEN** una foto con dettagli (non un'immagine uniforme come un cielo sereno) e una sua copia ridimensionata a metà e salvata con qualità JPEG 70 vengono importate
- **THEN** le due impronte differiscono di al massimo 6 bit

#### Scenario: Foto ritoccata
- **WHEN** il file di una foto viene sostituito con una versione modificata e la cartella viene aggiornata
- **THEN** l'impronta della foto viene ricalcolata dal nuovo contenuto

### Requirement: Impronta per le foto già importate
Il sistema SHALL calcolare l'impronta per le foto importate prima di questa funzione con un'attività in background, avviata una sola volta per libreria, che legge le miniature in cache. L'attività SHALL essere riprendibile e SHALL accodare l'analisi dei gruppi al termine.

#### Scenario: Aggiornamento dell'app
- **WHEN** l'utente apre con la nuova versione una libreria di 20.000 foto senza impronte
- **THEN** compare un'attività che calcola le 20.000 impronte e, al termine, i gruppi vengono ricalcolati

#### Scenario: Interruzione del recupero
- **WHEN** l'app viene chiusa dopo il calcolo di 8.000 impronte su 20.000
- **THEN** alla riapertura l'attività riprende e calcola solo le 12.000 mancanti

### Requirement: Confronto rapido delle impronte
Il confronto tra impronte SHALL usare la distanza di Hamming. Confrontare 200 foto nuove con una libreria di 100.000 foto SHALL richiedere meno di 1 secondo. Le coppie con impronte molto distanti SHALL essere scartate prima della misura SSIM.

#### Scenario: Libreria grande
- **WHEN** vengono importate 200 foto in una libreria di 100.000 foto con impronta
- **THEN** la ricerca delle candidate per impronta termina in meno di 1 secondo

#### Scenario: Candidata semantica con impronta lontana
- **WHEN** due foto hanno contenuto semantico simile ma impronte che differiscono per più di metà dei bit
- **THEN** la coppia viene scartata senza calcolare la SSIM
