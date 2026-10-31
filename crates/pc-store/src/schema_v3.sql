CREATE TABLE face_exclusion (
    id INTEGER PRIMARY KEY,
    media_id INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    bbox TEXT NOT NULL
);
CREATE INDEX face_exclusion_media ON face_exclusion(media_id);
CREATE TABLE person_centroid (
    person_id INTEGER NOT NULL REFERENCES person(id) ON DELETE CASCADE,
    model_ver TEXT NOT NULL,
    vector BLOB NOT NULL,
    PRIMARY KEY(person_id,model_ver)
);
