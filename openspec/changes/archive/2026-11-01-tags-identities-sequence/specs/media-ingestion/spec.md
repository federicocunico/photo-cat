# Spec Delta

## MODIFIED Requirements

### Requirement: Date di acquisizione e modifica
Il sistema SHALL determinare la data/ora di prima acquisizione (EXIF DateTimeOriginal, poi CreateDate, poi data di creazione del file) e la data/ora di ultima modifica, memorizzando la fonte usata. Insieme alla data di acquisizione SHALL memorizzare il fuso dichiarato nei metadati (OffsetTimeOriginal o equivalente), quando presente, e il dispositivo che ha prodotto il file.

#### Scenario: EXIF assente
- **WHEN** un file non ha date EXIF
- **THEN** la data di acquisizione è presa dal file system e la fonte è indicata come "file system"

#### Scenario: Fuso dichiarato
- **WHEN** un HEIC di iPhone ha DateTimeOriginal 2026-08-19 10:44:12 e OffsetTimeOriginal -06:00
- **THEN** la libreria memorizza la data 2026-08-19 10:44:12 con fuso -06:00 e dispositivo "Apple iPhone 17 Pro Max"

#### Scenario: Libreria esistente
- **WHEN** una libreria importata prima dell'aggiornamento viene aperta
- **THEN** i fusi mancanti vengono letti nuovamente dai file in un job in background, senza ripetere analisi AI né rigenerare miniature
