# Spec Delta

## MODIFIED Requirements

### Requirement: Moduli di navigazione
L'applicazione SHALL organizzare la navigazione in gruppi con un'etichetta:
- **Sfoglia**: Libreria, Sequenza;
- **Organizza**: Cataloghi, Persone, Tag;
- **AI**: AI;
- **Importa ed esporta**: Importa, Esporta.

In fondo alla navigazione SHALL stare **Attività** e **Impostazioni**.

Modelli AI e Diagnostica SHALL essere sezioni di Impostazioni e non voci di primo livello. SHALL restare raggiungibili direttamente da Attività, dalla barra di stato, dagli avvisi e dalla palette dei comandi. Persone e identità SHALL essere gestite in un unico modulo Persone. Il modulo AI riguarda lo stato e il controllo delle analisi; le soglie e i modelli restano in Impostazioni.

Ogni voce di primo livello SHALL avere una scorciatoia (Ctrl+1…Ctrl+8 nell'ordine dei gruppi). Un modulo SHALL poter essere aperto su un elemento specifico, ad esempio una persona, un volto, una sezione di Impostazioni, una sezione del modulo AI o un'attività, da altri moduli. La navigazione SHALL essere estendibile con nuovi moduli.

Un modulo salvato come attivo nelle preferenze che non è più una voce di primo livello SHALL aprire la sezione corrispondente.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria ad Attività e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

#### Scenario: Gruppi della navigazione
- **WHEN** l'utente apre l'app
- **THEN** la navigazione mostra i gruppi Sfoglia, Organizza, AI e Importa ed esporta con le loro voci, e in fondo Attività e Impostazioni

#### Scenario: Scorciatoia di modulo
- **WHEN** l'utente preme Ctrl+3
- **THEN** si apre il modulo Cataloghi

#### Scenario: Scorciatoia del modulo AI
- **WHEN** l'utente preme Ctrl+6
- **THEN** si apre il modulo AI

#### Scenario: Identità nel modulo Persone
- **WHEN** l'utente cerca nella navigazione il modulo Identità
- **THEN** non lo trova come voce separata, e le identità sono gestite nella sezione "Persone" del modulo Persone

#### Scenario: Preferenza del modulo non più esistente
- **WHEN** l'ultimo modulo aperto salvato nelle preferenze era "Identità", "Modelli AI" o "Diagnostica"
- **THEN** all'avvio si apre rispettivamente Persone sulla sezione "Persone", Impostazioni sulla sezione "Modelli AI e sistema" o Impostazioni sulla sezione "Diagnostica"

#### Scenario: Diagnostica da un errore
- **WHEN** l'utente fa clic sul conteggio degli errori nella barra di stato
- **THEN** si apre Impostazioni › Diagnostica filtrata sugli errori
