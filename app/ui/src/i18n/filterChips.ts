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
      rating: {
        at_least_one: "Valutazione: almeno {{count}} stella",
        at_least_other: "Valutazione: almeno {{count}} stelle",
        equal_one: "Valutazione: {{count}} stella",
        equal_other: "Valutazione: {{count}} stelle",
        at_most_one: "Valutazione: al massimo {{count}} stella",
        at_most_other: "Valutazione: al massimo {{count}} stelle",
      },
      color: "Colore: {{name}}",
      noMonth: "{{year}}: anno senza mese",
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
      rating: {
        at_least_one: "Rating: at least {{count}} star",
        at_least_other: "Rating: at least {{count}} stars",
        equal_one: "Rating: {{count}} star",
        equal_other: "Rating: {{count}} stars",
        at_most_one: "Rating: at most {{count}} star",
        at_most_other: "Rating: at most {{count}} stars",
      },
      color: "Colour: {{name}}",
      noMonth: "{{year}}: year without month",
      similarSize: {
        two: "Similar groups: 2 photos",
        three_five: "Similar groups: 3–5 photos",
        six_ten: "Similar groups: 6–10 photos",
        over10: "Similar groups: more than 10 photos",
      },
    },
  },
};
