-- v8: qualità dei volti, gruppi ignorati, copertina manuale, vincoli e suggerimenti di unione.

ALTER TABLE face ADD COLUMN sharpness REAL;
ALTER TABLE face ADD COLUMN frontality REAL;
ALTER TABLE face ADD COLUMN size_px INTEGER;
ALTER TABLE face ADD COLUMN quality_score REAL;
ALTER TABLE face ADD COLUMN quality_override INTEGER NOT NULL DEFAULT 0;

UPDATE face SET quality_score = quality WHERE quality_score IS NULL;

CREATE INDEX face_person_quality ON face(person_id, quality_score DESC, id);
CREATE INDEX face_quality_score ON face(quality_score DESC, id);

ALTER TABLE person ADD COLUMN ignored_at INTEGER;
ALTER TABLE person ADD COLUMN ignored_faces INTEGER NOT NULL DEFAULT 0;
ALTER TABLE person ADD COLUMN cover_face_id INTEGER REFERENCES face(id) ON DELETE SET NULL;

CREATE INDEX person_ignored ON person(ignored_at);

CREATE TABLE person_not_same (
    a          INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    b          INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    created_at INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (a, b),
    CHECK (a < b)
);

CREATE TABLE person_suggestion (
    a          INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    b          INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    score      REAL NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (a, b),
    CHECK (a < b)
);
