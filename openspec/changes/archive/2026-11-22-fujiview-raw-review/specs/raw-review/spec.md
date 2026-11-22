# Spec Delta

## Purpose

Permette di rivedere una cartella di RAF Fujifilm e delle loro varianti sviluppate senza importarla nella libreria: confrontare le varianti di ogni scatto, scegliere la migliore e valutarla, con una sessione di revisione che si conserva tra un'apertura e l'altra.

## ADDED Requirements

### Requirement: Apertura di una cartella senza import
Il modulo Varianti RAW (Fujifilm X-T5) SHALL aprire una cartella scelta dall'utente senza aggiungerla alla libreria. Gli scatti di quella cartella MUST NOT comparire in Libreria, nei cataloghi o nelle analisi AI finché non vengono esportati in un catalogo. Il modulo MUST NOT scrivere nella cartella aperta, salvo nella cartella di output delle varianti quando l'utente avvia uno sviluppo.

#### Scenario: Cartella di uno shooting
- **WHEN** l'utente apre in Varianti RAW (Fujifilm X-T5) `D:\Shooting\2026-11-20` con 160 RAF
- **THEN** il modulo mostra 160 scatti, la Libreria non contiene nessuna di quelle foto e nella cartella non compaiono file nuovi

#### Scenario: Cartella già nella libreria
- **WHEN** la cartella aperta è dentro una sorgente della libreria
- **THEN** la revisione funziona allo stesso modo e la libreria non cambia

### Requirement: Forme di cartella riconosciute
Il modulo SHALL riconoscere tre forme di cartella:
- RAF al primo livello;
- RAF con una sottocartella `FujiView varianti/<preset>/`;
- una sottocartella per preset con JPEG o TIFF.

Gli scatti SHALL essere abbinati per nome base senza distinzione di maiuscole e ordinati in modo naturale. Un file duplicato nello stesso preset SHALL essere ignorato con un avviso.

#### Scenario: RAF con varianti generate
- **WHEN** la cartella contiene `DSCF0001.RAF` e `FujiView varianti/C1 Kodachrome/DSCF0001.jpg` e `FujiView varianti/Come scattato/DSCF0001.jpg`
- **THEN** lo scatto `DSCF0001` mostra le varianti "C1 Kodachrome" e "Come scattato"

#### Scenario: Layout FujiView classico
- **WHEN** la cartella contiene le sottocartelle `Classic Chrome/` e `Velvia/` con `DSCF2.jpg` e `DSCF10.jpg`
- **THEN** gli scatti sono `DSCF2` e `DSCF10` in quest'ordine, ciascuno con due varianti

#### Scenario: Variante mancante in un preset
- **WHEN** il preset "Velvia" non ha `DSCF10.jpg`
- **THEN** lo scatto `DSCF10` resta nell'elenco e il pannello Velvia indica "variante non disponibile"

#### Scenario: Cartella senza foto
- **WHEN** la cartella non contiene RAF né sottocartelle con JPEG o TIFF
- **THEN** il modulo spiega quali forme di cartella sono supportate e non crea una sessione

### Requirement: Scatti senza varianti
Uno scatto RAF senza varianti SHALL essere confrontabile e valutabile sull'anteprima JPEG incorporata nel RAF, mostrata come pannello "Anteprima RAW". L'"Anteprima RAW" MUST NOT poter essere scelta come variante migliore, perché non è un file JPEG esportabile: per esportare lo scatto l'utente deve prima svilupparlo.

#### Scenario: Revisione prima dello sviluppo
- **WHEN** l'utente apre una cartella di soli RAF su Linux
- **THEN** ogni scatto mostra il pannello "Anteprima RAW" e può essere valutato, ma non ha una variante scelta

#### Scenario: Tentativo di scegliere l'anteprima RAW
- **WHEN** l'utente preme Invio sul pannello "Anteprima RAW"
- **THEN** la scelta non cambia e un messaggio spiega che va sviluppato lo scatto per poterlo esportare

### Requirement: Confronto delle varianti
Il modulo SHALL mostrare un pannello per ogni preset visibile, fino a 9, in una griglia adattata al loro numero, con zoom e spostamento sincronizzati tra i pannelli. SHALL mostrare data, esposizione, fotocamera, obiettivo e la ricetta Fujifilm di ogni variante. L'utente SHALL poter nascondere e mostrare i preset e riordinare i pannelli.

#### Scenario: Zoom sincronizzato
- **WHEN** l'utente ingrandisce con la rotellina su un dettaglio nel pannello "C1"
- **THEN** tutti i pannelli mostrano lo stesso dettaglio allo stesso ingrandimento

#### Scenario: Ricetta della variante
- **WHEN** l'utente passa il puntatore sull'intestazione del pannello "C3"
- **THEN** compaiono simulazione pellicola, gamma dinamica e bilanciamento del bianco letti dai metadati Fujifilm della variante

#### Scenario: Preset nascosto
- **WHEN** l'utente nasconde "Come scattato" e passa allo scatto successivo
- **THEN** il pannello resta nascosto anche per gli altri scatti e dopo la riapertura della sessione

### Requirement: Scelta e valutazione
Per ogni scatto l'utente SHALL poter scegliere una variante migliore (clic sul pannello e Invio, oppure Maiusc+1–9 nell'ordine dei pannelli) e assegnare una valutazione da 0 a 5 (tasti 1–5 e 0, oppure Alt+1–5 e Alt+0). Spazio e Maiusc+Spazio SHALL passare allo scatto successivo e precedente. La scelta SHALL essere evidenziata e MUST NOT far avanzare automaticamente allo scatto successivo.

#### Scenario: Scelta da tastiera
- **WHEN** l'utente preme Maiusc+2 e poi 4
- **THEN** la variante del secondo pannello è la migliore con 4 stelle e lo scatto mostrato non cambia

#### Scenario: Scorciatoie dei moduli
- **WHEN** l'utente è in Varianti RAW (Fujifilm X-T5) e preme Ctrl+3
- **THEN** si apre il modulo Cataloghi e la sessione di revisione resta salvata

### Requirement: Sessione di revisione persistente
Ogni cartella aperta SHALL avere una sessione salvata automaticamente nei dati dell'app, mai nella cartella delle foto. La sessione contiene scelte, valutazioni, preset nascosti, ordine dei pannelli e ultimo scatto. Riaprendo la cartella la sessione SHALL ripartire dallo stesso punto, aggiungendo gli scatti e i preset nuovi senza perdere le scelte esistenti. Il modulo SHALL elencare le sessioni recenti e poter aprire un progetto `.fujiview` di FujiView.

#### Scenario: Riapertura
- **WHEN** l'utente ha valutato 80 scatti su 160, chiude l'app e riapre la stessa cartella
- **THEN** le 80 valutazioni e scelte sono presenti e si riparte dall'ultimo scatto visto

#### Scenario: Nuove varianti dopo lo sviluppo
- **WHEN** dopo aver valutato gli scatti l'utente genera il preset "C5"
- **THEN** compare il pannello "C5" e valutazioni e scelte esistenti restano invariate

#### Scenario: Progetto FujiView
- **WHEN** l'utente apre un file `.fujiview` salvato da FujiView
- **THEN** la sessione riprende la cartella di origine con le sue scelte, valutazioni e preset nascosti

### Requirement: Anteprime rapide
Il modulo SHALL decodificare le anteprime ridotte alla dimensione del pannello, precaricare gli scatti vicini e conservarle in una cache su disco nei dati dell'app con limite di spazio configurabile. Passare a uno scatto già precaricato SHALL mostrare le anteprime senza attesa percepibile.

#### Scenario: Scorrimento degli scatti
- **WHEN** l'utente preme Spazio più volte su JPEG da 40 MP con 8 preset
- **THEN** ogni scatto precaricato compare entro 150 ms su hardware consumer e l'interfaccia resta reattiva

#### Scenario: Limite della cache
- **WHEN** la cache delle anteprime supera il limite configurato
- **THEN** vengono eliminate le anteprime usate meno di recente fino a rientrare nel limite
