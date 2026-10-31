CREATE TABLE analysis_version (
    media_id INTEGER NOT NULL REFERENCES media_item(id) ON DELETE CASCADE,
    stage TEXT NOT NULL,
    model_ver TEXT NOT NULL,
    taxonomy_ver INTEGER,
    PRIMARY KEY(media_id, stage)
);
CREATE INDEX analysis_stage_version ON analysis_version(stage, model_ver, taxonomy_ver);
