# command-palette

## ADDED Requirements

### Requirement: Palette dei comandi
Ctrl+K, o il campo di ricerca della barra superiore, SHALL aprire una palette che cerca in un solo elenco, raggruppato per tipo, comandi, persone, tag, cataloghi, luoghi e nomi di file. La palette SHALL:
- essere utilizzabile solo da tastiera;
- mostrare la scorciatoia di ogni comando;
- mostrare i comandi recenti quando è vuota;
- mostrare un messaggio esplicito quando non trova nulla.

#### Scenario: Vai a un modulo
- **WHEN** l'utente preme Ctrl+K, scrive "pers" e preme Invio sul comando "Vai a Persone"
- **THEN** si apre il modulo Persone

#### Scenario: Nessun risultato
- **WHEN** il testo non corrisponde a nulla
- **THEN** la palette mostra "Nessun risultato" e propone "Cerca «testo» nei nomi dei file"

### Requirement: Risultati della palette
Scegliere una persona, un tag, un catalogo o un luogo nella palette SHALL aprire la Libreria filtrata. Invio sul testo libero SHALL cercarlo come testo in Libreria. Le azioni sulla selezione SHALL comparire solo quando c'è una selezione.

#### Scenario: Filtro per persona
- **WHEN** l'utente scrive "giulia" e sceglie la persona "Giulia Rossi"
- **THEN** si apre la Libreria filtrata sulle foto di Giulia Rossi, con il relativo chip

#### Scenario: Azione sulla selezione
- **WHEN** sono selezionate 5 foto e l'utente sceglie "Aggiungi tag…" nella palette
- **THEN** si apre il selettore dei tag per le 5 foto

### Requirement: Scorciatoie da tastiera
L'app SHALL offrire scorciatoie globali:

| Scorciatoia | Azione |
|---|---|
| Ctrl+K | palette |
| Ctrl+1…7 | moduli |
| Ctrl+, | Impostazioni |
| Ctrl+Z | annulla operazione |
| Ctrl+B | navigazione |
| Ctrl+I | ispettore |
| Tab | pannelli |

Le scorciatoie MUST NOT attivarsi mentre il focus è in un campo di testo, tranne Ctrl+K ed Esc.

#### Scenario: Digitazione in un campo
- **WHEN** il focus è nel campo del luogo e l'utente digita "2"
- **THEN** viene scritto "2" e non viene assegnato il tag del tasto 2

### Requirement: Foglio delle scorciatoie
Il tasto `?` SHALL aprire un foglio che elenca tutte le scorciatoie dell'app, raggruppate per area, nella lingua dell'interfaccia. Menu e suggerimenti dei comandi SHALL mostrare la scorciatoia associata.

#### Scenario: Foglio delle scorciatoie
- **WHEN** l'utente preme `?` in Libreria
- **THEN** si apre il foglio con le scorciatoie di Generale, Libreria, Visualizzatore, Sequenza e Persone
