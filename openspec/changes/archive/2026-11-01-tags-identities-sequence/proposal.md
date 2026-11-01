# Proposal

## Why

Oggi i tag di photo-cat sono stringhe libere digitate a mano nell'Inspector o nella barra bulk: non c'è un posto dove vederli, rinominarli, unirli o organizzarli, né un modo rapido per assegnarli da tastiera. Le persone, poi, esistono solo dentro `catalog.db`: cambiando cartella dati o ricominciando una libreria si perdono nomi e riconoscimento, e bisogna ribattezzare tutti da capo.

In parallelo, il flusso di [photo-tagging-organizer](https://github.com/federicocunico/photo-tagging-organizer) (PhotoSeq) risolve un problema che photo-cat non copre: unire foto di sorgenti diverse (fotocamera, telefono, file ricevuti) in un'unica sequenza cronologica corretta, taggarle velocemente ed esportarle rinominate con una convenzione di nomi. Oggi la data di scatto in libreria è un'ora locale senza fuso (`media_dates.captured_at`), quindi l'ordinamento mescola scatti di fusi diversi e non c'è modo di correggere un orologio sbagliato.

## What Changes

- **Gestione dei tag**: nuovo modulo "Tag" con l'elenco di tutti i tag (conteggi, rinomina, unione, eliminazione, colore) e i **set di tag**: gruppi ordinati con tasti rapidi 1–9, eventualmente **esclusivi** (un solo tag del set per foto). Il set **Base** è predefinito e vale per tutte le librerie. Il set attivo assegna i tag da tastiera in Libreria e in Sequenza (0 rimuove).
- **Identità a livello di app**: nuovo modulo "Identità", un configuratore di persone note (nome, nome breve per i file, alias, gruppo, volti di riferimento con embedding, soglia di somiglianza) salvato in un archivio dell'app separato da `catalog.db`. Sopravvive a cambio della cartella dati e a nuove librerie, si può esportare e importare. In ogni libreria i gruppi di volti si collegano alle identità e i nuovi volti vengono riconosciuti anche dai volti di riferimento.
- **Istante reale e riconciliazione degli orari**: all'importazione si legge anche il fuso dichiarato (`OffsetTimeOriginal` e simili) e si calcola l'istante UTC. Le foto vengono raggruppate per sorgente (dispositivo + fuso); per ogni gruppo l'utente può impostare l'ora locale mostrata e uno scostamento dell'orologio, allineare due foto dello stesso momento per ricavare lo scostamento, e accettare i suggerimenti basati sul GPS. Le foto senza data si collocano trascinandole nella sequenza. Le correzioni vivono nel DB; **gli originali non vengono mai modificati**.
- **Sequenza**: nuovo modulo che mostra un ambito (libreria, catalogo, selezione) in ordine di istante reale, diviso in sessioni, con una striscia colorata per dispositivo e tagging rapido col set attivo.
- **Export della sequenza con naming convention**: modello di nome con token (`{seq}`, `{seq:04d}`, `{date}`, `{time}`, `{time_full}`, `{datetime}`, `{tag}`, `{device}`, `{people}`, `{place}`, `{orig}`) e anteprima live; cartella `sequenza/`, cartelle `per-tag/<Tag>/` con hard link, `manifest.csv`, due zip non compressi. Conversione JPEG verificata (qualità 97, 4:4:4 di default, controllo SSIM/PSNR con ricodifica fino a 100), profilo ICC preservato, orientamento applicato, metadati riscritti nelle copie (istante corretto, tag come parole chiave XMP, identità). Verifica preliminare dello spazio. Eseguito come job annullabile.
- **Persone**: "Chi è?" nella schermata Persone propone e crea identità invece di nomi liberi; il nome di una persona di libreria è quello dell'identità collegata.

## Capabilities

### New Capabilities

- `tag-management`: tag della libreria come entità gestibili (rinomina, unione, eliminazione, colore), set di tag con tasti rapidi ed esclusività, set Base a livello di app, tagging da tastiera.
- `identity-registry`: archivio delle identità a livello di app, indipendente dalla libreria; volti di riferimento, collegamento con le persone di ogni libreria, esportazione e importazione.
- `capture-timeline`: istante reale di scatto, gruppi di sorgente, correzione di fuso e scostamento, allineamento tra sorgenti, collocazione delle foto senza data, vista Sequenza ordinata per sessioni.
- `sequence-export`: export della sequenza rinominata con modello di nome, struttura sequenza/per-tag, manifest, zip, conversione JPEG verificata e metadati riscritti.

### Modified Capabilities

- `catalog-library`: il requisito "Tag utente" si estende a tag con identità propria (rinomina/unione preservano le assegnazioni); filtri e cataloghi smart possono usare l'istante reale.
- `face-recognition`: "Identificazione delle persone" passa dalle persone di libreria alle identità dell'app; i volti di riferimento partecipano all'associazione automatica; la cancellazione dei dati dei volti include i volti di riferimento.
- `media-ingestion`: la lettura dei metadati include fuso dichiarato e dispositivo, per calcolare l'istante reale.
- `bulk-operations`: tagging da tastiera col set attivo e rispetto dell'esclusività del set nelle azioni bulk sui tag.

## Impact

- **Codice Rust**: `pc-store` (migrazione schema v5: colore e set di tag, istante e gruppi di sorgente, correzioni orarie; nuovo archivio identità), `pc-ingest/src/metadata.rs` (fuso e dispositivo), `pc-ai` (associazione volti con riferimenti delle identità), `pc-organize` (nuovo export di sequenza), `pc-core/src/paths.rs` (percorso dell'archivio identità nella cartella di configurazione), `app/src-tauri` (nuovi comandi IPC).
- **UI**: nuovi moduli `tags`, `identities`, `sequence` (con export) in `app/ui/src/modules`, voce in `registry.ts` e `store/layout.ts`; modifiche a `PeoplePage`, `Inspector`, `BulkBar`, i18n it/en.
- **Dipendenze**: encoder JPEG con controllo della crominanza e incorporamento ICC (da scegliere nel design), scrittura EXIF/XMP sulle copie, `zip` per gli archivi; nessuna dipendenza di rete.
- **Dati**: nuovo file `identities.db` nella cartella di configurazione dell'utente (contiene dati biometrici, rimane locale). Migrazione automatica di `catalog.db`; le persone già nominate vengono convertite in identità alla prima apertura.
- **Mockup**: in `mockups/` di questo change (HTML statico e sorgenti del canvas).
