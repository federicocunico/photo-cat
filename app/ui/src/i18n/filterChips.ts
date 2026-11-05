// Chip dei filtri attivi per stato di analisi, confidenza delle etichette e gruppi di simili.
export const filterChips = {
  it: {
    filterChips: {
      stage: {
        classification: "Classificazione",
        persons: "Persone",
        faces: "Volti",
        similar: "Gruppi di simili",
        phash: "Impronte",
      },
      state: { current: "aggiornate", stale: "da aggiornare", never: "mai analizzate" },
      analysis: "{{stage}}: {{state}}",
      score: "{{group}}: confidenza {{min}}–{{max}}",
      area: "Area: {{label}}",
      similarSize: {
        two: "Gruppi di simili: 2 foto",
        three_five: "Gruppi di simili: 3–5 foto",
        six_ten: "Gruppi di simili: 6–10 foto",
        over10: "Gruppi di simili: più di 10 foto",
      },
    },
  },
  en: {
    filterChips: {
      stage: {
        classification: "Classification",
        persons: "People",
        faces: "Faces",
        similar: "Similar groups",
        phash: "Fingerprints",
      },
      state: { current: "up to date", stale: "to update", never: "never analyzed" },
      analysis: "{{stage}}: {{state}}",
      score: "{{group}}: confidence {{min}}–{{max}}",
      area: "Area: {{label}}",
      similarSize: {
        two: "Similar groups: 2 photos",
        three_five: "Similar groups: 3–5 photos",
        six_ten: "Similar groups: 6–10 photos",
        over10: "Similar groups: more than 10 photos",
      },
    },
  },
};
