# resource-governor Specification

## Purpose
Misura le risorse del sistema (RAM e CPU) e stabilisce un budget di memoria che regola concorrenza, dimensione dei batch e modelli caricati, così che l'app resti stabile e non degradi il computer dell'utente.

## Requirements

### Requirement: Rilevamento risorse all'avvio
All'avvio il sistema SHALL rilevare RAM totale, RAM disponibile, numero di core logici e fisici e acceleratori disponibili, e mostrarli nella schermata di sistema e in diagnostica.

#### Scenario: Verifica sistema
- **WHEN** l'app si avvia su una macchina con 16 GB di RAM e 8 core
- **THEN** la schermata di sistema mostra 16 GB totali, la RAM disponibile corrente e 8 core

### Requirement: Budget di memoria
Il sistema SHALL calcolare un budget di memoria per l'app in base alla RAM totale e disponibile e al profilo scelto (Leggero, Bilanciato, Prestazioni), e l'utente SHALL poter impostare un limite massimo manuale.

#### Scenario: Profilo predefinito
- **WHEN** l'utente non ha scelto un profilo
- **THEN** si usa "Bilanciato" e il budget non supera una frazione prudente della RAM disponibile all'avvio

#### Scenario: Limite manuale
- **WHEN** l'utente imposta un limite di 4 GB
- **THEN** il budget dell'app non supera 4 GB qualunque sia il profilo

### Requirement: Requisiti minimi
Se la RAM disponibile non basta per caricare il modello più piccolo richiesto, il sistema SHALL disattivare le analisi AI interessate con un avviso chiaro, mantenendo funzionanti import, metadati, libreria ed export.

#### Scenario: Poca RAM
- **WHEN** la RAM disponibile è inferiore al minimo richiesto dal riconoscimento volti
- **THEN** il riconoscimento volti non viene avviato, l'utente vede il motivo e le altre fasi proseguono

### Requirement: Ammissione dei lavori in base al budget
Ogni fase che consuma memoria (decodifica immagini, miniature, inferenza) SHALL dichiarare un costo stimato e partire solo se rientra nel budget residuo; la concorrenza e la dimensione dei batch SHALL derivare dal budget.

#### Scenario: Budget ridotto
- **WHEN** il budget è di 2 GB
- **THEN** il numero di immagini decodificate contemporaneamente e la dimensione dei batch di inferenza sono ridotti in modo che il consumo stimato resti sotto 2 GB

### Requirement: Residenza dei modelli
Il sistema SHALL caricare in memoria solo i modelli che rientrano nel budget, scaricando quelli inattivi quando serve spazio; se non entrano tutti insieme le fasi AI SHALL essere eseguite in sequenza anziché in parallelo.

#### Scenario: Modelli non contemporanei
- **WHEN** i modelli di classificazione e dei volti insieme superano il budget
- **THEN** il sistema esegue prima tutta la classificazione, scarica quel modello e poi esegue i volti

### Requirement: Adattamento alla pressione di memoria
Durante l'esecuzione il sistema SHALL monitorare periodicamente la RAM disponibile del sistema e il consumo dell'app; sotto una soglia di pressione SHALL ridurre la concorrenza e, se persiste, mettere in pausa le fasi AI fino al rientro.

#### Scenario: Altra applicazione occupa RAM
- **WHEN** durante un'analisi la RAM disponibile del sistema scende sotto la soglia di pressione
- **THEN** il sistema riduce i worker, registra l'evento e riprende il ritmo normale quando la RAM torna disponibile

### Requirement: Visibilità del consumo
L'interfaccia SHALL mostrare in modo continuo la RAM usata dall'app rispetto al budget e alla RAM del sistema, e il dettaglio dei modelli residenti.

#### Scenario: Indicatore RAM
- **WHEN** è in corso un'analisi
- **THEN** la barra superiore mostra la RAM usata dall'app e il budget, aggiornati almeno ogni 2 secondi
