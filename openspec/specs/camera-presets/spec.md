# camera-presets Specification

## Purpose
Rende disponibili come preset di sviluppo i banchi di impostazioni personalizzate (C1–C7) salvati nella fotocamera, così che ogni variante rifletta esattamente una ricetta della camera.

## Requirements

### Requirement: Lettura dei banchi custom
Il sistema SHALL leggere dalla camera collegata tutti i banchi custom disponibili, con numero, nome e parametri d'immagine (simulazione pellicola, gamma dinamica, grana, color chrome, bilanciamento del bianco e scostamenti, toni, colore, nitidezza, chiarezza, riduzione rumore).

#### Scenario: Elenco dei preset
- **WHEN** la X-T5 è pronta per la conversione e l'utente apre "Sviluppa dalla fotocamera"
- **THEN** il sistema mostra i 7 banchi con il loro nome (es. "C1 Kodachrome 64") e un riepilogo della simulazione pellicola

#### Scenario: Stato della camera invariato
- **WHEN** il sistema ha finito di leggere i banchi
- **THEN** il banco selezionato sulla camera è lo stesso di prima della lettura e nessun banco è stato modificato

### Requirement: Selezione dei preset
L'utente SHALL poter scegliere quali banchi usare per uno sviluppo; la prima volta sono selezionati tutti i banchi, poi SHALL essere ricordata l'ultima scelta. Il sistema SHALL offrire anche la variante "Come scattato", che usa le impostazioni registrate nel RAF.

#### Scenario: Sottoinsieme di preset
- **WHEN** l'utente deseleziona C6 e C7
- **THEN** lo sviluppo genera solo le varianti per C1–C5 (più "Come scattato" se selezionata)

#### Scenario: Scelta ricordata
- **WHEN** l'utente ha usato C1, C3 e "Come scattato" e apre di nuovo lo sviluppo
- **THEN** sono preselezionati C1, C3 e "Come scattato"

### Requirement: Nome della cartella di un preset
Ogni preset SHALL corrispondere a una cartella di output con nome stabile e valido su tutti i sistemi supportati, nel formato `C<n> <nome banco>`. I caratteri non ammessi nei nomi file MUST essere sostituiti e un banco senza nome MUST usare solo `C<n>`.

#### Scenario: Nome con caratteri non validi
- **WHEN** il banco C3 si chiama `Neg/Hi: 2`
- **THEN** la cartella si chiama `C3 Neg-Hi- 2`

#### Scenario: Banco rinominato fra due sviluppi
- **WHEN** l'utente rinomina un banco sulla camera dopo uno sviluppo e ne avvia un altro sulla stessa cartella
- **THEN** il sistema crea una nuova cartella col nuovo nome e non mescola file prodotti con impostazioni diverse

### Requirement: Fedeltà del preset applicato
La variante prodotta per un preset SHALL usare i parametri del banco per tutte le impostazioni d'immagine supportate dalla conversione in camera e, per il resto, le impostazioni registrate nel RAF stesso. Le impostazioni del banco non applicabili alla conversione MUST essere elencate all'utente.

#### Scenario: Confronto con X RAW Studio
- **WHEN** lo stesso RAF viene sviluppato con il banco C1 da photo-cat e da X RAW Studio con il preset C1 della camera
- **THEN** i due JPEG hanno la stessa simulazione pellicola e gli stessi parametri d'immagine nei metadati Fujifilm (MakerNotes)
