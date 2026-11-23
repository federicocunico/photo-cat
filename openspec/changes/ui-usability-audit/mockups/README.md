# Mockup del change

Le tavole stanno sul canvas di design "photo-cat — audit usabilità" (https://claude.ai/artifact/SF5zFG9SDs9tL8Hgg3MZBB). Il canvas è privato finché non viene condiviso dal menu Condividi. In `project/` c'è la copia dei sorgenti: un file `.dc.html` per ogni tavola, più `canvas.json`. Le tavole sono in tema scuro, con i colori di `app/ui/src/theme/tokens.ts`. I volti sono segnaposto: nessuna foto reale.

| Tavola | Contenuto | Rif. audit |
|---|---|---|
| `Main.dc.html` | Persone › Da rivedere: schede su una riga, elenco senza intestazione affollata, intestazione dei volti con soli comandi di vista, scheda con "È Gioia?" in cima, "Chi è?" e azioni secondarie; navigazione senza il gruppo "AI"; barra dei menu | P1–P4, P6, P8, P11–P13, S2, S3 |
| `PeopleSelection.dc.html` | Volti selezionati: la barra delle azioni sostituisce l'intestazione (Sposta in…, Non è questa persona, Altro, Deseleziona); selettore "Sposta in" con ricerca, persone e gruppi senza nome con conteggi; avviso con "Annulla operazione" | P1, P2, P4, P16 |
| `PeoplePerson.dc.html` | Persone › una persona: intestazione con copertina, "Mostra in Libreria" e menu ⋯; schede "Volti nella libreria" / "Volti di riferimento"; dettagli salvati subito (nome, nome nei file, cerchia, altri nomi, riconoscimento come sensibilità) | P6, P7, P9, P10 |
| `MenuBar.dc.html` | Barra dei menu con File aperto e contenuto degli altri menu (Modifica, Vista, Vai, Strumenti, Aiuto) | S2, S1 |
| `About.dc.html` | Informazioni su photo-cat: scheda "Informazioni" (versione, build, sistema, dati, autori, licenza, Copia informazioni) e scheda "Licenze di terze parti" | S1 |

## Cosa vale e cosa no

**Vincolanti**:
- struttura delle schermate e posizione dei comandi;
- comandi visibili e comandi che vanno in "Altro" o in "⋯";
- testi dei comandi, voci dei menu e glossario;
- contenuto della finestra Informazioni.

**Indicativi**:
- dati di esempio: nomi, conteggi, commit e data della build;
- misure esatte;
- icone, che sono segnaposto in stile lineare (nell'app si usa il set già adottato);
- volti e miniature, che sono riquadri segnaposto.

Colori, spaziature e corpi del testo si prendono dai token dell'app, non dagli stili dei file.

Le scorciatoie Ctrl+O e Ctrl+E del menu File sono nuove: vedi `design.md`, D6.
