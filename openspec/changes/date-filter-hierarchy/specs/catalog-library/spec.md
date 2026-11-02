## ADDED Requirements

### Requirement: Filtro per data gerarchico
Il filtro per data della libreria SHALL essere una gerarchia anno → mese → giorno con una sola scelta per volta. Scelto un anno SHALL comparire sotto di esso l'elenco dei mesi di quell'anno con foto; scelto un mese, l'elenco dei giorni di quel mese con foto; ogni voce con il numero di foto. Mesi e giorni SHALL esistere solo dentro l'anno o il mese scelto, mai come filtro a sé. Date e conteggi SHALL usare l'ora locale corretta di ogni foto.

#### Scenario: Scelta del mese dentro l'anno
- **WHEN** la libreria contiene foto del 2023 scattate a marzo e a maggio e l'utente sceglie l'anno 2023
- **THEN** sotto "2023" compaiono "marzo" e "maggio" con i rispettivi conteggi, e scegliendo "maggio" la griglia mostra solo le foto di maggio 2023

#### Scenario: Scelta del giorno dentro il mese
- **WHEN** l'utente ha scelto maggio 2023 e in quel mese ci sono foto del 14 e del 20
- **THEN** sotto "maggio" compaiono i giorni 14 e 20 con i conteggi, e scegliendo il 14 la griglia mostra solo le foto del 14 maggio 2023

#### Scenario: Mesi non selezionabili senza anno
- **WHEN** nessun anno è scelto
- **THEN** la lista mostra solo gli anni e non è possibile filtrare per un mese o un giorno di tutti gli anni

#### Scenario: Risalire la gerarchia
- **WHEN** è scelto il 14 maggio 2023 e l'utente toglie la spunta al giorno, poi al mese, poi all'anno
- **THEN** il filtro diventa prima maggio 2023, poi l'anno 2023, poi nessun filtro per data

#### Scenario: Cambio di anno o di mese
- **WHEN** è scelto maggio 2023 e l'utente sceglie l'anno 2022 oppure il mese di marzo 2023
- **THEN** il filtro diventa rispettivamente l'anno 2022 intero oppure marzo 2023, senza conservare il mese o il giorno precedente

#### Scenario: Altri mesi restano selezionabili
- **WHEN** è attivo il filtro "Esterno", l'utente ha scelto maggio 2023 e marzo 2023 contiene 40 foto esterne
- **THEN** "marzo" resta nella lista con 40 foto e sceglierlo sostituisce maggio

#### Scenario: Ora locale corretta
- **WHEN** una foto ha istante reale 31 maggio 2023 23:30 UTC e il fuso dell'ora mostrata è +02:00
- **THEN** la foto viene contata nel 1° giugno 2023 e compare scegliendo giugno 2023, non maggio

#### Scenario: Foto senza data
- **WHEN** alcune foto non hanno data di scatto
- **THEN** non sono contate in nessun anno, mese o giorno e non compaiono quando è scelto un filtro per data

#### Scenario: Chip del filtro attivo
- **WHEN** è scelto il 14 maggio 2023 con l'interfaccia in italiano
- **THEN** tra i filtri attivi compare il chip "14 maggio 2023" e rimuoverlo toglie il filtro per data
