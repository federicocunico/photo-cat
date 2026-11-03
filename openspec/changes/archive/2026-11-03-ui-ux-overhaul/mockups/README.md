# Mockup del change

Il canvas di design "photo-cat — revisione UI/UX" (https://claude.ai/artifact/FoQtdU21aWGGp1dZJK3aHz, privato finché non viene condiviso) contiene le schermate di riferimento. In `project/` c'è la copia dei sorgenti: un file `.dc.html` per ogni tavola, più `canvas.json`. Ogni tavola ha l'opzione "tema" (scuro/chiaro). I colori sono quelli di `app/ui/src/theme/tokens.ts`.

| Tavola | Contenuto | Task |
|---|---|---|
| `Main.dc.html` | Shell nuova: navigazione a gruppi, barra superiore con palette e Aiuto, barra laterale a sezioni richiudibili, barra di contesto (percorso, conteggio, chip, Ordina, Vista), "Primi passi", griglia con gruppo di simili e focus, Inspector riordinato, barra di stato | 2.x, 3.x |
| `Selection.dc.html` | Barra delle azioni sulla selezione, selettore dei tag con ricerca, Inspector con selezione multipla, avvisi impilati con "Annulla operazione" | 3.2–3.4 |
| `Palette.dc.html` | Palette Ctrl+K con risultati raggruppati e foglio delle scorciatoie | 4.x |
| `Settings.dc.html` | Impostazioni a sezioni con ricerca, sensibilità su scala e anteprima con "Applica", salvataggio immediato | 6.x |
| `Onboarding.dc.html` | Primo avvio in tre passi (passo 2: cartella, riepilogo del sistema in frasi) | 5.1 |
| `Rules.dc.html` | Elenco dei cataloghi con conteggi ed escluse; editor smart con condizioni leggibili e riepilogo | 7.2 |
| `Components.dc.html` | Token (tipografia, spaziature, raggi, movimento), pulsanti, comando disabilitato spiegato, conferma su disco, stato vuoto, errore sul campo, navigazione compatta, suggerimento al primo uso, glossario | 1.x |

## Cosa vale e cosa no

**Vincolanti**:
- struttura delle schermate e posizione dei comandi;
- gruppi e ordine della navigazione;
- testi dei comandi e dei messaggi;
- glossario;
- scorciatoie;
- regole di conferma e annullamento.

**Indicativi**:
- dati di esempio;
- misure esatte;
- icone, che sono segnaposto in stile lineare, da sostituire con il set scelto (vedi `design.md`);
- miniature, che sono riquadri segnaposto.

Colori e spaziature si prendono dai token dell'app, non dagli stili dei file.

Il modulo Persone non è ridisegnato qui: vale il mockup di `people-faces-workspace`, adattato solo ai componenti condivisi.
