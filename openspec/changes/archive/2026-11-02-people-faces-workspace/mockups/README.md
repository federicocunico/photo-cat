# Mockup del change

Schermata di riferimento per i task UI di `tasks.md` (gruppo 4). È HTML statico che usa gli stessi token di `app/ui/src/theme/tokens.ts`: tema scuro predefinito, tema chiaro con `prefers-color-scheme: light` o `data-theme="light"`.

| File | Contenuto | Task |
|---|---|---|
| `People.html` | Modulo Persone unico: schermata principale (gruppo da rivedere selezionato, volto evidenziato arrivando da una foto, suggerimento di unione, scheda "Chi è?") e tavola degli stati (persona con nome, dialogo di unione, gruppi probabilmente uguali, Ignorati, volti scartati per qualità, arrivo da "Apri in Persone", vuoto/caricamento, tastiera) | 4.1–4.6 |

## Come aprirlo

Aprire `People.html` direttamente nel browser: non ha script. I font (Source Sans 3, JetBrains Mono) arrivano da Google Fonts; offline si vedono i font di sistema. I volti sono sagome segnaposto disegnate in SVG.

## Cosa vale e cosa no

- **Vincolanti**:
  - struttura a tre colonne (elenco con sezioni Da rivedere / Persone / Ignorati, griglia dei volti, scheda);
  - contenuti di ciascuna colonna;
  - testi dei comandi e dei messaggi;
  - stati della tavola;
  - scorciatoie da tastiera;
  - evidenziazione del volto di arrivo (bordo di colore `warn`);
  - Identità che non è più una voce di navigazione.
- **Indicativi**: dati di esempio (nomi, conteggi, punteggi), misure esatte, l'icona a barre della qualità. Colori e spaziature vanno presi dai token e dalle classi dell'app, non copiati dagli stili del file.
