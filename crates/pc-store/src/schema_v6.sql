CREATE TABLE catalog_exclusion (
    catalog_id INTEGER NOT NULL REFERENCES catalog(id) ON DELETE CASCADE,
    media_id INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    excluded_at INTEGER NOT NULL,
    PRIMARY KEY (catalog_id, media_id)
);
CREATE INDEX catalog_exclusion_media ON catalog_exclusion(media_id);
