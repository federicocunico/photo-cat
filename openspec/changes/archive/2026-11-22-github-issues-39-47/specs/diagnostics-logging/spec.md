# Spec Delta

## ADDED Requirements

### Requirement: Errori archiviati
Dalla diagnostica l'utente SHALL poter archiviare gli errori già visti: singole voci, un gruppo di errori o tutti quelli che corrispondono al filtro corrente. Gli errori archiviati MUST NOT essere contati dall'indicatore globale degli errori, SHALL essere nascosti dall'elenco predefinito e restare consultabili con il filtro "Archiviati", da cui si possono ripristinare. L'archiviazione MUST NOT cancellare le voci dal registro né dal file di log.

#### Scenario: Errori visti e archiviati
- **WHEN** la diagnostica mostra 14 errori e l'utente sceglie "Archivia tutti" sul filtro corrente
- **THEN** l'indicatore globale mostra "Nessun errore", l'elenco predefinito è vuoto e il filtro "Archiviati" mostra le 14 voci

#### Scenario: Nuovo errore dopo l'archiviazione
- **WHEN** dopo l'archiviazione un nuovo import registra 2 errori
- **THEN** l'indicatore globale mostra 2 errori e l'elenco predefinito mostra solo quelle 2 voci

#### Scenario: Ripristino
- **WHEN** l'utente apre "Archiviati" e sceglie "Ripristina" su una voce
- **THEN** la voce torna nell'elenco predefinito e nel conteggio dell'indicatore globale

#### Scenario: Log intatto
- **WHEN** l'utente archivia tutti gli errori
- **THEN** il file di log e il pacchetto diagnostico contengono ancora le voci archiviate
