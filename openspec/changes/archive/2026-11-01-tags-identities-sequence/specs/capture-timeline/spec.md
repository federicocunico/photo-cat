# Spec Delta

## Purpose

Ricostruisce l'ordine cronologico reale di foto provenienti da sorgenti diverse (fotocamera, telefono, file ricevuti), permettendo di correggere fusi e orologi sbagliati senza modificare gli originali.

## ADDED Requirements

### Requirement: Istante reale di scatto
Per ogni foto il sistema SHALL calcolare l'istante di scatto in UTC dalla data di acquisizione e dal fuso dichiarato nei metadati; quando il fuso manca SHALL usare il fuso del gruppo di sorgente e indicare che l'istante è presunto.

#### Scenario: Fuso dichiarato
- **WHEN** una foto ha data originale 2026-08-19 10:43:00 e fuso -06:00
- **THEN** l'istante è 2026-08-19 16:43:00 UTC e non è presunto

#### Scenario: Fuso assente
- **WHEN** una foto JPG ricevuta non ha fuso e il suo gruppo usa Europe/Rome
- **THEN** l'istante è calcolato col fuso di Roma valido in quella data ed è marcato presunto

### Requirement: Gruppi di sorgente
Il sistema SHALL raggruppare le foto di un ambito per sorgente, definita da dispositivo (marca e modello) e fuso dichiarato; le foto senza dispositivo SHALL formare il gruppo "Altre sorgenti". Ogni gruppo mostra il numero di foto.

#### Scenario: Fotocamera con due fusi
- **WHEN** un ambito contiene 64 scatti Fujifilm X-T5 con fuso +02:00 e 310 con fuso -06:00
- **THEN** compaiono due gruppi distinti per la X-T5 con 64 e 310 foto

### Requirement: Ora locale di un gruppo
L'utente SHALL poter scegliere per un gruppo di sorgente il fuso dell'ora locale; il cambio modifica solo l'ora mostrata e quella usata nei nomi di export, non l'istante né l'ordine.

#### Scenario: Fotocamera rimasta sull'ora italiana
- **WHEN** l'utente imposta -07:00 come ora locale del gruppo X-T5 +02:00
- **THEN** uno scatto delle 10:43 +02:00 è mostrato alle 01:43 e la sua posizione nella sequenza non cambia

### Requirement: Scostamento dell'orologio
L'utente SHALL poter applicare a un gruppo di sorgente uno scostamento positivo o negativo con precisione al secondo, che sposta l'istante di tutte le foto del gruppo; l'anteprima SHALL mostrare l'ora prima e dopo la correzione.

#### Scenario: Orologio indietro
- **WHEN** l'utente applica +2 min 10 s al gruppo X-T5 -06:00
- **THEN** uno scatto delle 10:41:50 diventa delle 10:44:00 e la sequenza viene riordinata

### Requirement: Allineamento tra due sorgenti
L'utente SHALL poter scegliere due foto di gruppi diversi che ritraggono lo stesso momento; il sistema SHALL proporre per il secondo gruppo lo scostamento che rende uguali i due istanti.

#### Scenario: Stesso momento da due dispositivi
- **WHEN** l'utente allinea una foto iPhone delle 18:30:05 con una foto X-T5 delle 18:27:40 dello stesso istante
- **THEN** il sistema propone +2 min 25 s per il gruppo della X-T5

### Requirement: Suggerimento di fuso dal GPS
Quando le foto con coordinate GPS indicano un fuso diverso da quello dichiarato da un altro gruppo negli stessi istanti, il sistema SHALL proporre la correzione dell'ora locale del gruppo, che l'utente può applicare o ignorare.

#### Scenario: Fuso incoerente
- **WHEN** 64 scatti X-T5 dichiarano +02:00 e le foto iPhone con GPS negli stessi istanti sono in -07:00
- **THEN** il sistema propone -07:00 come ora locale del gruppo X-T5 indicando i 64 scatti

### Requirement: Correzioni persistenti e reversibili
Le correzioni di fuso, scostamento e collocazione SHALL essere salvate in libreria, applicate a ordinamento, filtri ed export, e annullabili; il sistema MUST NOT modificare i file originali per applicarle.

#### Scenario: Riavvio
- **WHEN** l'utente applica uno scostamento e riapre l'app
- **THEN** la sequenza mostra le foto nello stesso ordine corretto

#### Scenario: Originali intatti
- **WHEN** l'utente applica correzioni a 400 foto
- **THEN** i 400 file originali hanno lo stesso contenuto e la stessa data di modifica di prima

### Requirement: Foto senza data
Le foto senza data di acquisizione affidabile (fonte "file system") SHALL comparire in un'area separata della Sequenza; trascinandone una dopo una foto della sequenza, riceve un istante intermedio tra quella foto e la successiva e il loro fuso.

#### Scenario: Collocazione
- **WHEN** l'utente trascina una foto senza data dopo uno scatto delle 18:30 seguito da uno delle 18:34
- **THEN** la foto riceve un istante tra 18:30 e 18:34 ed esce dall'area delle foto senza data

### Requirement: Vista Sequenza
Il sistema SHALL mostrare un ambito (libreria, catalogo, selezione) in ordine di istante reale, diviso in sessioni separate da pause superiori a una soglia configurabile; ogni miniatura indica con un colore il dispositivo e mostra il tag del set attivo.

#### Scenario: Sessioni
- **WHEN** tra due scatti consecutivi passano più di 2 ore con la soglia predefinita
- **THEN** i due scatti appartengono a sessioni diverse, ciascuna con data, intervallo orario, luogo e numero di foto

#### Scenario: Selezione di una sessione
- **WHEN** l'utente preme A con il fuoco su una sessione
- **THEN** tutte le foto della sessione vengono selezionate
