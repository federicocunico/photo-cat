-- Archivio dell'app (app.db): identità e set di tag, indipendenti dalla libreria.

CREATE TABLE identity (
    uid         TEXT PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
    short_name  TEXT NOT NULL,
    grp         TEXT,
    -- Soglia di somiglianza propria (coseno); NULL = predefinita.
    threshold   REAL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE identity_alias (
    uid   TEXT NOT NULL REFERENCES identity(uid) ON DELETE CASCADE ON UPDATE CASCADE,
    alias TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (uid, alias)
);
CREATE INDEX identity_alias_alias ON identity_alias(alias);

-- Volti di riferimento: ritaglio allineato (PNG 112x112) ed embedding per
-- versione di modello. Nessuna foto originale.
CREATE TABLE identity_face (
    id         INTEGER PRIMARY KEY,
    uid        TEXT NOT NULL REFERENCES identity(uid) ON DELETE CASCADE ON UPDATE CASCADE,
    crop       BLOB NOT NULL,
    model_ver  TEXT NOT NULL,
    embedding  BLOB,
    source     TEXT NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX identity_face_uid ON identity_face(uid);

CREATE TABLE tag_set (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE COLLATE NOCASE,
    exclusive INTEGER NOT NULL DEFAULT 0,
    is_base   INTEGER NOT NULL DEFAULT 0,
    -- I tag del set diventano parole chiave XMP nelle copie esportate.
    export_keywords INTEGER NOT NULL DEFAULT 0,
    position  INTEGER NOT NULL
);

CREATE TABLE tag_set_item (
    set_id   INTEGER NOT NULL REFERENCES tag_set(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position BETWEEN 1 AND 9),
    tag_name TEXT NOT NULL COLLATE NOCASE,
    PRIMARY KEY (set_id, position),
    UNIQUE (set_id, tag_name)
);

CREATE TABLE tag_style (
    tag_name TEXT PRIMARY KEY COLLATE NOCASE,
    color    TEXT NOT NULL
);

CREATE TABLE app_kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Set "Base" predefinito (modificabile dall'utente).
INSERT INTO tag_set (id, name, exclusive, is_base, position) VALUES (1, 'Base', 0, 1, 0);
INSERT INTO tag_set_item (set_id, position, tag_name) VALUES
    (1, 1, 'Preferite'),
    (1, 2, 'Da stampare'),
    (1, 3, 'Da scartare'),
    (1, 4, 'Famiglia'),
    (1, 5, 'Viaggi'),
    (1, 6, 'Lavoro');
INSERT INTO app_kv (key, value) VALUES ('active_tag_set', '1');
