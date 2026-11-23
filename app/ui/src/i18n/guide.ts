// Testi della guida nell'app: primo avvio, "Primi passi", suggerimenti, guida (it/en).

const it = {
  onboarding: {
    title: "Benvenuto in photo-cat",
    progress: "Passo {{n}} di 3",
    steps: { 0: "Benvenuto", 1: "Le tue foto", 2: "Funzioni AI" },
    welcome:
      "Cataloga le tue foto in locale: date, luoghi, persone e soggetti, senza che nessuna immagine lasci il computer.",
    language: "Lingua",
    theme: "Tema",
    photosTitle: "Dove sono le tue foto?",
    photosHelp:
      "photo-cat legge le foto dove sono già: non sposta e non modifica i file. Puoi aggiungere altre cartelle in seguito da Importa.",
    folder: "Cartella delle foto",
    folderHint: "Trascina qui una cartella oppure sceglila.",
    recursive: "Includi le sottocartelle",
    continueHint: "«Continua» si attiva dopo aver scelto una cartella.",
    aiTitle: "Funzioni AI",
    aiHelp: "Volti, soggetti e luoghi si riconoscono sul computer.",
    modelsDownload: "I modelli si scaricano in background: intanto puoi usare l'app.",
    summary:
      "Il computer ha {{gb}} GB di memoria e {{cores}} processori: useremo il profilo {{profile}}. Puoi cambiarlo in Impostazioni.",
    gpu: "La scheda video supporta l'accelerazione: l'analisi delle foto sarà più rapida.",
    cpuOnly: "L'analisi userà il processore: è più lenta, ma funziona su ogni computer.",
    noRuntime: "ONNX Runtime non disponibile: le funzioni AI resteranno disattivate.",
    checking: "Verifica del sistema in corso…",
    modelsReady: "I modelli AI sono già pronti.",
    back: "Indietro",
    continue: "Continua",
    later: "Lo faccio dopo",
    start: "Scarica i modelli e inizia",
    finish: "Inizia",
    importStarted: "Importazione avviata: le foto compaiono in Libreria man mano.",
    followImport: "Segui l'importazione",
    aiOff: "Funzioni AI disattivate: i modelli non sono ancora stati scaricati.",
    download: "Scarica i modelli",
    downloadStarted: "Download dei modelli avviato.",
  },
  guide: {
    firstSteps: "Primi passi",
    progress: "{{done}} di {{total}}",
    hide: "Nascondi primi passi",
    steps: {
      import: "Importa una cartella",
      models: "Scarica i modelli AI",
      person: "Dai un nome a una persona",
      catalog: "Crea un catalogo",
      tags: "Usa un set di tag",
    },
    stepDone: "{{name}}: fatto",
    gotIt: "Ho capito",
    learnMore: "Scopri di più",
    tips: {
      people:
        "Qui i volti sono raggruppati per persona: dai un nome a un gruppo e la persona viene riconosciuta nelle altre foto.",
      catalogs:
        "Un catalogo raccoglie foto scelte a mano o con regole (Smart), e si aggiorna da solo quando importi nuove foto.",
      tags: "Un set assegna i tasti 1–9: in Libreria premi un numero per taggare le foto selezionate",
      sequence:
        "Sequenza mette in fila per data le foto di più cartelle: scegli qui le cartelle e scorri con le frecce.",
      moved: "Dove sono Modelli AI e Diagnostica? Ora sono sezioni di Impostazioni.",
    },
    tipsReset: "I suggerimenti ricompariranno al prossimo ingresso in ogni modulo.",
    title: "Guida",
    intro: "Una panoramica di photo-cat. Premi ? per l'elenco delle scorciatoie.",
    sections: {
      library: {
        title: "Libreria",
        body: "Tutte le foto importate. Filtra dalla barra laterale, seleziona con clic, Maiusc e Ctrl, e agisci sulla selezione dalla barra in alto. Ctrl+K cerca comandi, persone, tag e cataloghi.",
      },
      people: {
        title: "Persone",
        body: "I volti simili sono raggruppati. Dai un nome a un gruppo, unisci i gruppi della stessa persona e separa i volti sbagliati: ogni operazione si può annullare.",
      },
      catalogs: {
        title: "Cataloghi",
        body: "Un catalogo manuale contiene le foto che aggiungi tu; uno Smart segue delle regole (date, luoghi, persone, tag) e si aggiorna da solo.",
      },
      tags: {
        title: "Tag",
        body: "I tag sono etichette libere. Un set collega fino a nove tag ai tasti 1–9, per taggare velocemente in Libreria e in Sequenza.",
      },
      sequence: {
        title: "Sequenza",
        body: "Mostra le foto di più cartelle in ordine di scatto, una dopo l'altra, per scegliere e taggare in fretta.",
      },
      import: {
        title: "Importa ed esporta",
        body: "Importa aggiunge cartelle senza spostare né modificare i file. Esporta copia o organizza le foto in nuove cartelle, con un'anteprima prima di scrivere.",
      },
      autoUpdate: {
        title: "Aggiornamento automatico",
        body: "Con «Aggiorna automaticamente» attivo su una cartella, mentre photo-cat è aperto le foto aggiunte, spostate o cancellate in quella cartella vengono importate o aggiornate da sole. Ad app chiusa non succede nulla: alla riapertura usa «Aggiorna». Sotto la cartella, nella pagina Importa, vedi se l'aggiornamento automatico è attivo e quando è avvenuto l'ultimo, oppure perché non è disponibile.",
      },
      rawReview: {
        title: "Varianti RAW (Fujifilm X-T5)",
        body: "Il modulo si trova nel gruppo «Importa ed esporta» (Ctrl+9). Apre una cartella di foto Fujifilm senza importarla in libreria, mette a confronto le varianti di simulazione pellicola di ogni scatto, ti lascia scegliere la migliore e dare le stelle, e alla fine esporta le scelte in un nuovo catalogo. Per cominciare usa «Apri cartella…», una delle «Sessioni recenti» oppure «Apri progetto FujiView (.fujiview)…».\n\nLe forme di cartella riconosciute sono tre: solo file RAF; file RAF con le varianti già generate in «FujiView varianti/<preset>/»; una sottocartella per ogni preset con file JPEG o TIFF. Uno scatto senza varianti mostra l'«Anteprima RAW», cioè l'anteprima contenuta nel file RAF: serve per confrontare e valutare, ma non si può scegliere come variante migliore né esportare. Prima va sviluppato.\n\nSviluppo dalla fotocamera (solo su Windows). «Sviluppa dalla fotocamera…» fa sviluppare i file RAF alla X-T5 collegata con USB, con le sue ricette C1–C7 e con «Come scattato» (le impostazioni registrate nel RAF). Prepara la fotocamera così: in MENU > SET UP > CONNECTION SETTING > USB MODE scegli «USB RAW CONV./BACKUP RESTORE», disattiva lo spegnimento automatico (AUTO POWER OFF), chiudi FUJIFILM X RAW Studio e ogni altro programma Fujifilm, e usa file RAF scattati con lo stesso corpo macchina collegato (gli altri vengono saltati). La finestra controlla la fotocamera e, prima di partire, mostra quante conversioni servono, lo spazio su disco necessario e un tempo stimato: su una X-T5 con RAF non compressi da 87 MB servono circa 2 secondi per caricare ogni RAF e 1,5–2,5 secondi per ogni variante, e ogni JPEG a piena risoluzione pesa circa 26 MB. Le varianti vanno nella cartella di output che scegli (in modo predefinito «FujiView varianti/», accanto ai RAF). Lo sviluppo è un'attività: si può mettere in pausa e riprendere, si ferma da solo se finisce lo spazio o la fotocamera si scollega, salta le varianti già presenti (a meno di «Sovrascrivi le varianti esistenti») e non modifica mai i file RAF. Su Linux e macOS la voce è spenta con «Disponibile solo su Windows».\n\nConfronto. Ogni preset ha un pannello (fino a 9): rotellina per ingrandire, trascinamento per spostare e doppio clic per adattare, con zoom e spostamento uguali in tutti i pannelli. Nella barra a destra puoi nascondere e riordinare i preset e leggere la ricetta di ogni variante. Scorciatoie: Maiusc+1…9 sceglie la variante del pannello corrispondente, Invio quella del pannello a fuoco (si sposta con le frecce), 1–5 dà le stelle e 0 le toglie (valgono anche Alt+1–5 e Alt+0), Spazio passa allo scatto successivo e Maiusc+Spazio al precedente. Scegliere non fa avanzare da solo. Ctrl+1…9 continuano ad aprire i moduli. L'elenco completo è nel foglio delle scorciatoie (?).\n\nSessioni. Scelte, stelle, preset nascosti e ordine dei pannelli si salvano da soli nei dati dell'app, mai nella cartella delle foto: riaprendo la cartella riparti da dove eri. Le cartelle usate di recente compaiono in «Sessioni recenti». I progetti .fujiview di FujiView si possono aprire.\n\nEsportazione. «Esporta in un nuovo catalogo…» copia in una cartella a tua scelta, senza ricodificarli, i soli file JPEG delle varianti scelte: RAF e TIFF non vengono mai copiati. Puoi limitare l'esportazione a una valutazione minima e ad alcuni preset, e rinominare i file con un modello (per esempio «Dolomiti_{seq:04d}», con {date}, {time}, {device}, {orig} e {preset}). La cartella di destinazione viene aggiunta alla libreria, o aggiornata se ne fa già parte; il catalogo può essere nuovo o uno manuale già esistente. Le foto ricevono le stelle date in revisione (uno scatto senza stelle tiene quelle che ha già in libreria) e, con «Aggiungi il preset come tag», il preset come tag del set «Preset Fujifilm». Se annulli l'attività, i file già copiati restano ma il catalogo non viene creato.\n\nCache delle anteprime. Le anteprime si salvano su disco per aprirsi subito: il limite (20 GB in modo predefinito) si cambia in Impostazioni › Generale › «Cache delle anteprime di Varianti RAW (GB)», dove si può anche svuotare. Le foto non vengono mai toccate.",
      },
      settings: {
        title: "Impostazioni",
        body: "Tema, lingua, riconoscimento AI, modelli e diagnostica. Se qualcosa non va, Aiuto › Segnala un problema prepara il pacchetto diagnostico.",
      },
    },
  },
};

const en: typeof it = {
  onboarding: {
    title: "Welcome to photo-cat",
    progress: "Step {{n}} of 3",
    steps: { 0: "Welcome", 1: "Your photos", 2: "AI features" },
    welcome:
      "Catalog your photos locally: dates, places, people and subjects, without any image leaving your computer.",
    language: "Language",
    theme: "Theme",
    photosTitle: "Where are your photos?",
    photosHelp:
      "photo-cat reads photos where they already are: it doesn't move or modify files. You can add more folders later from Import.",
    folder: "Photo folder",
    folderHint: "Drag a folder here or choose one.",
    recursive: "Include subfolders",
    continueHint: "“Continue” becomes available once you choose a folder.",
    aiTitle: "AI features",
    aiHelp: "Faces, subjects and places are recognised on your computer.",
    modelsDownload: "Models download in the background: you can use the app meanwhile.",
    summary:
      "Your computer has {{gb}} GB of memory and {{cores}} processors: we'll use the {{profile}} profile. You can change it in Settings.",
    gpu: "Your graphics card supports acceleration: photo analysis will be faster.",
    cpuOnly: "Analysis will use the processor: slower, but it works on any computer.",
    noRuntime: "ONNX Runtime unavailable: AI features will stay disabled.",
    checking: "Checking your system…",
    modelsReady: "The AI models are ready.",
    back: "Back",
    continue: "Continue",
    later: "Later",
    start: "Download models and start",
    finish: "Start",
    importStarted: "Import started: photos appear in the Library as they are found.",
    followImport: "Follow the import",
    aiOff: "AI features are off: the models haven't been downloaded yet.",
    download: "Download models",
    downloadStarted: "Model download started.",
  },
  guide: {
    firstSteps: "First steps",
    progress: "{{done}} of {{total}}",
    hide: "Hide first steps",
    steps: {
      import: "Import a folder",
      models: "Download the AI models",
      person: "Name a person",
      catalog: "Create a catalog",
      tags: "Use a tag set",
    },
    stepDone: "{{name}}: done",
    gotIt: "Got it",
    learnMore: "Learn more",
    tips: {
      people:
        "Faces are grouped by person here: name a group and the person is recognised in other photos.",
      catalogs:
        "A catalog collects photos chosen by hand or by rules (Smart), and updates itself when you import new photos.",
      tags: "A set assigns keys 1–9: in the Library press a number to tag the selected photos",
      sequence:
        "Sequence lines up photos from several folders by date: choose the folders here and browse with the arrow keys.",
      moved: "Where are AI models and Diagnostics? They are now sections of Settings.",
    },
    tipsReset: "Tips will show again the next time you open each module.",
    title: "Guide",
    intro: "An overview of photo-cat. Press ? for the list of shortcuts.",
    sections: {
      library: {
        title: "Library",
        body: "All imported photos. Filter from the sidebar, select with click, Shift and Ctrl, and act on the selection from the bar at the top. Ctrl+K searches commands, people, tags and catalogs.",
      },
      people: {
        title: "People",
        body: "Similar faces are grouped. Name a group, merge groups of the same person and split wrong faces: every operation can be undone.",
      },
      catalogs: {
        title: "Catalogs",
        body: "A manual catalog holds the photos you add; a Smart one follows rules (dates, places, people, tags) and updates itself.",
      },
      tags: {
        title: "Tags",
        body: "Tags are free labels. A set links up to nine tags to keys 1–9, for fast tagging in the Library and in Sequence.",
      },
      sequence: {
        title: "Sequence",
        body: "Shows photos from several folders in shooting order, one after the other, to pick and tag quickly.",
      },
      import: {
        title: "Import and export",
        body: "Import adds folders without moving or modifying files. Export copies or organizes photos into new folders, with a preview before writing.",
      },
      autoUpdate: {
        title: "Automatic update",
        body: "With “Update automatically” on for a folder, while photo-cat is open photos added, moved or deleted in that folder are imported or updated on their own. Nothing happens while the app is closed: use “Update” when you reopen it. Under the folder, on the Import page, you can see whether automatic update is on and when the last one happened, or why it is not available.",
      },
      rawReview: {
        title: "RAW variants (Fujifilm X-T5)",
        body: "The module is in the “Import and export” group (Ctrl+9). It opens a folder of Fujifilm photos without importing it into the library, compares the film simulation variants of each shot, lets you pick the best one and give stars, and finally exports your picks into a new catalog. To start, use “Open folder…”, one of the “Recent sessions”, or “Open FujiView project (.fujiview)…”.\n\nThree folder layouts are recognised: RAF files only; RAF files with the variants already generated in “FujiView varianti/<preset>/”; one subfolder per preset with JPEG or TIFF files. A shot without variants shows the “RAW preview”, the preview embedded in the RAF file: it is for comparing and rating, but it cannot be picked as the best variant or exported. It has to be developed first.\n\nDeveloping from the camera (Windows only). “Develop from the camera…” has the X-T5 connected by USB develop the RAF files with its C1–C7 recipes and with “As shot” (the settings recorded in the RAF). Prepare the camera like this: in MENU > SET UP > CONNECTION SETTING > USB MODE choose “USB RAW CONV./BACKUP RESTORE”, turn off auto power off (AUTO POWER OFF), close FUJIFILM X RAW Studio and any other Fujifilm software, and use RAF files shot with the same camera body that is connected (others are skipped). The dialog checks the camera and, before starting, shows how many conversions are needed, the disk space required and an estimated time: on an X-T5 with 87 MB uncompressed RAFs it takes about 2 seconds to upload each RAF and 1.5–2.5 seconds per variant, and each full-size JPEG is about 26 MB. Variants go to the output folder you choose (by default “FujiView varianti/”, next to the RAFs). Developing runs as an activity: it can be paused and resumed, stops by itself if disk space runs out or the camera disconnects, skips variants already present (unless “Overwrite existing variants” is on) and never modifies the RAF files. On Linux and macOS the entry is disabled with “Available on Windows only”.\n\nComparing. Each preset has a panel (up to 9): mouse wheel to zoom, drag to pan and double-click to fit, with the same zoom and position in every panel. In the right sidebar you can hide and reorder presets and read each variant's recipe. Shortcuts: Shift+1…9 picks the variant of the matching panel, Enter that of the focused panel (move it with the arrow keys), 1–5 gives stars and 0 clears them (Alt+1–5 and Alt+0 work too), Space goes to the next shot and Shift+Space to the previous one. Picking does not advance by itself. Ctrl+1…9 still open the modules. The full list is in the shortcuts sheet (?).\n\nSessions. Picks, stars, hidden presets and panel order are saved automatically in the app data, never in the photo folder: reopening the folder, you pick up where you left off. Recently used folders appear in “Recent sessions”. FujiView .fujiview projects can be opened.\n\nExporting. “Export to a new catalog…” copies, without re-encoding, only the JPEG files of the picked variants to a folder of your choice: RAF and TIFF files are never copied. You can limit the export to a minimum rating and to some presets, and rename files with a template (for example “Dolomiti_{seq:04d}”, with {date}, {time}, {device}, {orig} and {preset}). The destination folder is added to the library, or updated if it is already part of it; the catalog can be new or an existing manual one. Photos get the stars given in the review (a shot without stars keeps any it already has in the library) and, with “Add the preset as a tag”, the preset as a tag of the “Preset Fujifilm” set. If you cancel the activity, files already copied stay but the catalog is not created.\n\nPreview cache. Previews are saved on disk so they open at once: the limit (20 GB by default) is changed in Settings › General › “RAW Variants preview cache (GB)”, where you can also clear it. Photos are never touched.",
      },
      settings: {
        title: "Settings",
        body: "Theme, language, AI recognition, models and diagnostics. If something goes wrong, Help › Report a problem prepares the diagnostic bundle.",
      },
    },
  },
};

export const guide = { it, en };
