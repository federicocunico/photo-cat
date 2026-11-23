-- v15: raggruppamento dei volti in due passaggi (face-identity-aggregation).
-- Storico delle unioni di gruppi di volti (manuali, automatiche, dalla coda), annullabili.
-- `target_person` non ha vincoli: la persona di destinazione può essere unita o eliminata
-- in seguito, e "Annulla" la ricrea con lo stesso id se serve.
CREATE TABLE merge_log (
    id            INTEGER PRIMARY KEY,
    at            INTEGER NOT NULL,                      -- ms epoch
    origin        TEXT NOT NULL CHECK (origin IN ('manual', 'auto', 'queue')),
    target_person INTEGER NOT NULL,
    faces         INTEGER NOT NULL,                      -- volti spostati
    snapshot      TEXT NOT NULL,                         -- JSON: gruppi d'origine e loro volti
    undone_at     INTEGER                                -- ms epoch dell'annullamento
);
CREATE INDEX merge_log_at ON merge_log(at);
CREATE INDEX merge_log_target ON merge_log(target_person);

-- Coda "Da confermare": domande del sistema, ricalcolate a ogni analisi. `a` è il gruppo
-- senza nome; `b` la persona o il gruppo proposto; `c` la seconda persona di "È A o B?".
CREATE TABLE review_question (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('group_person', 'group_group', 'group_choice')),
    a           INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    b           INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    c           INTEGER REFERENCES person(id) ON DELETE CASCADE,
    score       REAL NOT NULL,
    probability REAL NOT NULL,
    faces       INTEGER NOT NULL,                        -- volti risolti da una risposta
    rank        INTEGER NOT NULL,                        -- ordine della coda (design D6)
    created_at  INTEGER NOT NULL
);
CREATE INDEX review_question_rank ON review_question(rank);
CREATE INDEX review_question_a ON review_question(a);
CREATE INDEX review_question_b ON review_question(b);
CREATE INDEX review_question_c ON review_question(c);

-- Volti confermati: è il flag esistente `face.assigned_by = 'manual'`, che ricevono anche i
-- volti passati da una risposta della coda; nessuna colonna nuova. Indice dei volti per foto
-- (vincolo della stessa foto e taratura delle soglie): `face_media(media_id, person_id)` della v1.
