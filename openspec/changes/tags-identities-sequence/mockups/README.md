# Mockup del change

Schermate di riferimento per i task UI di `tasks.md`, prodotte con il canvas di design "photo-cat UI mockups" (stesso stile e token di `app/ui/src/theme/tokens.ts`, tema scuro).

| File | Schermata | Task |
|---|---|---|
| `Tags.html` | Tag e set di tag | 2.4, 2.5 |
| `Identities.html` | Identità (configuratore dell'app) | 3.6 |
| `Sequence.html` | Sequenza, orari e tagging rapido | 6.2 |
| `SequenceExport.html` | Export con naming convention | 7.6 |

## Come aprirli

Aprire i file `.html` direttamente nel browser: sono HTML statico, senza script. Serve la connessione solo per i font (Source Sans 3, JetBrains Mono da Google Fonts); senza, si vedono i font di sistema. I link verso schermate non incluse qui (Libreria, Persone, ecc.) non portano da nessuna parte: quelle schermate esistono già nell'app.

`src/*.dc.html` sono i sorgenti del canvas (formato Design Component: template con `{{…}}`, `<sc-for>`, `<sc-if>` e dati di esempio nella classe `Component`). Servono solo per rigenerare gli HTML o reimportarli nel canvas; non si aprono da soli nel browser.

## Cosa vale e cosa no

- **Vincolanti**: struttura delle schermate, contenuti dei pannelli, testi, stati (selezione, avvisi, conferme), comportamenti da tastiera indicati in barra di stato.
- **Indicativi**: dati di esempio (nomi, conteggi, orari), misure esatte. Colori e spaziature vanno presi dai token dell'app, non copiati dagli stili inline.
- Nella navigazione dei mockup compaiono i nuovi moduli Sequenza, Tag e Identità; l'ordine definitivo delle voci si decide in `modules/registry.ts`.

## Rigenerazione

Gli HTML sono stati prodotti dai sorgenti con un pre-render basato su `jsdom` (già presente in `app/ui/node_modules`) che risolve segnaposto, cicli e condizioni usando i dati di `renderVals()`. Se si modificano i sorgenti, aggiornare il canvas e rigenerare gli HTML allo stesso modo.
