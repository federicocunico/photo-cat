-- v12: etichette automatiche rifiutate dall'utente (media, gruppo, valore).
-- La rianalisi e la rivalutazione delle soglie non le riaggiungono; assegnare
-- a mano lo stesso valore annulla il rifiuto.

CREATE TABLE label_rejection (
    media_id   INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    grp        TEXT    NOT NULL,
    value      TEXT    NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (media_id, grp, value)
);
