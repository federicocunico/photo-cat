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
    aiHelp:
      "Volti, soggetti e luoghi si riconoscono sul computer. I modelli si scaricano in background: intanto puoi usare l'app.",
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
    aiHelp:
      "Faces, subjects and places are recognised on your computer. Models download in the background: you can use the app meanwhile.",
    summary:
      "Your computer has {{gb}} GB of memory and {{cores}} processors: we'll use the {{profile}} profile. You can change it in Settings.",
    gpu: "Your graphics card supports acceleration: photo analysis will be faster.",
    cpuOnly: "Analysis will use the processor: slower, but it works on any computer.",
    noRuntime: "ONNX Runtime unavailable: AI features will stay disabled.",
    checking: "Checking your system…",
    modelsReady: "AI models are already ready.",
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
      settings: {
        title: "Settings",
        body: "Theme, language, AI recognition, models and diagnostics. If something goes wrong, Help › Report a problem prepares the diagnostic bundle.",
      },
    },
  },
};

export const guide = { it, en };
