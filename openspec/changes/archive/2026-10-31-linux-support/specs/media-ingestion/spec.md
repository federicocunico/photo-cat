# Spec Delta

## ADDED Requirements

### Requirement: Decodifica HEIC su Linux
Su Linux il sistema SHALL decodificare i file HEIC usando la libreria libheif installata nel sistema, quando è presente e include un decoder HEVC; in sua assenza SHALL applicare il comportamento previsto per i formati non decodificabili.

#### Scenario: libheif di sistema presente
- **WHEN** su Linux con libheif e decoder HEVC installati viene scansionato un file HEIC
- **THEN** il file è in libreria con miniatura e analisi AI come gli altri formati supportati

#### Scenario: libheif di sistema assente
- **WHEN** su Linux senza libheif viene scansionato un file HEIC
- **THEN** il file è in libreria con i soli metadati, e l'avviso in diagnostica suggerisce di installare libheif
