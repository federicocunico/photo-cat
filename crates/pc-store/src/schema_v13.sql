-- v13: coordinate dei luoghi (nomi da cartella o manuali) per la mappa, e indice
-- sulle coordinate GPS delle foto.
CREATE TABLE place_geo (
    name       TEXT PRIMARY KEY,
    lat        REAL NOT NULL,
    lon        REAL NOT NULL,
    country    TEXT,
    source     TEXT NOT NULL CHECK (source IN ('gazetteer', 'manual')),
    updated_at INTEGER NOT NULL
);
CREATE INDEX location_geo ON location(lat, lon) WHERE lat IS NOT NULL;
