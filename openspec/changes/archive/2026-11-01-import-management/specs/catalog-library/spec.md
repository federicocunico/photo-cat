# Spec Delta

## MODIFIED Requirements

### Requirement: Stato dei file sul disco
Il sistema SHALL segnalare i file della libreria non più presenti sul disco come "mancanti" senza rimuoverli automaticamente, e permettere all'utente di ricollegarli o rimuoverli. Il ricollegamento SHALL poter indicare la nuova posizione di una cartella e ritrovare i file per percorso relativo o per impronta del contenuto; la rimozione SHALL poter riguardare tutti i mancanti di una sorgente in un'unica azione confermata.

#### Scenario: File spostato fuori dal programma
- **WHEN** un file catalogato viene cancellato o spostato esternamente
- **THEN** al successivo controllo il file è marcato "mancante" e mantiene le sue catalogazioni

#### Scenario: Cartella spostata su un altro disco
- **WHEN** l'utente ha spostato `D:\Foto\Viaggio` in `E:\Archivio\Viaggio`, la libreria segna 400 file mancanti e l'utente sceglie "Ricollega…" indicando la nuova cartella
- **THEN** i 400 file tornano disponibili con le loro catalogazioni, i tag, i cataloghi e le persone, e il riepilogo indica quanti file sono stati ritrovati e quanti restano mancanti

#### Scenario: File rinominati nella nuova cartella
- **WHEN** nella cartella indicata per il ricollegamento alcuni file hanno un nome diverso ma lo stesso contenuto
- **THEN** quei file vengono ricollegati tramite impronta del contenuto

#### Scenario: Rimozione dei mancanti
- **WHEN** una sorgente ha 5 file mancanti e l'utente sceglie "Rimuovi mancanti…"
- **THEN** dopo una conferma che indica 5 file, quei file non sono più in libreria e gli altri file della sorgente restano invariati
