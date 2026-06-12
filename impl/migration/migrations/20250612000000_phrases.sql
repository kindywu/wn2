-- ============================================================
-- phrases table — multi-word expressions, idioms, compound terms
-- separated from words (which are always single words)
-- ============================================================

CREATE TABLE phrases (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    phrase          CITEXT NOT NULL,
    phonetic        TEXT,
    phonetic_source data_source,
    audio           TEXT,
    collins         SMALLINT DEFAULT 0 CHECK (collins BETWEEN 0 AND 5),
    oxford          BOOLEAN DEFAULT false,
    bnc             INTEGER,
    frq             INTEGER,
    tags            TEXT[] DEFAULT '{}',
    exchange        JSONB,
    detail          JSONB,
    sources         TEXT[] NOT NULL DEFAULT '{}',
    curated         BOOLEAN NOT NULL DEFAULT false,
    curated_at      TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT uq_phrase UNIQUE (phrase)
);

CREATE INDEX idx_phrases_phrase      ON phrases (phrase);
CREATE INDEX idx_phrases_collins     ON phrases (collins);
CREATE INDEX idx_phrases_bnc         ON phrases (bnc);
CREATE INDEX idx_phrases_frq         ON phrases (frq);
CREATE INDEX idx_phrases_curated     ON phrases (curated);
CREATE INDEX idx_phrases_tags        ON phrases USING GIN (tags);
CREATE INDEX idx_phrases_sources     ON phrases USING GIN (sources);

CREATE TRIGGER trg_phrases_updated_at
    BEFORE UPDATE ON phrases
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE OR REPLACE FUNCTION set_phrases_curated_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.curated AND (OLD.curated IS NULL OR NOT OLD.curated) THEN
        NEW.curated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_phrases_curated_at
    BEFORE UPDATE ON phrases
    FOR EACH ROW EXECUTE FUNCTION set_phrases_curated_at();
