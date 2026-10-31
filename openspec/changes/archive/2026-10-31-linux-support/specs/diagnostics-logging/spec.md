# Spec Delta

## ADDED Requirements

### Requirement: Apertura della cartella dei log
Dalla diagnostica l'utente SHALL poter aprire la cartella dei log nel file manager predefinito del sistema su Windows, Linux e macOS. Se l'apertura non è possibile, l'utente SHALL vedere un messaggio che spiega cosa non ha funzionato e indica il percorso della cartella.

#### Scenario: Apertura su Linux
- **WHEN** su Linux l'utente sceglie "Apri cartella log"
- **THEN** la cartella dei log si apre nel file manager predefinito del desktop

#### Scenario: Nessun file manager disponibile
- **WHEN** il sistema non ha un programma in grado di aprire cartelle
- **THEN** l'utente vede un messaggio che indica che non è stato possibile aprire la cartella e ne mostra il percorso
