-- photo-cat schema v1

CREATE TABLE source (
    id              INTEGER PRIMARY KEY,
    path            TEXT NOT NULL UNIQUE,
    recursive       INTEGER NOT NULL DEFAULT 1,
    include_formats TEXT NOT NULL DEFAULT '[]',   -- JSON array di estensioni
    ignore_patterns TEXT NOT NULL DEFAULT '[]',   -- JSON array di glob
    watch           INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL
);

-- Un elemento della libreria. `kind` permette tipi futuri (video, documenti).
CREATE TABLE media_item (
    id           INTEGER PRIMARY KEY,
    kind         TEXT NOT NULL DEFAULT 'photo',
    source_id    INTEGER REFERENCES source(id) ON DELETE CASCADE,
    path         TEXT NOT NULL UNIQUE,
    name         TEXT NOT NULL,
    ext          TEXT NOT NULL,
    size         INTEGER NOT NULL,
    mtime        INTEGER NOT NULL,          -- ms epoch
    quick_hash   TEXT,
    content_hash TEXT,
    status       TEXT NOT NULL DEFAULT 'ok', -- ok | missing
    thumb        TEXT,                       -- chiave in cache miniature
    added_at     INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
CREATE INDEX media_source ON media_item(source_id);
CREATE INDEX media_quick_hash ON media_item(quick_hash);
CREATE INDEX media_content_hash ON media_item(content_hash);
CREATE INDEX media_ext ON media_item(ext);
CREATE INDEX media_status ON media_item(status);

CREATE TABLE media_tech (
    media_id    INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    format      TEXT,
    width       INTEGER,
    height      INTEGER,
    dpi_x       REAL,
    dpi_y       REAL,
    bit_depth   INTEGER,
    color_space TEXT,
    orientation INTEGER
);

CREATE TABLE media_dates (
    media_id        INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    captured_at     TEXT,   -- 'AAAA-MM-GG HH:MM:SS' ora locale dello scatto
    captured_source TEXT,   -- exif_original | exif_create | file_system
    modified_at     TEXT
);
CREATE INDEX dates_captured ON media_dates(captured_at);

CREATE TABLE exif (
    media_id     INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    camera_make  TEXT,
    camera_model TEXT,
    lens         TEXT,
    exposure     TEXT,
    f_number     REAL,
    iso          INTEGER,
    focal_length REAL,
    gps_lat      REAL,
    gps_lon      REAL,
    raw          TEXT     -- JSON con tutti i tag letti
);

CREATE TABLE location (
    media_id INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    name     TEXT,
    country  TEXT,
    lat      REAL,
    lon      REAL,
    source   TEXT NOT NULL   -- gps | directory | manual
);
CREATE INDEX location_name ON location(name, media_id);

-- Catalogazioni (automatiche o manuali) per gruppo della tassonomia.
CREATE TABLE label (
    media_id     INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    grp          TEXT NOT NULL,
    value        TEXT NOT NULL,
    score        REAL NOT NULL,
    origin       TEXT NOT NULL,  -- auto | manual
    model_ver    TEXT,
    taxonomy_ver INTEGER,
    PRIMARY KEY (media_id, grp, value)
);
CREATE INDEX label_grp_value ON label(grp, value, media_id);

CREATE TABLE embedding (
    media_id  INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    model_ver TEXT NOT NULL,
    vector    BLOB NOT NULL,
    PRIMARY KEY (media_id, model_ver)
);

CREATE TABLE media_people (
    media_id  INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    count     INTEGER NOT NULL,
    boxes     TEXT NOT NULL DEFAULT '[]',
    model_ver TEXT
);

CREATE TABLE person (
    id         INTEGER PRIMARY KEY,
    name       TEXT,
    centroid   BLOB,
    created_at INTEGER NOT NULL
);

CREATE TABLE face (
    id           INTEGER PRIMARY KEY,
    media_id     INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    bbox         TEXT NOT NULL,            -- JSON [x, y, w, h] normalizzati
    quality      REAL NOT NULL,
    embedding    BLOB,
    person_id    INTEGER REFERENCES person(id) ON DELETE SET NULL,
    assigned_by  TEXT,                      -- auto | manual
    rejected     TEXT NOT NULL DEFAULT '[]', -- JSON id di persone rifiutate
    model_ver    TEXT
);
CREATE INDEX face_media ON face(media_id, person_id);
CREATE INDEX face_person ON face(person_id, media_id);

CREATE TABLE tag (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE COLLATE NOCASE
);
CREATE TABLE media_tag (
    media_id INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    tag_id   INTEGER NOT NULL REFERENCES tag(id) ON DELETE CASCADE,
    PRIMARY KEY (media_id, tag_id)
);
CREATE INDEX media_tag_tag ON media_tag(tag_id, media_id);

CREATE TABLE catalog (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    kind       TEXT NOT NULL,     -- manual | smart
    rules      TEXT,              -- JSON per i cataloghi smart
    created_at INTEGER NOT NULL
);
CREATE TABLE catalog_item (
    catalog_id INTEGER NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    media_id   INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    PRIMARY KEY (catalog_id, media_id)
);
CREATE INDEX catalog_item_media ON catalog_item(media_id);

CREATE TABLE job (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL,
    state       TEXT NOT NULL,
    params      TEXT NOT NULL DEFAULT '{}',
    created_at  INTEGER NOT NULL,
    started_at  INTEGER,
    finished_at INTEGER
);
CREATE TABLE job_phase (
    job_id      INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    phase       TEXT NOT NULL,
    ord         INTEGER NOT NULL,
    done        INTEGER NOT NULL DEFAULT 0,
    failed      INTEGER NOT NULL DEFAULT 0,
    total       INTEGER NOT NULL DEFAULT 0,
    total_known INTEGER NOT NULL DEFAULT 0,
    started_at  INTEGER,
    finished_at INTEGER,
    PRIMARY KEY (job_id, phase)
);
-- Stato per elemento e fase: base della ripresa dei job.
CREATE TABLE item_stage (
    job_id  INTEGER NOT NULL REFERENCES job(id) ON DELETE CASCADE,
    item    TEXT NOT NULL,     -- id media o percorso, a seconda del job
    phase   TEXT NOT NULL,
    state   TEXT NOT NULL,     -- pending | done | failed
    PRIMARY KEY (job_id, item, phase)
);

CREATE TABLE error_log (
    id      INTEGER PRIMARY KEY,
    ts      TEXT NOT NULL,
    level   TEXT NOT NULL,
    code    TEXT,
    message TEXT NOT NULL,
    target  TEXT NOT NULL,
    job_id  INTEGER,
    phase   TEXT,
    path    TEXT,
    detail  TEXT
);
CREATE INDEX error_log_job ON error_log(job_id);
CREATE INDEX error_log_ts ON error_log(ts);

CREATE TABLE organize_op (
    id         INTEGER PRIMARY KEY,
    created_at INTEGER NOT NULL,
    dest       TEXT NOT NULL,
    method     TEXT NOT NULL,     -- symlink | hardlink | copy
    params     TEXT NOT NULL DEFAULT '{}',
    state      TEXT NOT NULL      -- running | done | undone
);
CREATE TABLE organize_entry (
    op_id   INTEGER NOT NULL REFERENCES organize_op(id) ON DELETE CASCADE,
    created TEXT NOT NULL,
    target  TEXT,
    kind    TEXT NOT NULL         -- dir | symlink | hardlink | copy
);

CREATE TABLE undo_log (
    id          INTEGER PRIMARY KEY,
    ts          INTEGER NOT NULL,
    description TEXT NOT NULL,
    payload     TEXT NOT NULL
);

CREATE TABLE settings_kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Ricerca testuale. rowid = media_item.id, aggiornata dall'applicazione.
CREATE VIRTUAL TABLE media_fts USING fts5(
    name, path, place, people, tags,
    tokenize = 'unicode61 remove_diacritics 2'
);
