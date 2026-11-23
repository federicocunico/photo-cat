# Glossario dell'interfaccia

Termini da usare in tutte le stringhe, in italiano (lingua di riferimento) e in inglese.
I termini vietati sono controllati da `glossary.test.ts`.

| Concetto                              | Italiano                                                                           | English                                                           | Da non usare                              |
| ------------------------------------- | ---------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------- |
| Lavoro in background                  | **Attività** ("Importazione di D:\Foto")                                           | **Activity** ("Importing D:\Photos")                              | job, `#12 · import`                       |
| Chiudere un dialogo senza fare nulla  | **Annulla** (`common.cancel`)                                                      | **Cancel**                                                        | "Annulla" per fermare o disfare           |
| Fermare un'attività                   | **Interrompi** (`common.stop`)                                                     | **Stop**                                                          | Annulla / Cancel                          |
| Tornare indietro su un'operazione     | **Annulla operazione** (`common.undo`)                                             | **Undo**                                                          | "Annulla ultima azione"                   |
| Togliere da un contenitore            | **Rimuovi da …**, **Escludi da …** (cataloghi)                                     | **Remove from …**, **Exclude from …**                             |                                           |
| Cancellare dati dell'app              | **Elimina**, dicendo che cosa resta sul disco                                      | **Delete**                                                        | Rimuovi, per i dati dell'app              |
| Aggiungere foto                       | **Importa**; **cartella**                                                          | **Import**; **folder**                                            | Import, sorgente, "Aggiungi una sorgente" |
| Portare foto fuori dal catalogo       | **Esporta** ("Esporta dati", "Organizza in cartelle", "Sequenza")                  | **Export**                                                        | "Esporta e organizza"                     |
| Valore 0–1 di affidabilità            | **Sensibilità**, scala "Più risultati ↔ Più precisi"                               | **Sensitivity**, "More results ↔ More precise"                    | soglia, confidenza, punteggi senza nome   |
| Voce della tassonomia                 | **Categoria**                                                                      | **Category**                                                      | voce, etichetta                           |
| Catalogo con regole                   | **Smart** ("aggiornato dalle regole")                                              | **Smart**                                                         | dinamico                                  |
| Giudizio da 0 a 5 stelle              | **Valutazione** ("almeno 4 stelle")                                                | **Rating**                                                        | voto, punteggio                           |
| Aspetto cromatico della foto          | **Colore**: Bianco e nero, Colore, **Colore virato**, **Seppia**, **Non indicato** | **Colour**: Black and white, Colour, Toned colour, Sepia, Not set | color_trait, tono                         |
| Foto con data di solo anno            | **Anno senza mese**                                                                | **Year without month**                                            | mese 00                                   |
| Chi compare nelle foto, con un nome   | **Persona** ("Nuova persona", "Elimina persona…")                                  | **Person**, **People**                                            | identità                                  |
| Gruppo di volti senza nome            | **Persona senza nome** ("Da rivedere")                                             | **Unnamed person**                                                | identità senza nome                       |
| Raggruppamento di persone             | **Cerchia** ("Famiglia", "Lavoro")                                                 | **Circle**                                                        | Gruppo, come campo della persona          |
| Altri modi di chiamare una persona    | **Altri nomi**                                                                     | **Other names**                                                   | alias                                     |
| Portare volti in un'altra persona     | **Sposta in…** ("Volto spostato in «Giulia»")                                      | **Move to…**                                                      | Riassegna, Assegna a                      |
| Fare di due gruppi o persone uno solo | **Unisci a…** ("Unisci a «Giulia»")                                                | **Merge into…**                                                   | Unisci in, Unisci con                     |

Altre etichette comuni (`common.*`): Salva / Save, Chiudi / Close, Riprova / Retry, Dettagli / Details, Applica / Apply, Ripristina / Reset.

## Gergo tecnico

Non compare nell'interfaccia, salvo nelle sezioni "Avanzate" e "Diagnostica", dove si spiega:
codici degli operatori (`eq`, `gte`, `people_count`), Self-test, SHA-256, glob, SSIM, embedding, id numerici.

## Date e numeri

Date, ore e numeri passano per `lib/format.ts` (`fmtDate`, `fmtDateTime`, `fmtTime`, `fmtPeriod`, `fmtInt`), che usa `Intl` nella lingua dell'app e non in quella del sistema. Non usare `toLocaleString` senza lingua.
