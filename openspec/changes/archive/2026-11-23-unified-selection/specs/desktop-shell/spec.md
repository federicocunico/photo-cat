# Spec Delta

## MODIFIED Requirements

### Requirement: Navigazione da tastiera nella griglia
Le griglie di Libreria, cataloghi, Sequenza e Persone, e gli elenchi selezionabili, SHALL essere navigabili come descritto nei requisiti "Selezione da tastiera" e "Comandi di selezione da tastiera" del modello di selezione comune:
- frecce, Home/Fine e Pagina su/giù per spostare il focus e selezionare l'elemento raggiunto;
- Maiusc con una freccia per estendere la selezione;
- Ctrl con una freccia per spostare solo il focus;
- Ctrl+Spazio per aggiungere o togliere l'elemento con il focus;
- Ctrl+A per selezionare tutti gli elementi della vista.

La miniatura con il focus SHALL essere sempre visibile e distinta da quelle selezionate.

#### Scenario: Selezione da tastiera
- **WHEN** l'utente porta il focus su una miniatura e preme Maiusc+→ due volte
- **THEN** sono selezionate tre foto consecutive e il focus è sulla terza

#### Scenario: Seleziona tutto
- **WHEN** la vista ha un filtro con 8.000 foto e l'utente preme Ctrl+A
- **THEN** sono selezionate tutte le 8.000 foto del filtro, senza caricarle tutte nell'interfaccia

#### Scenario: Frecce come in un gestore di file
- **WHEN** in Libreria è selezionata una foto e l'utente preme la freccia giù
- **THEN** è selezionata solo la foto sotto, che ha il focus, e la barra delle azioni mostra "1 selezionata"

#### Scenario: Spostare il focus senza selezionare
- **WHEN** in Libreria sono selezionate tre foto e l'utente preme Ctrl+→
- **THEN** il focus passa alla foto successiva e le tre foto restano le sole selezionate
