# Spec Delta

## Purpose

Mostra su una mappa del mondo, interamente offline, dove sono state scattate le foto della libreria o di un catalogo, e permette di aprire le foto di una zona e di collocare i luoghi che non hanno coordinate.

## ADDED Requirements

### Requirement: Mappa offline
Il modulo AI SHALL offrire una sezione "Mappa" con una mappa del mondo con i confini dei paesi, spostabile e ingrandibile almeno fino al livello di una regione. La mappa e i dati geografici SHALL essere inclusi nell'applicazione, e la sezione MUST NOT fare richieste di rete.

#### Scenario: Senza connessione
- **WHEN** l'utente apre la Mappa con il computer scollegato da Internet
- **THEN** la mappa del mondo e le foto vengono mostrate normalmente

#### Scenario: Nessuna richiesta di rete
- **WHEN** l'utente sposta e ingrandisce la mappa
- **THEN** l'applicazione non invia richieste di rete

### Requirement: Foto raggruppate per zona
La mappa SHALL mostrare le foto con posizione come gruppi per zona, con dimensione proporzionale al numero di foto, ricalcolati a ogni livello di zoom. Al passaggio del puntatore un gruppo SHALL mostrare il numero di foto, il luogo prevalente e alcune miniature. Il clic SHALL aprire la Libreria filtrata sull'area del gruppo.

#### Scenario: Ingrandimento
- **WHEN** la mappa mostra un gruppo di 1.200 foto sull'Italia e l'utente ingrandisce sulla Toscana
- **THEN** il gruppo si divide in gruppi più piccoli, ad esempio Firenze e Siena, con la somma delle foto invariata

#### Scenario: Apertura di una zona
- **WHEN** l'utente fa clic sul gruppo di Firenze con 340 foto
- **THEN** si apre la Libreria con il filtro per l'area di quel gruppo e 340 risultati

### Requirement: Posizioni precise e approssimate
La mappa SHALL collocare con precisione le foto con coordinate GPS. Le foto con un luogo ricavato dalla cartella o assegnato a mano SHALL essere collocate in modo approssimato alle coordinate del luogo, quando il luogo è noto al dizionario geografico offline o è stato posizionato dall'utente. I gruppi approssimati SHALL essere distinguibili e l'utente SHALL poterli nascondere.

#### Scenario: Cartella con nome di città
- **WHEN** 80 foto senza GPS hanno il luogo "Lisbona" ricavato dalla cartella
- **THEN** la mappa mostra un gruppo approssimato di 80 foto su Lisbona

#### Scenario: Solo posizioni precise
- **WHEN** l'utente disattiva "Mostra posizioni approssimate"
- **THEN** la mappa mostra solo le foto con coordinate GPS

### Requirement: Ambito e cataloghi sulla mappa
L'utente SHALL poter limitare la mappa a un catalogo. Un elenco accanto alla mappa SHALL indicare i cataloghi con foto nell'area visibile, con il numero di foto in quell'area. Il clic su un catalogo dell'elenco SHALL limitare la mappa a quel catalogo.

#### Scenario: Un catalogo
- **WHEN** l'utente sceglie il catalogo "Viaggio Giappone"
- **THEN** la mappa mostra solo le foto di quel catalogo

#### Scenario: Cataloghi nell'area
- **WHEN** l'area visibile è la Sardegna e contiene foto dei cataloghi "Estate 2023" e "Matrimonio"
- **THEN** l'elenco mostra i due cataloghi con il numero di foto di ciascuno nell'area

### Requirement: Luoghi senza coordinate
La sezione Mappa SHALL elencare i luoghi senza coordinate (ad esempio "Viaggio Giappone"), con il numero di foto. L'utente SHALL poter posizionare un luogo con un clic sulla mappa: la posizione SHALL valere per tutte le foto con quel luogo, SHALL essere modificabile e rimovibile, e MUST NOT alterare le coordinate GPS delle foto.

#### Scenario: Posizionamento manuale
- **WHEN** l'utente sceglie "Viaggio Giappone" (230 foto) dall'elenco e fa clic su Tokyo
- **THEN** il luogo esce dall'elenco e la mappa mostra un gruppo approssimato di 230 foto su Tokyo

#### Scenario: Foto con GPS nello stesso luogo
- **WHEN** alcune foto con luogo "Viaggio Giappone" hanno anche coordinate GPS
- **THEN** quelle foto restano collocate alle proprie coordinate GPS

### Requirement: Prestazioni della mappa
Con 100.000 foto con posizione, la mappa SHALL mostrare i gruppi entro 1 secondo dall'apertura e SHALL aggiornarli entro 300 millisecondi dalla fine di uno spostamento o di uno zoom.

#### Scenario: Libreria grande
- **WHEN** l'utente apre la Mappa con 100.000 foto con posizione e poi ingrandisce su un paese
- **THEN** i gruppi compaiono entro 1 secondo e si aggiornano entro 300 millisecondi dalla fine dello zoom
