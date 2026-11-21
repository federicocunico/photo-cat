# Spec Delta

## ADDED Requirements

### Requirement: Avvisi delle cartelle chiudibili
Gli avvisi di una cartella nella pagina Import (file mancanti, file con errori) SHALL riferirsi solo all'ultimo aggiornamento e potersi chiudere con "Ignora". Un avviso chiuso SHALL restare nascosto finché un aggiornamento successivo non trova nuovi file mancanti o nuovi errori, e allora mostrare solo questi. Chiudere un avviso MUST NOT cambiare lo stato dei file né cancellare le voci del registro errori o del log.

#### Scenario: Foto spostate di proposito
- **WHEN** l'utente ha cancellato 5 foto, l'aggiornamento segnala "5 mancanti" e l'utente sceglie "Ignora"
- **THEN** l'avviso scompare dalla cartella, i 5 file restano segnati mancanti in libreria e "Ricollega i mancanti" e "Rimuovi i mancanti" restano nel menu della cartella

#### Scenario: Nuovi mancanti dopo un avviso chiuso
- **WHEN** dopo aver ignorato "5 mancanti" l'utente cancella altre 2 foto e aggiorna la cartella
- **THEN** la cartella mostra "2 nuovi mancanti" e non i 5 già ignorati

#### Scenario: Errori di un aggiornamento precedente
- **WHEN** un aggiornamento ha trovato 3 file con errori e quello successivo non ne trova
- **THEN** la pagina Import non mostra più errori per quella cartella e le 3 voci restano consultabili in Diagnostica e nel log

## MODIFIED Requirements

### Requirement: Riepilogo dell'aggiornamento
Al termine dell'aggiornamento di una sorgente il sistema SHALL mostrare quanti file sono nuovi, modificati, spostati o rinominati, diventati mancanti, tornati presenti e con errori, con accesso diretto alla libreria filtrata su ciascun gruppo. Il riepilogo SHALL riguardare solo l'ultimo aggiornamento e sostituire quello precedente.

#### Scenario: Cartella riorganizzata
- **WHEN** l'utente ha aggiunto 30 foto, spostato 12 foto in un'altra sottocartella e cancellato 5 foto, e poi aggiorna la sorgente
- **THEN** il riepilogo indica 30 nuove, 12 spostate e 5 mancanti, e un clic su "5 mancanti" mostra in libreria quei 5 file

#### Scenario: Nessuna modifica
- **WHEN** l'utente aggiorna una sorgente senza cambiamenti
- **THEN** il riepilogo indica che non ci sono modifiche

#### Scenario: Riepilogo sostituito
- **WHEN** il primo aggiornamento ha trovato 5 mancanti e 2 errori e il secondo trova solo 3 nuove foto
- **THEN** il riepilogo mostra solo "3 nuove"

### Requirement: Monitoraggio delle sorgenti
L'utente SHALL poter attivare per ogni sorgente il monitoraggio delle modifiche, che accoda automaticamente un import incrementale quando la cartella cambia mentre l'app è aperta. L'opzione SHALL chiamarsi "Aggiorna automaticamente" e spiegare, al passaggio del puntatore e nella guida, che le modifiche alla cartella vengono importate da sole solo mentre l'app è aperta. La cartella SHALL mostrare se il monitoraggio è attivo, la data dell'ultimo aggiornamento automatico, oppure che non è disponibile e perché.

#### Scenario: Nuova foto aggiunta
- **WHEN** l'utente copia una nuova foto in una sorgente monitorata
- **THEN** il file viene importato e accodato per l'analisi senza azioni manuali

#### Scenario: Spiegazione dell'opzione
- **WHEN** l'utente passa il puntatore su "Aggiorna automaticamente" nella pagina Import
- **THEN** compare la spiegazione che, mentre photo-cat è aperto, le foto aggiunte, spostate o cancellate nella cartella vengono importate o aggiornate da sole

#### Scenario: Stato del monitoraggio
- **WHEN** una cartella monitorata è stata aggiornata in automatico alle 15:42
- **THEN** la cartella indica "Aggiornamento automatico attivo · ultimo alle 15:42"

#### Scenario: Cartella non raggiungibile
- **WHEN** la cartella monitorata è su un disco scollegato
- **THEN** la cartella indica che l'aggiornamento automatico non è disponibile perché la cartella non è raggiungibile
