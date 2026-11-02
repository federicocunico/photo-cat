-- Gruppi di foto quasi identiche (raffiche, scatti ripetuti).
CREATE TABLE similar_group (
    id INTEGER PRIMARY KEY,
    cover_media_id INTEGER REFERENCES media_item(id) ON DELETE SET NULL,
    cover_manual INTEGER NOT NULL DEFAULT 0,
    params TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
CREATE TABLE similar_member (
    media_id INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    group_id INTEGER NOT NULL REFERENCES similar_group(id) ON DELETE CASCADE,
    score REAL NOT NULL
);
CREATE INDEX similar_member_group ON similar_member(group_id);
-- Vincoli manuali tra due foto (a < b): 'must' unisce, 'cannot' separa.
CREATE TABLE similar_link (
    a INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    b INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('must', 'cannot')),
    created_at INTEGER NOT NULL,
    PRIMARY KEY (a, b)
);
CREATE INDEX similar_link_b ON similar_link(b);
-- Coppie candidate confermate con il loro SSIM (anche sotto soglia, fino a 0,6), per il retune.
CREATE TABLE similar_edge (
    a INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    b INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    ssim REAL NOT NULL,
    PRIMARY KEY (a, b)
);
CREATE INDEX similar_edge_b ON similar_edge(b);
-- Foto già elaborate con i parametri del calcolo (analisi incrementale e riprendibile).
CREATE TABLE similar_done (
    media_id INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    params TEXT NOT NULL,
    -- Misure della miniatura per proporre la foto principale.
    sharpness REAL NOT NULL DEFAULT 0,
    exposure REAL NOT NULL DEFAULT 1
);
-- Gruppi sciolti: le foto con lo stesso token non si uniscono più automaticamente.
CREATE TABLE similar_dissolved (
    media_id INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    token INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (media_id, token)
);
CREATE INDEX similar_dissolved_token ON similar_dissolved(token);
