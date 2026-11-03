# ui-foundations

## ADDED Requirements

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
Gli avvisi non modali SHALL potersi chiudere, SHALL impilarsi senza coprirsi e MUST NOT coprire l'elemento che ha il focus.

#### Scenario: Più avvisi
- **WHEN** due operazioni terminano a pochi secondi di distanza
- **THEN** i due avvisi sono visibili uno sopra l'altro, ognuno con il proprio pulsante di chiusura

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
