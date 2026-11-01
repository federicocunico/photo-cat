-- v5: collegamento delle persone alle identità dell'app (app.db).
-- L'UUID (non un id intero) perché i due archivi evolvono separatamente.
ALTER TABLE person ADD COLUMN identity_uid TEXT;
CREATE INDEX person_identity ON person(identity_uid);

-- Istante reale di scatto e gruppi di sorgente (dispositivo + fuso dichiarato).
-- captured_at resta l'ora letta dal file; l'istante corretto è materializzato.
CREATE TABLE source_group (
    id                  INTEGER PRIMARY KEY,
    device              TEXT,
    declared_offset_min INTEGER,
    -- Fuso dell'ora locale mostrata (IANA) o scostamento fisso, scelti dall'utente.
    display_tz          TEXT,
    display_offset_min  INTEGER,
    -- Correzione dell'orologio in secondi.
    shift_s             INTEGER NOT NULL DEFAULT 0
);
CREATE UNIQUE INDEX source_group_key
    ON source_group(COALESCE(device, ''), COALESCE(declared_offset_min, -100000));

ALTER TABLE media_dates ADD COLUMN declared_offset_min INTEGER;
ALTER TABLE media_dates ADD COLUMN device TEXT;
-- Millisecondi UTC, già corretti con fuso e scostamento del gruppo.
ALTER TABLE media_dates ADD COLUMN instant_utc INTEGER;
-- 1 = fuso non dichiarato nel file (dedotto dal gruppo o dal sistema).
ALTER TABLE media_dates ADD COLUMN instant_presumed INTEGER NOT NULL DEFAULT 0;
-- Fuso dell'ora locale mostrata, in minuti.
ALTER TABLE media_dates ADD COLUMN local_offset_min INTEGER;
ALTER TABLE media_dates ADD COLUMN group_id INTEGER REFERENCES source_group(id) ON DELETE SET NULL;
CREATE INDEX media_dates_instant ON media_dates(instant_utc);
CREATE INDEX media_dates_group ON media_dates(group_id);

-- Foto senza data collocate a mano nella sequenza: hanno precedenza sul calcolo.
CREATE TABLE media_time_placement (
    media_id         INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    instant_utc      INTEGER NOT NULL,
    local_offset_min INTEGER
);
