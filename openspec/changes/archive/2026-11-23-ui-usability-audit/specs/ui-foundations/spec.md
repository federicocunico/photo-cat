# Spec Delta

## MODIFIED Requirements

### Requirement: Avvisi non modali
Gli avvisi non modali SHALL potersi chiudere, SHALL impilarsi senza coprirsi e MUST NOT coprire l'elemento che ha il focus. Ogni avviso SHALL dire che cosa è stato fatto e su che cosa, con lo stesso verbo del comando; MUST NOT usare testi generici come "Operazione completata" o "Salvataggio completato". Quando l'operazione si può annullare, l'avviso SHALL offrire "Annulla operazione".

#### Scenario: Più avvisi
- **WHEN** due operazioni terminano a pochi secondi di distanza
- **THEN** i due avvisi sono visibili uno sopra l'altro, ognuno con il proprio pulsante di chiusura

#### Scenario: Avviso che nomina l'oggetto
- **WHEN** l'utente dà il nome "Gioia" a un gruppo di volti
- **THEN** l'avviso dice "Gruppo chiamato Gioia" e offre "Annulla operazione"

#### Scenario: Fine di un'importazione
- **WHEN** termina l'importazione di "D:\Foto\Halloween"
- **THEN** l'avviso dice "Importazione di Halloween completata: 54 foto", non "Operazione completata"

## ADDED Requirements

### Requirement: Barre dei comandi
Una barra dei comandi MUST NOT andare a capo e nessun testo MUST uscire da un pulsante, fino alla larghezza minima della finestra (1.024 px). Le azioni che non entrano SHALL passare, dalla meno usata, in un menu "Altro". Le azioni sulla selezione SHALL comparire solo quando c'è una selezione; i comandi di vista (ordinamento, filtri della vista) SHALL stare a destra, separati.

#### Scenario: Colonna stretta
- **WHEN** la finestra è larga 1.024 px e l'utente seleziona tre volti in Persone
- **THEN** la barra resta su una riga e le azioni che non entrano sono nel menu "Altro"

#### Scenario: Niente azioni disattivate in fila
- **WHEN** l'utente apre un modulo senza aver selezionato nulla
- **THEN** non vede una fila di pulsanti d'azione disattivati in attesa di una selezione

### Requirement: Numeri e punteggi
Numeri e date SHALL seguire il formato della lingua dell'interfaccia (in italiano la virgola decimale). Punteggi di somiglianza o di confidenza tra 0 e 1 MUST NOT comparire fuori da Impostazioni › Avanzate e dalla Diagnostica: altrove SHALL essere espressi in parole o come sensibilità. Ogni testo con un numero SHALL usare la forma singolare o plurale corretta.

#### Scenario: Punteggi dei volti
- **WHEN** l'utente apre i volti di un gruppo
- **THEN** sotto i volti non compaiono valori come "0,77"; i volti di qualità bassa hanno un'icona con la spiegazione

#### Scenario: Un solo elemento
- **WHEN** l'utente seleziona un volto
- **THEN** il conteggio dice "1 selezionato"

### Requirement: Terminologia di Persone
I testi del modulo Persone e di ogni punto che ne parla SHALL usare:
- **persona**: un'identità con nome, valida in tutte le librerie ("identità" MUST NOT comparire nei testi);
- **persona senza nome**: un gruppo di volti da rivedere;
- **cerchia**: il raggruppamento delle persone, ad esempio Famiglia;
- **Sposta in…**: spostare volti;
- **Unisci a…**: unire gruppi o persone.

La parola "gruppo" SHALL indicare solo un gruppo di volti.

#### Scenario: Nuova persona
- **WHEN** l'utente vuole creare un'identità dalla sezione "Persone"
- **THEN** il comando si chiama "Nuova persona" e non "Nuova identità"

#### Scenario: Cerchia
- **WHEN** l'utente modifica i dettagli di una persona
- **THEN** il campo per Famiglia o Amici si chiama "Cerchia" e non "Gruppo"
