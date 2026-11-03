-- v9: indici sull'ordinamento effettivo dei volti (qualità con ripiego, poi id decrescente),
-- così le pagine del modulo Persone e le copertine non ordinano tutto il gruppo in memoria.

DROP INDEX IF EXISTS face_person_quality;
DROP INDEX IF EXISTS face_quality_score;

CREATE INDEX face_person_rank ON face(person_id, COALESCE(quality_score, quality) DESC, id DESC);
CREATE INDEX face_rank ON face(COALESCE(quality_score, quality) DESC, id DESC);
