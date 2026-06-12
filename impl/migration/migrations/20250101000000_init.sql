-- ============================================================
-- dict.db baseline migration
-- ============================================================
CREATE EXTENSION IF NOT EXISTS citext;

-- ============================================================
-- Enum types
-- ============================================================
CREATE TYPE data_source    AS ENUM ('stardict', 'wn', 'llm', 'human');
CREATE TYPE pos_type       AS ENUM ('n', 'v', 'a', 'r', 's');
CREATE TYPE form_type      AS ENUM (
    'plural', 'past', 'past_participle',
    'present_participle', 'third_person',
    'comparative', 'superlative'
);
CREATE TYPE change_action  AS ENUM ('create', 'update', 'delete', 'review', 'unreview');
CREATE TYPE change_status  AS ENUM ('pending', 'approved', 'rejected');
CREATE TYPE severity_level AS ENUM ('critical', 'warning', 'info');
CREATE TYPE import_mode    AS ENUM ('full', 'incremental');
CREATE TYPE import_status  AS ENUM ('running', 'completed', 'failed');

-- ============================================================
-- Shared trigger functions
-- ============================================================
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION data_quality_priority(
    source data_source, reviewed boolean, modified boolean
) RETURNS integer AS $$
BEGIN
    IF source = 'human' OR modified THEN RETURN 0; END IF;
    IF source IN ('stardict', 'wn') AND reviewed THEN RETURN 1; END IF;
    IF source IN ('stardict', 'wn') AND NOT reviewed THEN RETURN 2; END IF;
    IF source = 'llm' AND reviewed THEN RETURN 3; END IF;
    RETURN 4;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- ============================================================
-- import_log (must be before pending_changes)
-- ============================================================
CREATE TABLE import_log (
    id                   BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    source               TEXT NOT NULL CHECK (source IN ('stardict', 'wn')),
    source_version       TEXT,
    mode                 import_mode NOT NULL,
    started_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at          TIMESTAMPTZ,
    duration_seconds     INTEGER,
    new_words            INTEGER DEFAULT 0,
    updated_words        INTEGER DEFAULT 0,
    new_definitions      INTEGER DEFAULT 0,
    updated_definitions  INTEGER DEFAULT 0,
    new_translations     INTEGER DEFAULT 0,
    updated_translations INTEGER DEFAULT 0,
    new_examples         INTEGER DEFAULT 0,
    updated_examples     INTEGER DEFAULT 0,
    new_relations        INTEGER DEFAULT 0,
    updated_relations    INTEGER DEFAULT 0,
    new_forms            INTEGER DEFAULT 0,
    updated_forms        INTEGER DEFAULT 0,
    conflicts            INTEGER DEFAULT 0,
    status               import_status NOT NULL DEFAULT 'running',
    error                TEXT
);
CREATE INDEX idx_import_log_source ON import_log (source);
CREATE INDEX idx_import_log_status ON import_log (status);

-- ============================================================
-- words
-- ============================================================
CREATE TABLE words (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word            CITEXT NOT NULL,
    pos             pos_type,
    pos_source      data_source,
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
    CONSTRAINT uq_word_pos UNIQUE NULLS NOT DISTINCT (word, pos)
);
CREATE INDEX idx_words_word    ON words (word);
CREATE INDEX idx_words_pos     ON words (pos);
CREATE INDEX idx_words_collins ON words (collins);
CREATE INDEX idx_words_bnc     ON words (bnc);
CREATE INDEX idx_words_frq     ON words (frq);
CREATE INDEX idx_words_curated ON words (curated);
CREATE INDEX idx_words_tags    ON words USING GIN (tags);
CREATE INDEX idx_words_sources ON words USING GIN (sources);

CREATE TRIGGER trg_words_updated_at
BEFORE UPDATE ON words FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE OR REPLACE FUNCTION set_curated_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.curated = true AND (OLD.curated = false OR OLD.curated IS NULL) THEN
        NEW.curated_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_words_curated_at
BEFORE UPDATE ON words FOR EACH ROW EXECUTE FUNCTION set_curated_at();

-- ============================================================
-- definitions
-- ============================================================
CREATE TABLE definitions (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word_id     BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    text_hash   TEXT NOT NULL,
    source      data_source NOT NULL,
    reviewed    BOOLEAN NOT NULL DEFAULT false,
    reviewed_at TIMESTAMPTZ,
    modified    BOOLEAN NOT NULL DEFAULT false,
    modified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE NULLS NOT DISTINCT (word_id, text_hash, source)
);
CREATE INDEX idx_definitions_word     ON definitions (word_id);
CREATE INDEX idx_definitions_source   ON definitions (source);
CREATE INDEX idx_definitions_reviewed ON definitions (reviewed);

CREATE TRIGGER trg_definitions_updated_at
BEFORE UPDATE ON definitions FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- translations
-- ============================================================
CREATE TABLE translations (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word_id     BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text        TEXT NOT NULL,
    text_hash   TEXT NOT NULL,
    language    TEXT NOT NULL DEFAULT 'zh',
    source      data_source NOT NULL,
    reviewed    BOOLEAN NOT NULL DEFAULT false,
    reviewed_at TIMESTAMPTZ,
    modified    BOOLEAN NOT NULL DEFAULT false,
    modified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE NULLS NOT DISTINCT (word_id, text_hash, language, source)
);
CREATE INDEX idx_translations_word     ON translations (word_id);
CREATE INDEX idx_translations_source   ON translations (source);
CREATE INDEX idx_translations_reviewed ON translations (reviewed);

CREATE TRIGGER trg_translations_updated_at
BEFORE UPDATE ON translations FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- examples
-- ============================================================
CREATE TABLE examples (
    id                BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word_id           BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    text              TEXT NOT NULL,
    text_hash         TEXT NOT NULL,
    translation       TEXT,
    source            data_source NOT NULL,
    reviewed          BOOLEAN NOT NULL DEFAULT false,
    reviewed_at       TIMESTAMPTZ,
    modified          BOOLEAN NOT NULL DEFAULT false,
    modified_at       TIMESTAMPTZ,
    trans_source      data_source,
    trans_reviewed    BOOLEAN NOT NULL DEFAULT false,
    trans_reviewed_at TIMESTAMPTZ,
    trans_modified    BOOLEAN NOT NULL DEFAULT false,
    trans_modified_at TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE NULLS NOT DISTINCT (word_id, text_hash)
);
CREATE INDEX idx_examples_word     ON examples (word_id);
CREATE INDEX idx_examples_source   ON examples (source);
CREATE INDEX idx_examples_reviewed ON examples (reviewed);

CREATE TRIGGER trg_examples_updated_at
BEFORE UPDATE ON examples FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- word_relations
-- ============================================================
CREATE TABLE word_relations (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word_id       BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    related_word  CITEXT NOT NULL,
    relation_type TEXT NOT NULL
                  CHECK (relation_type IN (
                      'synonym','antonym','similar',
                      'hypernym','hyponym','instance_hypernym','instance_hyponym',
                      'holo_part','mero_part','holo_member','mero_member','holo_substance','mero_substance',
                      'causes','is_caused_by','entails','is_entailed_by',
                      'derivation','participle','pertainym',
                      'domain_region','has_domain_region','domain_topic','has_domain_topic',
                      'also','attribute','exemplifies','is_exemplified_by','other'
                  )),
    source        data_source NOT NULL DEFAULT 'wn',
    reviewed      BOOLEAN NOT NULL DEFAULT false,
    reviewed_at   TIMESTAMPTZ,
    modified      BOOLEAN NOT NULL DEFAULT false,
    modified_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (word_id, related_word, relation_type)
);
CREATE INDEX idx_relations_word     ON word_relations (word_id);
CREATE INDEX idx_relations_related  ON word_relations (related_word);
CREATE INDEX idx_relations_type     ON word_relations (relation_type);
CREATE INDEX idx_relations_reviewed ON word_relations (reviewed);

CREATE TRIGGER trg_word_relations_updated_at
BEFORE UPDATE ON word_relations FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- word_forms
-- ============================================================
CREATE TABLE word_forms (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    word_id     BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    form        TEXT NOT NULL,
    form_type   form_type NOT NULL,
    source      data_source NOT NULL DEFAULT 'stardict'
                CHECK (source <> 'llm'),
    reviewed    BOOLEAN NOT NULL DEFAULT false,
    reviewed_at TIMESTAMPTZ,
    modified    BOOLEAN NOT NULL DEFAULT false,
    modified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (word_id, form, form_type)
);
CREATE INDEX idx_wordforms_word ON word_forms (word_id);

CREATE TRIGGER trg_word_forms_updated_at
BEFORE UPDATE ON word_forms FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- ============================================================
-- evaluations (before function that references it)
-- ============================================================
CREATE TABLE evaluations (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    target_table TEXT NOT NULL
                 CHECK (target_table IN ('definitions','translations','examples','word_relations')),
    target_id    BIGINT NOT NULL,
    word_id      BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    dimension    TEXT NOT NULL,
    score        SMALLINT NOT NULL CHECK (score BETWEEN 1 AND 5),
    comment      TEXT,
    suggestion   TEXT,
    severity     severity_level NOT NULL DEFAULT 'info',
    reviewed     BOOLEAN NOT NULL DEFAULT false,
    reviewed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_evaluations_target   ON evaluations (target_table, target_id);
CREATE INDEX idx_evaluations_word     ON evaluations (word_id);
CREATE INDEX idx_evaluations_score    ON evaluations (score);
CREATE INDEX idx_evaluations_reviewed ON evaluations (reviewed);

CREATE OR REPLACE FUNCTION check_evaluation_target()
RETURNS TRIGGER AS $$
BEGIN
    CASE NEW.target_table
        WHEN 'definitions'    THEN PERFORM 1 FROM definitions    WHERE id = NEW.target_id;
        WHEN 'translations'   THEN PERFORM 1 FROM translations   WHERE id = NEW.target_id;
        WHEN 'examples'       THEN PERFORM 1 FROM examples       WHERE id = NEW.target_id;
        WHEN 'word_relations' THEN PERFORM 1 FROM word_relations WHERE id = NEW.target_id;
        ELSE RAISE EXCEPTION 'Unknown target_table: %', NEW.target_table;
    END CASE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'target_id % not found in table %', NEW.target_id, NEW.target_table;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_evaluations_target_check
BEFORE INSERT OR UPDATE ON evaluations
FOR EACH ROW EXECUTE FUNCTION check_evaluation_target();

-- ============================================================
-- pending_changes
-- ============================================================
CREATE TABLE pending_changes (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    table_name    TEXT NOT NULL
                  CHECK (table_name IN ('words','definitions','translations','examples','word_relations','word_forms')),
    row_id        BIGINT,
    word_id       BIGINT NOT NULL REFERENCES words(id) ON DELETE CASCADE,
    field         TEXT,
    old_value     TEXT,
    new_value     TEXT,
    source        TEXT NOT NULL CHECK (source IN ('stardict', 'wn')),
    import_log_id BIGINT REFERENCES import_log(id),
    status        change_status NOT NULL DEFAULT 'pending',
    resolved_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_pending_word       ON pending_changes (word_id);
CREATE INDEX idx_pending_status     ON pending_changes (status);
CREATE INDEX idx_pending_import_log ON pending_changes (import_log_id);

-- ============================================================
-- change_log
-- ============================================================
CREATE TABLE change_log (
    id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    table_name TEXT NOT NULL
               CHECK (table_name IN ('words','definitions','translations','examples','word_relations','word_forms')),
    row_id     BIGINT NOT NULL,
    field      TEXT,
    old_value  TEXT,
    new_value  TEXT,
    action     change_action NOT NULL,
    operator   TEXT NOT NULL DEFAULT 'system',
    changed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_changelog_table_row  ON change_log (table_name, row_id);
CREATE INDEX idx_changelog_changed_at ON change_log (changed_at);
