# Spec Delta

## MODIFIED Requirements

### Requirement: Soglie dell'automatismo configurabili
Il sistema SHALL esporre nelle impostazioni, con valori predefiniti ripristinabili, le soglie che governano l'automatismo: soglia CLIP per gruppo e per categoria, soglia di rilevamento persone, soglia di rilevamento volti, dimensione minima dei volti, soglia di somiglianza per il raggruppamento dei volti e soglia di somiglianza dei gruppi di foto simili. Valori fuori dall'intervallo 0–1 MUST essere rifiutati.

#### Scenario: Ripristino dei predefiniti
- **WHEN** l'utente ha modificato più soglie e sceglie "Ripristina predefiniti"
- **THEN** tutte le soglie tornano ai valori predefiniti dell'applicazione

#### Scenario: Valore non valido
- **WHEN** l'utente inserisce 1,5 come soglia di rilevamento persone
- **THEN** il valore viene rifiutato e la soglia precedente resta in vigore

#### Scenario: Soglia dei gruppi di foto simili
- **WHEN** l'utente cambia la soglia di somiglianza dei gruppi di foto simili
- **THEN** l'anteprima indica quanti gruppi si formerebbero e, al salvataggio, i gruppi vengono ricalcolati senza rieseguire i modelli
