# raw-review-export Specification

## Purpose
Trasforma il risultato di una revisione RAW in foto della libreria. Copia le varianti scelte in una cartella, con filtri e rinomina, e crea un nuovo catalogo con il nome indicato dall'utente.

## Requirements

### Requirement: Selezione degli scatti da esportare
L'export SHALL includere solo gli scatti con una variante scelta e SHALL filtrarli per valutazione minima (da 0, cioè tutte le scelte, a 5) e, se l'utente lo chiede, per preset della variante scelta. Il dialogo SHALL mostrare il numero di scatti per ciascuna valutazione e il totale che verrà esportato, aggiornati mentre cambiano i filtri.

#### Scenario: Valutazione minima
- **WHEN** la sessione ha 40 scelte, di cui 12 con almeno 4 stelle, e l'utente imposta "almeno ★★★★"
- **THEN** il dialogo indica 12 file da esportare

#### Scenario: Scatti senza scelta
- **WHEN** 20 scatti hanno una valutazione ma nessuna variante scelta
- **THEN** quei 20 scatti non vengono esportati e il dialogo indica quanti sono stati esclusi per questo

### Requirement: Solo JPEG nella destinazione
L'export SHALL copiare nella destinazione solo file JPEG (`.jpg`, `.jpeg`). I file RAF e qualsiasi altro formato MUST NOT essere mai copiati, senza opzioni per cambiarlo. Uno scatto la cui variante scelta non è un JPEG, per esempio un TIFF del layout per preset, SHALL essere escluso, e il dialogo SHALL indicare quanti scatti sono esclusi per questo motivo.

#### Scenario: Cartella di RAF sviluppati
- **WHEN** l'utente esporta 12 scatti di una cartella di RAF con le varianti generate
- **THEN** la destinazione contiene solo i 12 JPEG scelti e nessun file `.RAF`

#### Scenario: Variante TIFF scelta
- **WHEN** in una cartella per preset 3 scatti hanno come scelta una variante TIFF
- **THEN** quei 3 scatti non vengono esportati e il dialogo indica "3 esclusi: non JPEG"

### Requirement: Copia fedele nella destinazione
L'export SHALL copiare i file scelti nella cartella di destinazione byte per byte, senza ricodifica e con i metadati originali. Ogni file SHALL essere scritto in un file temporaneo e rinominato a copia completata. Un file già presente con lo stesso nome SHALL essere saltato, salvo che l'utente abbia scelto di sovrascrivere. I file sorgente MUST NOT essere modificati.

#### Scenario: Copia senza ricodifica
- **WHEN** l'utente esporta 12 JPEG scelti
- **THEN** i 12 file nella destinazione hanno lo stesso hash delle varianti scelte

#### Scenario: Esportazione ripetuta
- **WHEN** l'utente ripete l'export nella stessa destinazione senza "Sovrascrivi"
- **THEN** i file già presenti vengono saltati e il riepilogo li conta come saltati

#### Scenario: Interruzione
- **WHEN** l'app viene chiusa durante l'export
- **THEN** nella destinazione non restano file parziali con il nome finale

### Requirement: Rinomina dei file esportati
L'utente SHALL poter mantenere i nomi originali oppure rinominare i file con il modello di nomi dell'export della Sequenza, con i token ricavabili dai file (`{seq}`, `{seq:0Nd}`, `{date}`, `{time}`, `{time_full}`, `{datetime}`, `{device}`, `{orig}`) più `{preset}`. Il predefinito SHALL essere `<nome del catalogo>_{seq:04d}` in ordine di scatto (data originale, poi nome), con anteprima dei primi nomi, estensione conservata e suffisso deterministico per i nomi uguali.

#### Scenario: Numerazione per data di scatto
- **WHEN** il nome del catalogo è "Dolomiti" e tre scatti hanno ora 10:02, 09:15 e 11:40
- **THEN** i file si chiamano `Dolomiti_0001.jpg` (09:15), `Dolomiti_0002.jpg` (10:02) e `Dolomiti_0003.jpg` (11:40)

#### Scenario: Modello non valido
- **WHEN** l'utente scrive un modello con un token sconosciuto o caratteri non validi nei nomi file
- **THEN** il dialogo lo segnala e l'export non può partire

### Requirement: Nuovo catalogo dall'export
Al termine della copia il sistema SHALL importare in libreria i file esportati e creare un catalogo manuale con il nome indicato dall'utente che contiene esattamente quei file. Se la destinazione non è dentro una sorgente della libreria SHALL aggiungerla come nuova sorgente, altrimenti SHALL aggiornare solo quella cartella. Se il nome del catalogo esiste già, l'utente SHALL scegliere tra aggiungere le foto al catalogo esistente e indicare un altro nome.

#### Scenario: Catalogo creato
- **WHEN** l'utente esporta 12 file in `D:\Foto\2026\Dolomiti` con il nome catalogo "Dolomiti 2026"
- **THEN** la cartella diventa una sorgente della libreria, compare il catalogo manuale "Dolomiti 2026" con le 12 foto e un avviso offre "Apri il catalogo"

#### Scenario: Destinazione già nella libreria
- **WHEN** la destinazione è `D:\Foto\2026\Dolomiti` e `D:\Foto` è già una sorgente
- **THEN** non viene creata una nuova sorgente, viene aggiornata solo `Dolomiti` e il catalogo contiene le foto esportate

#### Scenario: Nome già usato
- **WHEN** esiste già il catalogo "Dolomiti 2026" e l'utente avvia l'export con lo stesso nome
- **THEN** prima di copiare il dialogo chiede se aggiungere le foto al catalogo esistente o cambiare nome

#### Scenario: File saltati
- **WHEN** 3 dei 12 file erano già nella destinazione e sono stati saltati
- **THEN** anche quei 3 file, se presenti in libreria dopo l'aggiornamento, entrano nel catalogo

### Requirement: Valutazione e preset trasferiti in libreria
Le foto del nuovo catalogo SHALL ricevere la valutazione data in revisione. Se l'opzione "Aggiungi il preset come tag" è attiva (predefinita), SHALL ricevere come tag del set "Preset Fujifilm" il nome del preset della variante scelta, creando il set e il tag se mancano. Una foto già in libreria con una valutazione diversa SHALL ricevere quella della revisione.

#### Scenario: Valutazioni in libreria
- **WHEN** l'export porta in libreria uno scatto con 5 stelle e preset "C1 Kodachrome 64"
- **THEN** in Libreria la foto ha 5 stelle e il tag "C1 Kodachrome 64" del set "Preset Fujifilm"

#### Scenario: Opzione disattivata
- **WHEN** l'utente disattiva "Aggiungi il preset come tag"
- **THEN** le foto ricevono la valutazione ma nessun tag di preset

### Requirement: Export come attività
Copia, import e creazione del catalogo SHALL essere eseguiti come un'unica attività con avanzamento e annullamento. Il riepilogo finale SHALL indicare i file esportati, saltati e con errori, con il motivo di ciascun errore. Un file con errore MUST NOT fermare gli altri. Se l'utente annulla dopo la copia, i file già copiati SHALL restare nella destinazione e il catalogo MUST NOT essere creato.

#### Scenario: Errore su un file
- **WHEN** uno dei 12 file sorgente non è più leggibile
- **THEN** gli altri 11 vengono esportati e messi nel catalogo, e il riepilogo elenca il file con il motivo

#### Scenario: Annullamento
- **WHEN** l'utente annulla durante l'import dei file copiati
- **THEN** l'attività si ferma, i file copiati restano nella destinazione e il catalogo non viene creato
