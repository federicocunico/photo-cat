# Spec Delta

## MODIFIED Requirements

### Requirement: Pannello di ispezione
Selezionando un file, il pannello di ispezione SHALL mostrare anteprima, percorso, nome, caratteristiche tecniche, date, luogo, catalogazioni AI con confidenza, persone, tag e cataloghi, permettendo la modifica dei campi correggibili, incluso il luogo.

#### Scenario: Ispezione file
- **WHEN** l'utente seleziona una foto
- **THEN** il pannello mostra tutti i dati del file e consente di correggere il tipo soggetto

#### Scenario: Correzione del luogo
- **WHEN** l'utente modifica il luogo di una foto da "D7000" a "Giappone" nel pannello di ispezione
- **THEN** il luogo diventa "Giappone" con fonte "manuale" e il facet "Luogo" della libreria si aggiorna

#### Scenario: Suggerimenti del luogo
- **WHEN** l'utente inizia a digitare il luogo
- **THEN** il pannello suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline

### Requirement: Impostazioni
L'utente SHALL poter configurare: profilo risorse e limite RAM, provider di esecuzione, soglie dell'automatismo AI (CLIP per gruppo e categoria, rilevamento persone, rilevamento volti, somiglianza dei volti), tassonomia, attivazione del riconoscimento volti, nomi di cartella da ignorare per il luogo, applicazioni esterne e azione del doppio clic, livello di log, tema, lingua e percorso dei dati dell'app.

#### Scenario: Modifica profilo
- **WHEN** l'utente cambia il profilo risorse da "Bilanciato" a "Leggero"
- **THEN** i job successivi usano il nuovo budget di memoria

#### Scenario: Soglie di gruppo preservate
- **WHEN** l'utente ha impostato soglie diverse per i gruppi della tassonomia e modifica un'altra impostazione
- **THEN** le soglie dei gruppi restano quelle impostate dall'utente

## ADDED Requirements

### Requirement: Menu contestuale della griglia
Il clic destro su una miniatura SHALL aprire un menu contestuale che agisce sulla selezione corrente (o sulla sola miniatura se non selezionata) con almeno: "Apri con ▸" (applicazioni esterne configurate e app predefinita del sistema), "Mostra nella cartella", "Imposta luogo…" e le azioni bulk più comuni; il menu SHALL essere accessibile da tastiera.

#### Scenario: Clic destro su miniatura non selezionata
- **WHEN** sono selezionate 3 foto e l'utente fa clic destro su una quarta foto non selezionata
- **THEN** la selezione diventa la sola quarta foto e il menu agisce su di essa

#### Scenario: Clic destro sulla selezione
- **WHEN** sono selezionate 20 foto e l'utente fa clic destro su una di esse
- **THEN** il menu indica che agisce su 20 foto

#### Scenario: Apertura da tastiera
- **WHEN** una miniatura ha il focus e l'utente preme il tasto Menu o Maiusc+F10
- **THEN** si apre il menu contestuale navigabile con le frecce
