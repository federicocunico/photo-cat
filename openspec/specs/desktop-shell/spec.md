# desktop-shell Specification

## Purpose
Definisce la struttura dell'applicazione desktop: layout a pannelli in stile professionale ma semplice, navigazione tra moduli, indicatori globali di avanzamento e risorse, impostazioni e procedura di primo avvio.

## Requirements

### Requirement: Layout a pannelli
L'interfaccia SHALL presentare una barra superiore, una navigazione dei moduli, un pannello laterale sinistro (in Libreria: cataloghi, filtri e, in fondo, cartelle), un'area centrale (griglia o lista), un pannello destro di ispezione e una barra di stato. I pannelli laterali SHALL essere ridimensionabili e richiudibili, anche da tastiera (Ctrl+Maiusc+B nasconde e mostra entrambi i pannelli laterali, Ctrl+B la navigazione dei moduli, Ctrl+I l'ispettore; Tab resta riservato allo spostamento del focus). La navigazione dei moduli SHALL potersi ridurre a sole icone con descrizione al passaggio del mouse, e SHALL ridursi automaticamente a icone sotto i 1.008 px di larghezza della finestra. Le sezioni del pannello sinistro SHALL potersi richiudere singolarmente. Lo stato del layout SHALL essere ricordato.

#### Scenario: Pannello richiuso
- **WHEN** l'utente chiude il pannello di ispezione e riavvia l'app
- **THEN** il pannello risulta ancora chiuso

#### Scenario: Navigazione a icone
- **WHEN** l'utente preme Ctrl+B
- **THEN** la navigazione dei moduli mostra solo le icone, ognuna con nome e scorciatoia al passaggio del mouse, e la griglia guadagna lo spazio liberato

#### Scenario: Finestra stretta
- **WHEN** la finestra viene ridotta a 900 px di larghezza
- **THEN** la navigazione passa a sole icone senza cambiare la preferenza salvata per le finestre larghe

#### Scenario: Concentrazione sulla griglia
- **WHEN** l'utente preme Ctrl+Maiusc+B in Libreria senza un campo di testo attivo
- **THEN** pannello sinistro e ispettore si nascondono, e un nuovo Ctrl+Maiusc+B li ripristina com'erano

#### Scenario: Ordine del pannello sinistro in Libreria
- **WHEN** l'utente apre la Libreria
- **THEN** il pannello sinistro mostra dall'alto la sezione dei cataloghi, poi i filtri e infine le cartelle

### Requirement: Moduli di navigazione
L'applicazione SHALL organizzare la navigazione in gruppi con un'etichetta:
- **Sfoglia**: Libreria, Sequenza;
- **Organizza**: Cataloghi, Persone, Tag;
- **AI**: AI;
- **Importa ed esporta**: Importa, Esporta.

In fondo alla navigazione SHALL stare **Attività** e **Impostazioni**.

Modelli AI e Diagnostica SHALL essere sezioni di Impostazioni e non voci di primo livello. SHALL restare raggiungibili direttamente da Attività, dalla barra di stato, dagli avvisi e dalla palette dei comandi. Persone e identità SHALL essere gestite in un unico modulo Persone. Il modulo AI riguarda lo stato e il controllo delle analisi; le soglie e i modelli restano in Impostazioni.

Ogni voce di primo livello SHALL avere una scorciatoia (Ctrl+1…Ctrl+8 nell'ordine dei gruppi). Un modulo SHALL poter essere aperto su un elemento specifico, ad esempio una persona, un volto, una sezione di Impostazioni, una sezione del modulo AI o un'attività, da altri moduli. La navigazione SHALL essere estendibile con nuovi moduli.

Un modulo salvato come attivo nelle preferenze che non è più una voce di primo livello SHALL aprire la sezione corrispondente.

#### Scenario: Cambio modulo
- **WHEN** l'utente passa da Libreria ad Attività e torna a Libreria
- **THEN** la Libreria conserva filtri, posizione di scorrimento e selezione

#### Scenario: Gruppi della navigazione
- **WHEN** l'utente apre l'app
- **THEN** la navigazione mostra i gruppi Sfoglia, Organizza, AI e Importa ed esporta con le loro voci, e in fondo Attività e Impostazioni

#### Scenario: Scorciatoia di modulo
- **WHEN** l'utente preme Ctrl+3
- **THEN** si apre il modulo Cataloghi

#### Scenario: Scorciatoia del modulo AI
- **WHEN** l'utente preme Ctrl+6
- **THEN** si apre il modulo AI

#### Scenario: Identità nel modulo Persone
- **WHEN** l'utente cerca nella navigazione il modulo Identità
- **THEN** non lo trova come voce separata, e le identità sono gestite nella sezione "Persone" del modulo Persone

#### Scenario: Preferenza del modulo non più esistente
- **WHEN** l'ultimo modulo aperto salvato nelle preferenze era "Identità", "Modelli AI" o "Diagnostica"
- **THEN** all'avvio si apre rispettivamente Persone sulla sezione "Persone", Impostazioni sulla sezione "Modelli AI e sistema" o Impostazioni sulla sezione "Diagnostica"

#### Scenario: Diagnostica da un errore
- **WHEN** l'utente fa clic sul conteggio degli errori nella barra di stato
- **THEN** si apre Impostazioni › Diagnostica filtrata sugli errori

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

### Requirement: Pannello di ispezione
Selezionando un file, il pannello di ispezione SHALL mostrare, in quest'ordine:
- anteprima e nome;
- data, luogo, persone e tag;
- cataloghi e foto simili;
- catalogazioni AI con confidenza;
- caratteristiche tecniche e percorso.

Le sezioni SHALL potersi richiudere, e lo stato SHALL essere ricordato. I campi correggibili, compresi luogo, tipo soggetto e le altre categorie, SHALL essere modificabili direttamente sul campo, senza un modulo separato di modifica. Con più foto selezionate il pannello SHALL mostrare i valori comuni e permettere di modificarli per tutte.

#### Scenario: Ispezione file
- **WHEN** l'utente seleziona una foto
- **THEN** il pannello mostra tutti i dati del file e consente di correggere il tipo soggetto facendo clic sul valore

#### Scenario: Correzione del luogo
- **WHEN** l'utente modifica il luogo di una foto da "D7000" a "Giappone" nel pannello di ispezione
- **THEN** il luogo diventa "Giappone" con fonte "manuale" e il facet "Luogo" della libreria si aggiorna

#### Scenario: Suggerimenti del luogo
- **WHEN** l'utente inizia a digitare il luogo
- **THEN** il pannello suggerisce i luoghi già presenti in libreria e le località del dataset geografico offline

#### Scenario: Selezione multipla
- **WHEN** sono selezionate 12 foto con luoghi diversi e l'utente imposta il luogo "Roma" nel pannello
- **THEN** il pannello mostra "Valori diversi" prima della modifica, e dopo la modifica tutte le 12 foto hanno luogo "Roma", con la possibilità di annullare l'operazione

#### Scenario: Sezione richiusa
- **WHEN** l'utente richiude la sezione "Dati tecnici" e seleziona un'altra foto
- **THEN** la sezione resta richiusa

### Requirement: Indicatori globali
La barra superiore SHALL mostrare sempre un indicatore delle attività in corso con la percentuale complessiva. La barra di stato SHALL mostrare:
- il numero di elementi e di selezionati;
- lo stato dei modelli;
- il consumo di RAM;
- il numero di errori recenti;

con accesso diretto alle rispettive schermate.

L'indicatore delle attività SHALL animarsi solo quando ci sono attività in corso, e SHALL indicare in parole che cosa sta facendo (ad esempio "Importazione · 42%"), mai un identificatore interno. La barra superiore SHALL mostrare un indicatore distinto quando richieste dell'interfaccia al motore sono in corso da più di 300 ms. Quando la memoria è sotto pressione, l'indicatore della RAM SHALL segnalarlo anche con un'icona e un testo, non solo con il colore.

#### Scenario: Job in corso da altro modulo
- **WHEN** è in corso un'analisi e l'utente è in Libreria
- **THEN** la barra superiore mostra "Analisi AI" con la percentuale, e un clic porta al modulo Attività su quell'attività

#### Scenario: Nessun job attivo
- **WHEN** non ci sono attività in corso
- **THEN** l'indicatore è fermo e mostra "Nessuna attività"

#### Scenario: Richiesta lenta
- **WHEN** l'utente applica un filtro il cui calcolo richiede 2 secondi
- **THEN** dopo 300 ms compare nella barra superiore l'indicatore di caricamento, che scompare all'arrivo dei risultati

#### Scenario: Memoria sotto pressione
- **WHEN** il governatore delle risorse segnala pressione di memoria
- **THEN** l'indicatore della RAM nella barra di stato mostra un'icona di avviso e il testo "Memoria quasi esaurita", con accesso al modulo Attività

### Requirement: Stati di caricamento
Ogni elenco, griglia o pannello che mostra dati SHALL distinguere tre stati: in caricamento (indicatore o segnaposto), vuoto (messaggio di stato vuoto) ed errore (messaggio con "Riprova"). Lo stato vuoto MUST NOT essere mostrato prima che i dati siano arrivati. Quando un dato già visibile si sta aggiornando, il pannello SHALL segnalarlo senza nascondere il contenuto.

#### Scenario: Elenco delle sorgenti in caricamento
- **WHEN** l'utente apre la pagina Import e l'elenco delle sorgenti non è ancora arrivato
- **THEN** la pagina mostra un segnaposto di caricamento e non il messaggio "nessuna sorgente"

#### Scenario: Elenco vuoto
- **WHEN** l'elenco delle persone è arrivato ed è vuoto
- **THEN** la pagina Persone mostra lo stato vuoto con l'indicazione di come ottenere persone

#### Scenario: Errore di caricamento
- **WHEN** il caricamento dei cataloghi fallisce
- **THEN** la pagina mostra il messaggio d'errore e "Riprova", che ripete la richiesta

#### Scenario: Primo caricamento della griglia
- **WHEN** l'utente apre la Libreria con 100.000 foto
- **THEN** la griglia mostra segnaposto delle miniature e il conteggio "in calcolo…" finché il totale non è noto

#### Scenario: Aggiornamento in background
- **WHEN** la griglia mostra dei risultati e l'utente cambia filtro
- **THEN** i risultati precedenti restano visibili attenuati con una barra di avanzamento sottile finché arrivano i nuovi

### Requirement: Azioni in corso
Un comando che avvia un'operazione SHALL mostrare che è in corso (indicatore nel comando) e MUST NOT poter essere avviato di nuovo finché l'operazione non termina. Le operazioni che durano più di un secondo SHALL concludersi con un avviso non modale che ne riporta l'esito.

#### Scenario: Doppio clic su Salva
- **WHEN** l'utente fa doppio clic su "Salva" nell'editor di un'identità
- **THEN** il salvataggio viene inviato una sola volta e il pulsante mostra lo stato "in corso" fino all'esito

#### Scenario: Esito di un'operazione lenta
- **WHEN** l'utente unisce due tag con 5.000 foto e l'operazione dura 3 secondi
- **THEN** al termine compare un avviso con l'esito, che si chiude da solo

### Requirement: Schermata di avvio
All'avvio l'app SHALL mostrare subito una finestra con una schermata di avvio che indica la fase in corso (apertura del catalogo, backup e migrazione, caricamento del runtime AI). La Libreria o l'onboarding SHALL comparire solo quando la configurazione è stata letta. Un errore bloccante SHALL essere mostrato nella schermata con il percorso dei log.

#### Scenario: Migrazione del catalogo
- **WHEN** l'app si avvia dopo un aggiornamento che migra un catalogo grande
- **THEN** la finestra compare subito con "Backup e aggiornamento del catalogo…" invece di restare invisibile

#### Scenario: Primo avvio
- **WHEN** l'onboarding non è ancora stato completato
- **THEN** dopo la schermata di avvio compare direttamente l'onboarding, senza mostrare prima la Libreria

#### Scenario: Errore all'avvio
- **WHEN** il catalogo non si apre perché il disco dati non è disponibile
- **THEN** la schermata di avvio mostra l'errore, il percorso dei log e i comandi per riprovare o scegliere un'altra cartella dati

### Requirement: Stile visivo
L'interfaccia SHALL usare:
- tre temi, "Come il sistema" (predefinito), scuro e chiaro;
- contrasto del testo di almeno 4,5:1, e di almeno 3:1 per bordi dei controlli, icone e indicatore del focus;
- un solo colore d'accento;
- una scala tipografica fissa e una scala di spaziature, raggi ed elevazioni definita come token;
- un unico insieme di icone.

Le animazioni SHALL durare al massimo 200 ms e SHALL essere ridotte quando il sistema chiede movimento ridotto. Ogni pagina di modulo SHALL avere un'intestazione con titolo uguale al nome della voce di navigazione.

#### Scenario: Cambio tema
- **WHEN** l'utente seleziona il tema chiaro
- **THEN** l'intera interfaccia passa al tema chiaro senza riavvio

#### Scenario: Tema del sistema
- **WHEN** il tema è "Come il sistema" e il sistema operativo passa al tema scuro
- **THEN** l'app passa al tema scuro senza intervento dell'utente

#### Scenario: Titolo coerente
- **WHEN** l'utente apre la voce di navigazione "Importa"
- **THEN** la pagina ha come titolo "Importa"

### Requirement: Primo avvio guidato
Al primo avvio l'app SHALL guidare l'utente in tre passi:
1. benvenuto, con lingua e tema;
2. scelta della prima cartella di foto, con selettore di cartelle e trascinamento;
3. preparazione delle funzioni AI. Mostra il riepilogo del sistema in frasi comprensibili, ad esempio "Il computer ha 16 GB di memoria: profilo consigliato Bilanciato", e avvia il download dei modelli in background.

Ogni passo SHALL poter essere saltato. La verifica del sistema SHALL avvenire comunque in background. Al termine l'app SHALL aprire la Libreria, che mostra le foto man mano che vengono importate e la lista "Primi passi". L'introduzione SHALL poter essere rivista dal menu Aiuto.

#### Scenario: Download rimandato
- **WHEN** l'utente salta il passo delle funzioni AI al primo avvio
- **THEN** l'app si apre con le funzioni AI disattivate e un avviso che permette di avviare il download

#### Scenario: Fine dell'introduzione
- **WHEN** l'utente sceglie la cartella "D:\Foto" e completa l'introduzione
- **THEN** si apre la Libreria con le foto che compaiono man mano che l'importazione procede, e l'indicatore delle attività mostra l'avanzamento

#### Scenario: Rivedere l'introduzione
- **WHEN** l'utente sceglie Aiuto › "Rivedi l'introduzione"
- **THEN** l'introduzione si riapre senza cambiare libreria né impostazioni finché l'utente non conferma un passo

### Requirement: Impostazioni
Le impostazioni SHALL essere organizzate in sezioni, con una sotto-navigazione e una ricerca:
- **Generale**: tema, lingua, azione del doppio clic, percorso dei dati dell'app;
- **Libreria e importazione**: nomi di cartella da ignorare per il luogo, cartelle e formati predefiniti;
- **Riconoscimento AI**: attivazione del riconoscimento volti, sensibilità per gruppo e categoria, persone e volti, somiglianza tra volti e tra foto, tassonomia;
- **Modelli AI e sistema**: verifica del sistema, modelli, profilo risorse e limite RAM, provider di esecuzione;
- **Applicazioni esterne**;
- **Diagnostica**: livello di log, errori, log, pacchetto diagnostico;
- **Avanzate**: valori numerici delle soglie, eliminazione dei dati dei volti.

Ogni impostazione SHALL esistere in un solo punto.

Le preferenze SHALL essere salvate appena modificate, con conferma visibile accanto al campo. Le impostazioni che ricalcolano dati della libreria (soglie, tassonomia, percorso dei dati) SHALL essere applicate con un comando "Applica" e un'anteprima dell'effetto.

Le soglie SHALL essere presentate come sensibilità, su una scala da "Più risultati" a "Più precisi", e il valore numerico SHALL essere visibile in "Avanzate".

#### Scenario: Modifica profilo
- **WHEN** l'utente cambia il profilo risorse da "Bilanciato" a "Leggero"
- **THEN** l'impostazione è salvata subito e i job successivi usano il nuovo budget di memoria

#### Scenario: Soglie di gruppo preservate
- **WHEN** l'utente ha impostato soglie diverse per i gruppi della tassonomia e modifica un'altra impostazione
- **THEN** le soglie dei gruppi restano quelle impostate dall'utente

#### Scenario: Ricerca di un'impostazione
- **WHEN** l'utente scrive "log" nella ricerca delle impostazioni
- **THEN** compare la sezione Diagnostica con il campo "Livello di log" evidenziato

#### Scenario: Soglia con anteprima
- **WHEN** l'utente sposta la sensibilità del rilevamento persone verso "Più precisi"
- **THEN** la pagina mostra quante foto cambierebbero e applica la modifica solo con "Applica"

#### Scenario: Nessun duplicato
- **WHEN** l'utente cerca il provider di esecuzione
- **THEN** lo trova solo in "Modelli AI e sistema"

### Requirement: Localizzazione
Tutti i testi dell'interfaccia SHALL provenire da risorse traducibili; la prima versione SHALL essere disponibile in italiano e in inglese.

#### Scenario: Cambio lingua
- **WHEN** l'utente imposta la lingua inglese
- **THEN** l'interfaccia è mostrata in inglese

### Requirement: Pubblicazione dell'installer Windows
La pipeline di distribuzione SHALL compilare l'eseguibile/setup Windows e, su un tag di versione corrispondente alla versione dell'app, SHALL creare automaticamente una GitHub Release con installer EXE e checksum SHA-256. I test completi SHALL essere eseguiti prevalentemente in locale; i normali push SHALL usare una build Windows con cache senza pubblicare release.

#### Scenario: Tag di versione
- **WHEN** viene pubblicato un tag `v<versione>` valido
- **THEN** la pipeline crea una release della stessa versione con installer EXE scaricabile e relativo checksum, senza fotografie né dati personali dell'utente

### Requirement: Esecuzione su Linux
L'applicazione SHALL avviarsi e funzionare su Linux x86_64 con un desktop grafico (Wayland o X11), sia in sviluppo sia dalla build distribuita, con le stesse funzioni di Windows salvo quelle dichiarate specifiche di una piattaforma.

#### Scenario: Avvio in sviluppo
- **WHEN** su Linux, con le dipendenze di sistema documentate installate, lo sviluppatore esegue l'avvio locale in debug
- **THEN** la finestra principale si apre e diventa interattiva senza variabili d'ambiente aggiuntive

#### Scenario: Avvio dall'archivio
- **WHEN** l'utente estrae l'archivio Linux di una release e avvia l'eseguibile
- **THEN** l'app si apre, trova le proprie risorse (modelli, runtime, dati di riferimento) e usa la cartella dati utente standard di Linux

#### Scenario: Installazione del pacchetto deb
- **WHEN** l'utente installa il pacchetto deb di una release su Ubuntu o Debian
- **THEN** le dipendenze di sistema necessarie vengono installate dal gestore pacchetti, photo-cat compare nel menu delle applicazioni e si avvia trovando le proprie risorse

### Requirement: Verifica Linux nella pipeline
Ogni push e pull request SHALL compilare e verificare l'app anche su Linux, con controllo di formattazione, analisi statica, test Rust e build di release, oltre alla build Windows già prevista.

#### Scenario: Regressione solo su Linux
- **WHEN** una modifica compila su Windows ma non su Linux
- **THEN** la pipeline fallisce sul job Linux

### Requirement: Pacchetti Linux nelle release
Su un tag di versione valido, la release SHALL includere, oltre all'installer Windows, un pacchetto deb per Linux x64 e un archivio Linux x64 con l'eseguibile e le risorse necessarie, ciascuno con il relativo checksum SHA-256, senza fotografie né dati personali.

#### Scenario: Tag di versione
- **WHEN** viene pubblicato un tag `v<versione>` valido
- **THEN** la release contiene `photo-cat_<versione>_amd64.deb` e `photo-cat-<versione>-linux-x64.tar.gz`, ciascuno con un file `.sha256` verificabile con `sha256sum -c`

### Requirement: Menu contestuale della griglia
Il clic destro su una miniatura SHALL aprire un menu contestuale che agisce sulla selezione corrente (o sulla sola miniatura se non selezionata) con almeno: "Apri con ▸" (applicazioni esterne configurate e app predefinita del sistema), "Mostra nella cartella", "Imposta luogo…", "Aggiungi a catalogo ▸" e le azioni bulk più comuni; il menu SHALL essere accessibile da tastiera. "Aggiungi a catalogo ▸" SHALL mostrare un elenco scorrevole con ricerca dei cataloghi manuali dell'utente e "Nuovo catalogo…".

#### Scenario: Clic destro su miniatura non selezionata
- **WHEN** sono selezionate 3 foto e l'utente fa clic destro su una quarta foto non selezionata
- **THEN** la selezione diventa la sola quarta foto e il menu agisce su di essa

#### Scenario: Clic destro sulla selezione
- **WHEN** sono selezionate 20 foto e l'utente fa clic destro su una di esse
- **THEN** il menu indica che agisce su 20 foto

#### Scenario: Apertura da tastiera
- **WHEN** una miniatura ha il focus e l'utente preme il tasto Menu o Maiusc+F10
- **THEN** si apre il menu contestuale navigabile con le frecce

#### Scenario: Aggiunta a un catalogo dal menu
- **WHEN** sono selezionate 20 foto, l'utente apre il menu contestuale, sceglie "Aggiungi a catalogo ▸", scrive "ma" e sceglie "Mare"
- **THEN** le 20 foto vengono aggiunte a "Mare" con un avviso con "Annulla operazione"

#### Scenario: Molti cataloghi
- **WHEN** l'utente ha 60 cataloghi manuali e apre "Aggiungi a catalogo ▸"
- **THEN** l'elenco mostra tutti i cataloghi manuali in ordine alfabetico, scorre entro l'altezza del menu e i cataloghi smart e di sistema non compaiono

#### Scenario: Nuovo catalogo dal menu
- **WHEN** l'utente sceglie "Aggiungi a catalogo ▸" › "Nuovo catalogo…" e scrive "Compleanno"
- **THEN** viene creato il catalogo manuale "Compleanno" con le foto su cui agisce il menu

### Requirement: Pannelli flottanti
L'app SHALL poter mostrare contenuti in pannelli flottanti non modali sopra l'interfaccia: trascinabili dalla barra del titolo, ridimensionabili, chiudibili con un pulsante o con Esc quando hanno il focus, sempre interamente raggiungibili dentro la finestra. Mentre un pannello è aperto l'utente SHALL poter continuare a usare il resto dell'interfaccia. Posizione e dimensione SHALL essere ricordate tra un'apertura e l'altra e tra i riavvii.

#### Scenario: Elenco luoghi in un pannello flottante
- **WHEN** l'utente sceglie "Apri in finestra" sulla lista Luoghi
- **THEN** si apre un pannello flottante con l'elenco completo dei luoghi, e l'utente può selezionare foto nella griglia senza chiuderlo

#### Scenario: Aggiornamento col filtro
- **WHEN** il pannello dei luoghi è aperto e l'utente attiva il filtro "Anno 2023" dalla barra laterale
- **THEN** il pannello mostra i luoghi e i conteggi delle sole foto del 2023

#### Scenario: Più pannelli
- **WHEN** l'utente apre il pannello dei luoghi e quello dei tag
- **THEN** i due pannelli sono visibili insieme e quello cliccato per ultimo è in primo piano

#### Scenario: Posizione ricordata
- **WHEN** l'utente sposta e ridimensiona il pannello dei luoghi, lo chiude e lo riapre dopo un riavvio
- **THEN** il pannello ricompare con la stessa posizione e dimensione

#### Scenario: Finestra rimpicciolita
- **WHEN** la finestra dell'app viene ridotta e un pannello aperto resterebbe in parte fuori
- **THEN** il pannello viene riposizionato e, se serve, ridotto per restare interamente visibile

#### Scenario: Uso da tastiera
- **WHEN** il pannello ha il focus e l'utente preme Esc
- **THEN** il pannello si chiude e il focus torna al comando che lo aveva aperto

### Requirement: Revisione delle persone
Il modulo Persone SHALL presentare un elenco diviso nelle sezioni "Da rivedere", "Persone" e "Ignorati" con conteggi e ricerca, i volti dell'elemento scelto in una griglia con selezione multipla, e una scheda con "Chi è?", unione e ignora. L'elenco e la griglia SHALL restare fluidi con 100.000 volti, caricando i dati a pagine, e SHALL aggiornarsi quando cambiano i dati dei volti senza ricaricamenti periodici. Tutte le azioni SHALL essere disponibili da tastiera.

#### Scenario: Revisione da tastiera
- **WHEN** l'utente scorre "Da rivedere" con le frecce, preme N, scrive un nome e conferma
- **THEN** il gruppo diventa una persona con quel nome e il focus passa al gruppo successivo da rivedere

#### Scenario: Libreria grande
- **WHEN** la libreria contiene 100.000 volti in 3.000 gruppi
- **THEN** l'elenco e la griglia dei volti si aprono e scorrono senza blocchi percepibili

#### Scenario: Aggiornamento dopo l'analisi
- **WHEN** termina un'analisi dei volti mentre il modulo Persone è aperto
- **THEN** elenco e conteggi si aggiornano senza perdere la selezione e la posizione di scorrimento

### Requirement: Preferenze conservate tra installazioni
Installare una nuova versione sopra una esistente, oppure disinstallare e reinstallare, SHALL conservare impostazioni, libreria, identità, set di tag e preferenze dell'interfaccia (tema, lingua, modulo attivo, pannelli laterali, ordinamento e dimensione delle miniature, posizione dei pannelli flottanti), anche quando la disinstallazione elimina i dati del profilo dell'interfaccia. I dati SHALL essere rimossi solo cancellando a mano la cartella dell'app.

#### Scenario: Aggiornamento sopra la versione installata
- **WHEN** l'utente installa la nuova versione sopra quella esistente
- **THEN** al primo avvio libreria, impostazioni e preferenze dell'interfaccia sono quelle di prima

#### Scenario: Disinstallazione con eliminazione dei dati dell'app
- **WHEN** l'utente aveva ordinato la griglia per nome e ridimensionato la barra laterale, poi disinstalla scegliendo "Elimina i dati dell'applicazione" e reinstalla
- **THEN** al primo avvio la griglia è ordinata per nome, la barra laterale ha la larghezza di prima e la libreria è intatta

#### Scenario: Preferenze della versione precedente
- **WHEN** l'utente avvia per la prima volta la nuova versione dopo aver usato una versione che non salvava le preferenze dell'interfaccia nel file
- **THEN** le preferenze in uso vengono salvate subito e sopravvivono a una successiva disinstallazione

### Requirement: Recupero dalla chiusura dell'interfaccia
Se il processo dell'interfaccia termina mentre l'app è aperta, il sistema SHALL lasciare proseguire o mettere in pausa in sicurezza i job in corso, ricaricare l'interfaccia e mostrare all'utente che cosa è successo, senza perdere dati del catalogo.

#### Scenario: Interfaccia ricaricata
- **WHEN** la WebView termina durante un import
- **THEN** l'interfaccia viene ricaricata, mostra un avviso che indica la terminazione e il collegamento alla diagnostica, e il job di import risulta ancora in corso o in pausa, mai perso

#### Scenario: Terminazioni ripetute
- **WHEN** l'interfaccia termina tre volte in pochi minuti
- **THEN** il sistema mette in pausa i job, smette di ricaricare automaticamente e mostra una finestra di errore con il percorso dei log e il comando per creare il pacchetto diagnostico

### Requirement: Barra delle azioni sulla selezione
Quando in Libreria, in un catalogo o in Sequenza è selezionata almeno una foto, la barra degli strumenti della vista SHALL essere sostituita da una barra delle azioni. La barra contiene:
- il numero di foto selezionate;
- "Deseleziona" (Esc);
- comandi diretti per Tag, catalogo (Aggiungi o Escludi), Persona, Luogo, Apri con ed Esporta;
- un menu "Altro" con le azioni restanti.

Senza selezione la barra MUST NOT essere visibile.

#### Scenario: Selezione di foto
- **WHEN** l'utente seleziona 3 foto in Libreria
- **THEN** al posto della barra degli strumenti compare "3 selezionate" con i comandi diretti, e premendo Esc la selezione si svuota e torna la barra degli strumenti

#### Scenario: Azione che cambia modulo
- **WHEN** l'utente sceglie "Esporta" con 40 foto selezionate
- **THEN** si apre il modulo Esporta con ambito "Selezione (40 foto)" già impostato

### Requirement: Selettori delle azioni sulla selezione
Ogni comando della barra delle azioni che richiede un valore SHALL aprire accanto al comando un selettore con ricerca, utilizzabile da tastiera. Quando il tipo di valore lo consente, il selettore SHALL permettere anche di creare un valore nuovo.

#### Scenario: Aggiunta di un tag
- **WHEN** l'utente fa clic su "Tag" nella barra delle azioni, scrive "mare" e preme Invio
- **THEN** il tag "mare" è aggiunto alle foto selezionate e compare un avviso con "Annulla operazione"

### Requirement: Visualizzatore delle foto
In Libreria, nei cataloghi e in Sequenza, Spazio o Invio su una miniatura con il focus SHALL aprire la foto in un visualizzatore interno. Il doppio clic SHALL eseguire l'azione configurata in Impostazioni (vedi `external-apps`, "Azione del doppio clic").

#### Scenario: Doppio clic configurato
- **WHEN** l'azione del doppio clic è "app predefinita del sistema" e l'utente fa doppio clic su una miniatura
- **THEN** la foto si apre nell'app predefinita del sistema

### Requirement: Comandi del visualizzatore
Il visualizzatore delle foto SHALL offrire:
- le frecce per scorrere le foto nell'ordine della vista;
- i tasti 1–9 e 0 per i tag del set attivo;
- I per le informazioni;
- Esc o Spazio per tornare alla griglia sulla stessa foto.

#### Scenario: Revisione rapida
- **WHEN** l'utente preme Spazio su una foto, poi → tre volte e 2
- **THEN** il visualizzatore mostra la quarta foto della vista, le assegna il tag del tasto 2 e, alla chiusura con Esc, la griglia ha il focus sulla quarta foto

### Requirement: Navigazione da tastiera nella griglia
Le griglie di Libreria, cataloghi, Sequenza e Persone SHALL essere navigabili con:
- frecce, Home/Fine e Pagina su/giù per spostare il focus;
- Maiusc con una freccia per estendere la selezione;
- Ctrl+Spazio per aggiungere o togliere la foto con il focus dalla selezione;
- Ctrl+A per selezionare tutte le foto della vista.

La miniatura con il focus SHALL essere sempre visibile e distinta da quelle selezionate.

#### Scenario: Selezione da tastiera
- **WHEN** l'utente porta il focus su una miniatura e preme Maiusc+→ due volte
- **THEN** sono selezionate tre foto consecutive e il focus è sulla terza

#### Scenario: Seleziona tutto
- **WHEN** la vista ha un filtro con 8.000 foto e l'utente preme Ctrl+A
- **THEN** sono selezionate tutte le 8.000 foto del filtro, senza caricarle tutte nell'interfaccia

### Requirement: Contesto della vista in Libreria
La Libreria SHALL mostrare in un'unica barra:
- dove si trova l'utente, come percorso, ad esempio "Tutte le foto › Catalogo Viaggio Toscana › Escluse";
- il numero di foto della vista;
- i filtri attivi come chip rimovibili;
- un menu "Ordina";
- un menu "Vista", con dimensione delle miniature e raggruppamento delle foto simili.

I filtri, compreso "Solo foto con simili", SHALL stare nel pannello Filtri e SHALL generare un chip quando sono attivi.

#### Scenario: Catalogo aperto
- **WHEN** l'utente apre il catalogo "Viaggio Toscana" dalla barra laterale
- **THEN** la barra mostra "Tutte le foto › Viaggio Toscana · 312 foto" e il comando "Escluse (6)", e facendo clic su "Tutte le foto" si torna alla libreria completa

#### Scenario: Filtro delle foto simili
- **WHEN** l'utente attiva "Solo foto con simili" nel pannello Filtri
- **THEN** nella barra compare il chip "Con foto simili" che lo rimuove con un clic
