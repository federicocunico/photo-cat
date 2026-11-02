# Spec Delta

## MODIFIED Requirements

### Requirement: Soglie dell'automatismo configurabili
Il sistema SHALL esporre nelle impostazioni, con valori predefiniti ripristinabili, le soglie che governano l'automatismo: soglia CLIP per gruppo e per categoria, soglia di rilevamento persone, soglia di rilevamento volti, dimensione minima dei volti, nitidezza minima dei volti e soglia di somiglianza per il raggruppamento dei volti. Valori fuori dall'intervallo 0–1 MUST essere rifiutati.

#### Scenario: Ripristino dei predefiniti
- **WHEN** l'utente ha modificato più soglie e sceglie "Ripristina predefiniti"
- **THEN** tutte le soglie tornano ai valori predefiniti dell'applicazione

#### Scenario: Valore non valido
- **WHEN** l'utente inserisce 1,5 come soglia di rilevamento persone
- **THEN** il valore viene rifiutato e la soglia precedente resta in vigore

#### Scenario: Nitidezza minima dei volti
- **WHEN** l'utente alza la nitidezza minima dei volti
- **THEN** l'anteprima indica quanti volti verrebbero scartati e, al salvataggio, il raggruppamento viene ricalcolato senza rianalizzare le foto
