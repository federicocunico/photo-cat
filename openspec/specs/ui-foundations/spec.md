# ui-foundations Specification

## Purpose
Un solo insieme di componenti e regole di interazione per tutta l'interfaccia: dialoghi e avvisi accessibili, conferme solo per ciò che non si annulla, annullamento globale, messaggi d'errore utili, stati vuoti con un'azione, terminologia coerente e requisiti WCAG 2.2 AA.

## Requirements

### Requirement: Componenti condivisi
L'interfaccia SHALL costruire dialoghi, conferme, menu, avvisi, messaggi in linea, stati vuoti, intestazioni di pagina, tabelle, campi di modulo, selettori di cartella, controlli segmentati, schede, etichette di stato e suggerimenti con un unico insieme di componenti condivisi. Colori, spaziature, raggi, tipografia, elevazioni e durate delle animazioni SHALL provenire dai token del tema. Uno stesso elemento SHALL avere lo stesso aspetto e lo stesso comportamento in tutti i moduli.

#### Scenario: Stesso dialogo ovunque
- **WHEN** l'utente apre il dialogo per rimuovere una cartella in Importa e quello per eliminare un catalogo in Cataloghi
- **THEN** i due dialoghi hanno la stessa struttura (titolo, descrizione dell'effetto, comando principale a destra, "Annulla"), gli stessi tasti e lo stesso stile

#### Scenario: Cambio di un token
- **WHEN** il raggio dei controlli nel tema passa da 6 a 8 px
- **THEN** pulsanti, campi e menu di tutti i moduli adottano il nuovo raggio senza altre modifiche

### Requirement: Dialoghi accessibili
Ogni dialogo modale SHALL:
- spostare il focus al suo interno all'apertura;
- tenere il focus al suo interno finché è aperto;
- chiudersi con Esc;
- restituire il focus al comando che l'ha aperto.

Il comando principale SHALL rispondere a Invio quando il focus non è in un campo multilinea.

#### Scenario: Esc subito dopo l'apertura
- **WHEN** l'utente apre "Rimuovi cartella dalla libreria" e preme subito Esc
- **THEN** il dialogo si chiude senza effetti e il focus torna sul comando "Rimuovi"

#### Scenario: Tab nel dialogo
- **WHEN** il dialogo è aperto e l'utente preme Tab più volte
- **THEN** il focus percorre solo i controlli del dialogo

### Requirement: Conferme
L'app SHALL chiedere conferma con un dialogo solo per le operazioni che modificano file sul disco o che non si possono annullare. Il dialogo SHALL indicare quanti elementi sono coinvolti e che cosa succede ai file originali. Il comando SHALL avere un verbo esplicito, in stile di pericolo quando l'operazione è distruttiva. L'app MUST NOT usare dialoghi nativi del browser.

#### Scenario: Organizzazione su disco
- **WHEN** l'utente avvia "Crea cartelle" spostando 1.200 file
- **THEN** compare un dialogo "Spostare 1.200 file in D:\Ordinate?" con il comando "Sposta 1.200 file" in stile di pericolo

### Requirement: Annullamento delle operazioni
Le operazioni sui dati del catalogo SHALL essere eseguite senza conferma: tag, cataloghi ed esclusioni, persone, luoghi, categorie e rimozione dalla libreria. SHALL essere annullabili da:
- un avviso con "Annulla operazione";
- Ctrl+Z, finché sono l'ultima operazione eseguita;
- lo storico del modulo Attività.

#### Scenario: Unione di due persone
- **WHEN** l'utente unisce "Giulia" e "Giulia R."
- **THEN** l'unione avviene subito, compare l'avviso "Persone unite · Annulla operazione", e Ctrl+Z la annulla

#### Scenario: Annullamento dopo aver cambiato pagina
- **WHEN** l'utente rinomina un tag in Tag e poi preme Ctrl+Z
- **THEN** la rinomina è annullata anche se la barra delle azioni della Libreria non è visibile

### Requirement: Messaggi d'errore
Gli errori di validazione SHALL comparire accanto al campo. Gli errori di un'operazione SHALL comparire dove l'utente l'ha avviata, o come avviso se ha cambiato pagina. Ogni messaggio d'errore SHALL dire in linguaggio comprensibile che cosa è successo e che cosa fare, con il dettaglio tecnico solo a richiesta. Un esito negativo MUST NOT avere l'aspetto di un esito neutro o positivo.

#### Scenario: Cartella non trovata
- **WHEN** l'utente avvia l'importazione di una cartella che non esiste più
- **THEN** sotto il campo della cartella compare "La cartella non esiste o non è raggiungibile. Scegline un'altra.", e "Dettagli" mostra il messaggio tecnico

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

### Requirement: Stati vuoti utili
Ogni stato vuoto SHALL dire perché la vista è vuota e SHALL offrire almeno un comando che porta al passo successivo, quando esiste (ad esempio "Importa una cartella", "Azzera i filtri", "Avvia l'analisi dei volti"). Uno stato vuoto MUST NOT rimandare a funzioni o pagine che non esistono.

#### Scenario: Libreria vuota
- **WHEN** la libreria non contiene foto
- **THEN** la griglia mostra "Non ci sono ancora foto" con il comando "Importa una cartella"

#### Scenario: Nessun risultato per i filtri
- **WHEN** i filtri attivi non producono risultati
- **THEN** la griglia mostra "Nessuna foto corrisponde ai filtri" con il comando "Azzera i filtri"

#### Scenario: Nessuna persona
- **WHEN** il riconoscimento volti è attivo ma non ha ancora trovato volti
- **THEN** il modulo Persone mostra il motivo e il comando che avvia l'analisi dei volti, o indica l'attività in corso

### Requirement: Comandi disabilitati spiegati
Un comando disabilitato SHALL spiegare perché, con un testo accanto o un suggerimento raggiungibile anche da tastiera, e SHALL indicare che cosa lo abilita.

#### Scenario: Esporta senza selezione
- **WHEN** l'ambito di Esporta è "Selezione" e non ci sono foto selezionate
- **THEN** il comando di avvio è disabilitato con la spiegazione "Nessuna foto selezionata: seleziona foto in Libreria o scegli un altro ambito"

#### Scenario: Passi preliminari
- **WHEN** "Crea cartelle" richiede l'anteprima e la verifica del metodo
- **THEN** la pagina mostra i passi come elenco numerato, con lo stato di ciascuno, e il comando si abilita al completamento

### Requirement: Terminologia coerente
Tutti i testi dell'interfaccia SHALL usare il glossario dell'app:
- **Attività**: lavori in background;
- **Annulla**: chiudere un dialogo;
- **Interrompi**: fermare un'attività;
- **Annulla operazione**: tornare indietro;
- **Rimuovi da** / **Escludi da**: togliere da un contenitore;
- **Elimina**: cancellare dati dell'app;
- **Sensibilità**: soglie;
- **Categoria**: voce della tassonomia.

#### Scenario: Fermare un'attività
- **WHEN** l'utente vuole fermare un'importazione in corso
- **THEN** il comando si chiama "Interrompi" e non "Annulla"

### Requirement: Nessun identificatore interno
L'interfaccia MUST NOT mostrare identificatori interni: codici di attività, operatori o campi, chiavi di configurazione, id numerici di persone o volti. Le date SHALL essere formattate secondo la lingua dell'interfaccia.

#### Scenario: Attività nello storico
- **WHEN** l'utente apre lo storico delle attività
- **THEN** ogni voce mostra tipo in parole, oggetto, data ed esito, ad esempio "Importazione di D:\Foto · 3 nov, 10:12 · 1.240 foto", e non "#12 · import"

#### Scenario: Persona senza nome
- **WHEN** un gruppo di volti non ha nome
- **THEN** è mostrato come "Persona senza nome" con il numero di foto, senza id interno

### Requirement: Accessibilità
L'interfaccia SHALL rispettare i criteri WCAG 2.2 AA applicabili a un'app desktop:
- bersagli interattivi di almeno 24×24 px, o con spaziatura equivalente;
- focus sempre visibile e non coperto;
- nessun controllo annidato in un altro;
- nessuna informazione affidata al solo colore;
- un'etichetta accessibile per ogni controllo con sola icona.

#### Scenario: Gruppo di foto simili
- **WHEN** una miniatura rappresenta un gruppo di foto simili
- **THEN** l'apertura del gruppo è un comando separato e raggiungibile da tastiera, non annidato nel pulsante della miniatura

#### Scenario: Colore del dispositivo
- **WHEN** Sequenza distingue le foto di due fotocamere
- **THEN** oltre al colore ogni miniatura mostra anche un'iniziale o un'icona del dispositivo

### Requirement: Uso da tastiera
Tutte le funzioni SHALL essere utilizzabili da tastiera, comprese le righe di tabelle e le liste selezionabili. Ogni operazione di trascinamento SHALL avere un'alternativa senza trascinamento.

#### Scenario: Riga di diagnostica da tastiera
- **WHEN** l'utente porta il focus sulla tabella degli errori e usa le frecce e Invio
- **THEN** la riga con il focus si seleziona e il dettaglio si apre

#### Scenario: Pannello flottante senza mouse
- **WHEN** un pannello flottante ha il focus e l'utente sceglie "Sposta" dal menu del pannello e usa le frecce
- **THEN** il pannello si sposta senza trascinamento

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
