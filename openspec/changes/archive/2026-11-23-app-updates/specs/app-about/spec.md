## ADDED Requirements

### Requirement: Aggiornamenti nelle Informazioni
Impostazioni › Informazioni SHALL contenere l'impostazione "Controlla gli aggiornamenti all'avvio" (attiva in modo predefinito) e il comando "Controlla ora", con l'esito in parole accanto ("photo-cat 0.5.3 è la versione più recente", "È disponibile photo-cat 0.5.4", oppure il motivo dell'errore).

#### Scenario: Controllo automatico disattivato
- **WHEN** l'utente disattiva "Controlla gli aggiornamenti all'avvio" e riavvia l'app
- **THEN** all'avvio non parte alcuna richiesta a GitHub, e "Controlla ora" continua a funzionare
