# Spec Delta

## ADDED Requirements

### Requirement: Data di scatto impostata a mano
L'utente SHALL poter impostare a mano la data di scatto di una o più foto dall'Inspector o in bulk, con precisione giorno, mese oppure solo anno, e ripristinare la data letta dal file. La data manuale SHALL avere fonte "manuale", prevalere su quella dei metadati in ordinamento, filtri per data, Sequenza, cataloghi smart ed export, ed essere annullabile. I file originali MUST NOT essere modificati.

#### Scenario: Negativo scansionato
- **WHEN** una scansione ha data EXIF 2019-03-02 e l'utente imposta "giugno 1984"
- **THEN** l'Inspector mostra "giugno 1984 (impostata a mano)" e la foto compare nel filtro per data sotto 1984 › giugno

#### Scenario: Solo anno
- **WHEN** l'utente imposta l'anno 1962 su 40 stampe scansionate
- **THEN** le 40 foto compaiono sotto l'anno 1962 nel filtro per data, senza mese, e l'Inspector mostra "1962 (impostata a mano)"

#### Scenario: Ordinamento con precisione ridotta
- **WHEN** in un catalogo ci sono una foto con data manuale "1984" e una con data EXIF 1984-06-10 14:00
- **THEN** la foto con solo anno è ordinata al 1° gennaio 1984 alle 00:00, quindi prima dell'altra

#### Scenario: Ripristino
- **WHEN** l'utente sceglie "Ripristina data del file" su una foto con data manuale
- **THEN** la foto torna alla data e alla fonte lette dai metadati o dal file system

#### Scenario: Esclusa dalle foto senza data
- **WHEN** una foto con fonte "file system" riceve una data manuale
- **THEN** non compare più nell'area delle foto senza data della Sequenza

#### Scenario: Correzioni di gruppo
- **WHEN** una foto ha una data manuale e l'utente applica uno scostamento al suo gruppo di sorgente
- **THEN** la data manuale della foto non cambia
