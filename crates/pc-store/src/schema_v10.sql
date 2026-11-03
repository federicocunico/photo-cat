-- v10: cataloghi di sistema (Cestino, Duplicati, Qualità insufficiente) e misure di qualità
-- delle foto. Le foto di un catalogo di sistema sono nascoste dalle viste della Libreria.

ALTER TABLE catalog ADD COLUMN system TEXT;
CREATE UNIQUE INDEX catalog_system ON catalog(system) WHERE system IS NOT NULL;

INSERT INTO catalog(name, kind, rules, created_at, system) VALUES
    ('Cestino', 'manual', NULL, 0, 'trash'),
    ('Duplicati', 'manual', NULL, 0, 'duplicates'),
    ('Qualità insufficiente', 'manual', NULL, 0, 'low_quality');

-- Misure del controllo di qualità: una riga per foto esaminata (anche se idonea), così un
-- nuovo controllo esamina solo le foto nuove e non annulla i ripristini dell'utente.
CREATE TABLE media_quality (
    media_id    INTEGER PRIMARY KEY REFERENCES media_item(id) ON DELETE CASCADE,
    sharpness   REAL,
    megapixels  REAL,
    measured_at INTEGER NOT NULL
);
