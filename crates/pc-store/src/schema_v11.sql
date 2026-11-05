-- v11: hash percettivo (pHash a 64 bit, DCT) delle foto, per trovare le quasi-duplicate.
-- Nullable e senza indice: i valori si calcolano all'import e con il backfill.

ALTER TABLE media_item ADD COLUMN phash INTEGER;
