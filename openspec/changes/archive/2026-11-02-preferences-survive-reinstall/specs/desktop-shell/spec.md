## ADDED Requirements

### Requirement: Preferenze conservate tra installazioni
Installare una nuova versione sopra una esistente, oppure disinstallare e reinstallare, SHALL conservare impostazioni, libreria, identità, set di tag e preferenze dell'interfaccia (tema, lingua, modulo attivo, pannelli laterali, ordinamento e dimensione delle miniature, posizione dei pannelli flottanti), anche quando la disinstallazione elimina i dati del profilo dell'interfaccia. I dati SHALL essere rimossi solo cancellando a mano la cartella dell'app.

#### Scenario: Aggiornamento sopra la versione installata
- **WHEN** l'utente installa la nuova versione sopra quella esistente
- **THEN** al primo avvio libreria, impostazioni e preferenze dell'interfaccia sono quelle di prima

#### Scenario: Disinstallazione con eliminazione dei dati dell'app
- **WHEN** l'utente aveva ordinato la griglia per nome e ridimensionato la barra laterale, poi disinstalla scegliendo "Elimina i dati dell'applicazione" e reinstalla
- **THEN** al primo avvio la griglia è ordinata per nome, la barra laterale ha la larghezza di prima e la libreria è intatta

#### Scenario: Preferenze della versione precedente
- **WHEN** l'utente avvia per la prima volta la nuova versione dopo aver usato una versione che non salvava le preferenze dell'interfaccia nel file
- **THEN** le preferenze in uso vengono salvate subito e sopravvivono a una successiva disinstallazione
