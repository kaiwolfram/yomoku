-- Analyzer evidence belongs to the vocabulary generation, not a live document.
-- Exact spelling is the identity; readings and repeated observations add no rows.
CREATE TABLE vocabulary_dictionary_forms (
    vocabulary_id INTEGER NOT NULL REFERENCES vocabulary(id) ON DELETE CASCADE,
    dictionary_form TEXT NOT NULL CHECK (length(CAST(dictionary_form AS BLOB)) > 0),
    PRIMARY KEY (vocabulary_id, dictionary_form)
) STRICT;

-- Recover only evidence still available at upgrade. Never create vocabulary or
-- infer missing forms from surface spelling, dictionary lookup or manual splits.
INSERT INTO vocabulary_dictionary_forms (vocabulary_id, dictionary_form)
    SELECT v.id, t.dictionary_form
    FROM vocabulary v JOIN analyzed_tokens t ON t.surface = v.surface
    WHERE t.is_japanese = 1 AND t.dictionary_form IS NOT NULL
    UNION
    SELECT vocabulary_id, dictionary_form FROM reading_occurrences
    WHERE dictionary_form IS NOT NULL;
