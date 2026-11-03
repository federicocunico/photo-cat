# Spec Delta

## ADDED Requirements

### Requirement: Editor delle regole comprensibile
L'editor dei cataloghi smart SHALL presentare ogni condizione come campo, confronto e valore espressi nella lingua dell'interfaccia, e MUST NOT mostrare identificatori interni di campi o operatori. Per il valore SHALL offrire un selettore adatto al campo: elenco con ricerca, numero con unità o anno. L'editor SHALL mostrare un riepilogo della regola in una frase e il numero di foto corrispondenti, aggiornati durante la modifica.

#### Scenario: Nuova regola
- **WHEN** l'utente aggiunge una condizione e sceglie il campo "Persone nella foto"
- **THEN** i confronti offerti sono "è", "non è", "più di", "almeno", "meno di", "al massimo", e il valore è un numero

#### Scenario: Riepilogo della regola
- **WHEN** la regola è "Tipo soggetto è Ritratto e Anno è almeno 2022"
- **THEN** l'editor mostra la frase "Foto con tipo soggetto Ritratto, dal 2022 in poi" e il numero di foto corrispondenti

### Requirement: Conteggi nell'elenco dei cataloghi
Nell'elenco dei cataloghi ogni catalogo SHALL mostrare il tipo, il numero di foto e, se presenti, il numero di foto escluse, con accesso diretto alla vista delle escluse.

#### Scenario: Catalogo con esclusioni nell'elenco
- **WHEN** il catalogo "Viaggio Toscana" ha 312 foto e 6 escluse
- **THEN** l'elenco dei cataloghi mostra "Smart · 312 foto · 6 escluse", e il clic su "6 escluse" apre la vista delle foto escluse
