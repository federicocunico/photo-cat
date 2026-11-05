# Spec Delta

## MODIFIED Requirements

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

## ADDED Requirements

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
