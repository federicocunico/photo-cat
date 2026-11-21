-- v14: valutazione e colore delle foto, data manuale con precisione, data di
-- scomparsa dei file mancanti, errori archiviati.
ALTER TABLE media_item ADD COLUMN rating INTEGER NOT NULL DEFAULT 0
    CHECK (rating BETWEEN 0 AND 5);
ALTER TABLE media_item ADD COLUMN color_trait TEXT
    CHECK (color_trait IN ('bw', 'color', 'faded', 'sepia'));
ALTER TABLE media_item ADD COLUMN missing_since INTEGER; -- ms epoch
CREATE INDEX media_rating ON media_item(rating);
CREATE INDEX media_color_trait ON media_item(color_trait);
UPDATE media_item SET missing_since = CAST(strftime('%s', 'now') AS INTEGER) * 1000
    WHERE status = 'missing';

ALTER TABLE media_dates ADD COLUMN manual_at TEXT; -- AAAA-MM-GG HH:MM:SS
ALTER TABLE media_dates ADD COLUMN manual_precision TEXT
    CHECK (manual_precision IN ('day', 'month', 'year'));

ALTER TABLE error_log ADD COLUMN archived_at TEXT;
CREATE INDEX error_log_active ON error_log(id) WHERE archived_at IS NULL;
