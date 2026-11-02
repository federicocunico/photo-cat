# Spec Delta

## MODIFIED Requirements

### Requirement: Griglia di miniature performante
La griglia SHALL visualizzare librerie di almeno 100.000 elementi con scorrimento fluido, caricando le miniature in modo progressivo e con dimensione delle miniature regolabile. La memoria usata dall'interfaccia per la griglia SHALL restare limitata indipendentemente da quanto l'utente ha scorso, e nessuna richiesta di dati SHALL bloccare l'interfaccia mentre il motore calcola.

#### Scenario: Scorrimento rapido
- **WHEN** l'utente scorre rapidamente una libreria da 100.000 foto
- **THEN** l'interfaccia resta reattiva e mostra segnaposto finché le miniature non sono caricate

#### Scenario: Scorrimento fino in fondo
- **WHEN** l'utente scorre dall'inizio alla fine una libreria da 100.000 foto
- **THEN** la memoria dell'interfaccia resta entro un limite costante e tornando all'inizio le pagine vengono ricaricate

#### Scenario: Filtro durante un import
- **WHEN** è in corso un import da 100.000 file e l'utente cambia filtro nella libreria
- **THEN** la finestra continua a rispondere a scorrimento e clic mentre i nuovi risultati vengono calcolati

## ADDED Requirements

### Requirement: Recupero dalla chiusura dell'interfaccia
Se il processo dell'interfaccia termina mentre l'app è aperta, il sistema SHALL lasciare proseguire o mettere in pausa in sicurezza i job in corso, ricaricare l'interfaccia e mostrare all'utente che cosa è successo, senza perdere dati del catalogo.

#### Scenario: Interfaccia ricaricata
- **WHEN** la WebView termina durante un import
- **THEN** l'interfaccia viene ricaricata, mostra un avviso che indica la terminazione e il collegamento alla diagnostica, e il job di import risulta ancora in corso o in pausa, mai perso

#### Scenario: Terminazioni ripetute
- **WHEN** l'interfaccia termina tre volte in pochi minuti
- **THEN** il sistema mette in pausa i job, smette di ricaricare automaticamente e mostra una finestra di errore con il percorso dei log e il comando per creare il pacchetto diagnostico
