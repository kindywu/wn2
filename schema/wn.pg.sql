CREATE TABLE wn.adjpositions (
    sense_rowid integer NOT NULL,
    adjposition text NOT NULL
);

ALTER TABLE wn.adjpositions ADD CONSTRAINT adjpositions_sense_rowid_not_null NOT NULL sense_rowid;
ALTER TABLE wn.adjpositions ADD CONSTRAINT adjpositions_adjposition_not_null NOT NULL adjposition;
ALTER TABLE wn.adjpositions ADD CONSTRAINT adjpositions_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;

CREATE INDEX adjposition_sense_index ON wn.adjpositions USING btree (sense_rowid);

CREATE TABLE wn.counts (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    sense_rowid integer NOT NULL,
    count integer NOT NULL,
    metadata jsonb
);

ALTER TABLE wn.counts ADD CONSTRAINT counts_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.counts ADD CONSTRAINT counts_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.counts ADD CONSTRAINT counts_sense_rowid_not_null NOT NULL sense_rowid;
ALTER TABLE wn.counts ADD CONSTRAINT counts_count_not_null NOT NULL count;
ALTER TABLE wn.counts ADD CONSTRAINT counts_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.counts ADD CONSTRAINT counts_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.counts ADD CONSTRAINT counts_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;

CREATE INDEX count_index ON wn.counts USING btree (sense_rowid);

CREATE TABLE wn.definitions (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    synset_rowid integer NOT NULL,
    definition text,
    language text,
    sense_rowid integer,
    metadata jsonb
);

ALTER TABLE wn.definitions ADD CONSTRAINT definitions_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_synset_rowid_not_null NOT NULL synset_rowid;
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_synset_rowid_fkey FOREIGN KEY (synset_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;
ALTER TABLE wn.definitions ADD CONSTRAINT definitions_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE SET NULL;

CREATE INDEX definition_rowid_index ON wn.definitions USING btree (synset_rowid);
CREATE INDEX definition_sense_index ON wn.definitions USING btree (sense_rowid);

CREATE TABLE wn.entries (
    rowid bigint NOT NULL,
    id text NOT NULL,
    lexicon_rowid integer NOT NULL,
    pos text NOT NULL,
    metadata jsonb
);

ALTER TABLE wn.entries ADD CONSTRAINT entries_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.entries ADD CONSTRAINT entries_id_not_null NOT NULL id;
ALTER TABLE wn.entries ADD CONSTRAINT entries_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.entries ADD CONSTRAINT entries_pos_not_null NOT NULL pos;
ALTER TABLE wn.entries ADD CONSTRAINT entries_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.entries ADD CONSTRAINT entries_id_lexicon_rowid_key UNIQUE (id, lexicon_rowid);
ALTER TABLE wn.entries ADD CONSTRAINT entries_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;

CREATE INDEX entry_id_index ON wn.entries USING btree (id);

CREATE TABLE wn.entry_index (
    entry_rowid integer NOT NULL,
    lemma text NOT NULL
);

ALTER TABLE wn.entry_index ADD CONSTRAINT entry_index_entry_rowid_not_null NOT NULL entry_rowid;
ALTER TABLE wn.entry_index ADD CONSTRAINT entry_index_lemma_not_null NOT NULL lemma;
ALTER TABLE wn.entry_index ADD CONSTRAINT entry_index_entry_rowid_key UNIQUE (entry_rowid);
ALTER TABLE wn.entry_index ADD CONSTRAINT entry_index_entry_rowid_fkey FOREIGN KEY (entry_rowid) REFERENCES wn.entries(rowid) ON DELETE CASCADE;

CREATE INDEX entry_index_entry_index ON wn.entry_index USING btree (entry_rowid);
CREATE INDEX entry_index_lemma_index ON wn.entry_index USING btree (lemma);

CREATE TABLE wn.forms (
    rowid bigint NOT NULL,
    id text,
    lexicon_rowid integer NOT NULL,
    entry_rowid integer NOT NULL,
    form text NOT NULL,
    normalized_form text,
    script text,
    rank integer DEFAULT 1
);

ALTER TABLE wn.forms ADD CONSTRAINT forms_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.forms ADD CONSTRAINT forms_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.forms ADD CONSTRAINT forms_entry_rowid_not_null NOT NULL entry_rowid;
ALTER TABLE wn.forms ADD CONSTRAINT forms_form_not_null NOT NULL form;
ALTER TABLE wn.forms ADD CONSTRAINT forms_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.forms ADD CONSTRAINT forms_entry_rowid_form_script_key UNIQUE (entry_rowid, form, script);
ALTER TABLE wn.forms ADD CONSTRAINT forms_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.forms ADD CONSTRAINT forms_entry_rowid_fkey FOREIGN KEY (entry_rowid) REFERENCES wn.entries(rowid) ON DELETE CASCADE;

CREATE INDEX form_entry_index ON wn.forms USING btree (entry_rowid);
CREATE INDEX form_index ON wn.forms USING btree (form);
CREATE INDEX form_norm_index ON wn.forms USING btree (normalized_form);

CREATE TABLE wn.ili_statuses (
    rowid bigint NOT NULL,
    status text NOT NULL
);

ALTER TABLE wn.ili_statuses ADD CONSTRAINT ili_statuses_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.ili_statuses ADD CONSTRAINT ili_statuses_status_not_null NOT NULL status;
ALTER TABLE wn.ili_statuses ADD CONSTRAINT ili_statuses_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.ili_statuses ADD CONSTRAINT ili_statuses_status_key UNIQUE (status);

CREATE INDEX ili_status_index ON wn.ili_statuses USING btree (status);

CREATE TABLE wn.ilis (
    rowid bigint NOT NULL,
    id text NOT NULL,
    status_rowid integer NOT NULL,
    definition text,
    metadata jsonb
);

ALTER TABLE wn.ilis ADD CONSTRAINT ilis_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.ilis ADD CONSTRAINT ilis_id_not_null NOT NULL id;
ALTER TABLE wn.ilis ADD CONSTRAINT ilis_status_rowid_not_null NOT NULL status_rowid;
ALTER TABLE wn.ilis ADD CONSTRAINT ilis_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.ilis ADD CONSTRAINT ilis_id_key UNIQUE (id);
ALTER TABLE wn.ilis ADD CONSTRAINT ilis_status_rowid_fkey FOREIGN KEY (status_rowid) REFERENCES wn.ili_statuses(rowid);

CREATE INDEX ili_id_index ON wn.ilis USING btree (id);

CREATE TABLE wn.lexfiles (
    rowid bigint NOT NULL,
    name text NOT NULL
);

ALTER TABLE wn.lexfiles ADD CONSTRAINT lexfiles_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.lexfiles ADD CONSTRAINT lexfiles_name_not_null NOT NULL name;
ALTER TABLE wn.lexfiles ADD CONSTRAINT lexfiles_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.lexfiles ADD CONSTRAINT lexfiles_name_key UNIQUE (name);

CREATE INDEX lexfile_index ON wn.lexfiles USING btree (name);

CREATE TABLE wn.lexicon_dependencies (
    dependent_rowid integer NOT NULL,
    provider_id text NOT NULL,
    provider_version text NOT NULL,
    provider_url text,
    provider_rowid integer
);

ALTER TABLE wn.lexicon_dependencies ADD CONSTRAINT lexicon_dependencies_dependent_rowid_not_null NOT NULL dependent_rowid;
ALTER TABLE wn.lexicon_dependencies ADD CONSTRAINT lexicon_dependencies_provider_id_not_null NOT NULL provider_id;
ALTER TABLE wn.lexicon_dependencies ADD CONSTRAINT lexicon_dependencies_provider_version_not_null NOT NULL provider_version;
ALTER TABLE wn.lexicon_dependencies ADD CONSTRAINT lexicon_dependencies_dependent_rowid_fkey FOREIGN KEY (dependent_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.lexicon_dependencies ADD CONSTRAINT lexicon_dependencies_provider_rowid_fkey FOREIGN KEY (provider_rowid) REFERENCES wn.lexicons(rowid) ON DELETE SET NULL;

CREATE INDEX lexicon_dependent_index ON wn.lexicon_dependencies USING btree (dependent_rowid);

CREATE TABLE wn.lexicon_extensions (
    extension_rowid integer NOT NULL,
    base_id text NOT NULL,
    base_version text NOT NULL,
    base_url text,
    base_rowid integer
);

ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_extension_rowid_not_null NOT NULL extension_rowid;
ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_base_id_not_null NOT NULL base_id;
ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_base_version_not_null NOT NULL base_version;
ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_extension_rowid_base_rowid_key UNIQUE (extension_rowid, base_rowid);
ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_extension_rowid_fkey FOREIGN KEY (extension_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.lexicon_extensions ADD CONSTRAINT lexicon_extensions_base_rowid_fkey FOREIGN KEY (base_rowid) REFERENCES wn.lexicons(rowid);

CREATE INDEX lexicon_extension_index ON wn.lexicon_extensions USING btree (extension_rowid);

CREATE TABLE wn.lexicons (
    rowid bigint NOT NULL,
    specifier text NOT NULL,
    id text NOT NULL,
    label text NOT NULL,
    language text NOT NULL,
    email text NOT NULL,
    license text NOT NULL,
    version text NOT NULL,
    url text,
    citation text,
    logo text,
    metadata jsonb,
    modified boolean NOT NULL DEFAULT false
);

ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_specifier_not_null NOT NULL specifier;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_id_not_null NOT NULL id;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_label_not_null NOT NULL label;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_language_not_null NOT NULL language;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_email_not_null NOT NULL email;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_license_not_null NOT NULL license;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_version_not_null NOT NULL version;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_modified_not_null NOT NULL modified;
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_id_version_key UNIQUE (id, version);
ALTER TABLE wn.lexicons ADD CONSTRAINT lexicons_specifier_key UNIQUE (specifier);

CREATE INDEX lexicon_specifier_index ON wn.lexicons USING btree (specifier);

CREATE TABLE wn.pronunciations (
    form_rowid integer NOT NULL,
    lexicon_rowid integer NOT NULL,
    value text,
    variety text,
    notation text,
    phonemic boolean NOT NULL DEFAULT true,
    audio text
);

ALTER TABLE wn.pronunciations ADD CONSTRAINT pronunciations_form_rowid_not_null NOT NULL form_rowid;
ALTER TABLE wn.pronunciations ADD CONSTRAINT pronunciations_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.pronunciations ADD CONSTRAINT pronunciations_phonemic_not_null NOT NULL phonemic;
ALTER TABLE wn.pronunciations ADD CONSTRAINT pronunciations_form_rowid_fkey FOREIGN KEY (form_rowid) REFERENCES wn.forms(rowid) ON DELETE CASCADE;
ALTER TABLE wn.pronunciations ADD CONSTRAINT pronunciations_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;

CREATE INDEX pronunciation_form_index ON wn.pronunciations USING btree (form_rowid);

CREATE TABLE wn.proposed_ilis (
    rowid bigint NOT NULL,
    synset_rowid integer,
    definition text,
    metadata jsonb
);

ALTER TABLE wn.proposed_ilis ADD CONSTRAINT proposed_ilis_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.proposed_ilis ADD CONSTRAINT proposed_ilis_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.proposed_ilis ADD CONSTRAINT proposed_ilis_synset_rowid_key UNIQUE (synset_rowid);
ALTER TABLE wn.proposed_ilis ADD CONSTRAINT proposed_ilis_synset_rowid_fkey FOREIGN KEY (synset_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;

CREATE INDEX proposed_ili_synset_rowid_index ON wn.proposed_ilis USING btree (synset_rowid);

CREATE TABLE wn.relation_types (
    rowid bigint NOT NULL,
    type text NOT NULL
);

ALTER TABLE wn.relation_types ADD CONSTRAINT relation_types_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.relation_types ADD CONSTRAINT relation_types_type_not_null NOT NULL type;
ALTER TABLE wn.relation_types ADD CONSTRAINT relation_types_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.relation_types ADD CONSTRAINT relation_types_type_key UNIQUE (type);

CREATE INDEX relation_type_index ON wn.relation_types USING btree (type);

CREATE TABLE wn.sense_examples (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    sense_rowid integer NOT NULL,
    example text,
    language text,
    metadata jsonb
);

ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_sense_rowid_not_null NOT NULL sense_rowid;
ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_examples ADD CONSTRAINT sense_examples_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;

CREATE INDEX sense_example_index ON wn.sense_examples USING btree (sense_rowid);

CREATE TABLE wn.sense_relations (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    source_rowid integer NOT NULL,
    target_rowid integer NOT NULL,
    type_rowid integer NOT NULL,
    metadata jsonb
);

ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_source_rowid_not_null NOT NULL source_rowid;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_target_rowid_not_null NOT NULL target_rowid;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_type_rowid_not_null NOT NULL type_rowid;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_source_rowid_fkey FOREIGN KEY (source_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_target_rowid_fkey FOREIGN KEY (target_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_relations ADD CONSTRAINT sense_relations_type_rowid_fkey FOREIGN KEY (type_rowid) REFERENCES wn.relation_types(rowid);

CREATE INDEX sense_relation_source_index ON wn.sense_relations USING btree (source_rowid);
CREATE INDEX sense_relation_target_index ON wn.sense_relations USING btree (target_rowid);

CREATE TABLE wn.sense_synset_relations (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    source_rowid integer NOT NULL,
    target_rowid integer NOT NULL,
    type_rowid integer NOT NULL,
    metadata jsonb
);

ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_source_rowid_not_null NOT NULL source_rowid;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_target_rowid_not_null NOT NULL target_rowid;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_type_rowid_not_null NOT NULL type_rowid;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_source_rowid_fkey FOREIGN KEY (source_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_target_rowid_fkey FOREIGN KEY (target_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;
ALTER TABLE wn.sense_synset_relations ADD CONSTRAINT sense_synset_relations_type_rowid_fkey FOREIGN KEY (type_rowid) REFERENCES wn.relation_types(rowid);

CREATE INDEX sense_synset_relation_source_index ON wn.sense_synset_relations USING btree (source_rowid);
CREATE INDEX sense_synset_relation_target_index ON wn.sense_synset_relations USING btree (target_rowid);

CREATE TABLE wn.senses (
    rowid bigint NOT NULL,
    id text NOT NULL,
    lexicon_rowid integer NOT NULL,
    entry_rowid integer NOT NULL,
    entry_rank integer DEFAULT 1,
    synset_rowid integer NOT NULL,
    synset_rank integer DEFAULT 1,
    metadata jsonb
);

ALTER TABLE wn.senses ADD CONSTRAINT senses_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.senses ADD CONSTRAINT senses_id_not_null NOT NULL id;
ALTER TABLE wn.senses ADD CONSTRAINT senses_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.senses ADD CONSTRAINT senses_entry_rowid_not_null NOT NULL entry_rowid;
ALTER TABLE wn.senses ADD CONSTRAINT senses_synset_rowid_not_null NOT NULL synset_rowid;
ALTER TABLE wn.senses ADD CONSTRAINT senses_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.senses ADD CONSTRAINT senses_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.senses ADD CONSTRAINT senses_entry_rowid_fkey FOREIGN KEY (entry_rowid) REFERENCES wn.entries(rowid) ON DELETE CASCADE;
ALTER TABLE wn.senses ADD CONSTRAINT senses_synset_rowid_fkey FOREIGN KEY (synset_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;

CREATE INDEX sense_entry_rowid_index ON wn.senses USING btree (entry_rowid);
CREATE INDEX sense_id_index ON wn.senses USING btree (id);
CREATE INDEX sense_synset_rowid_index ON wn.senses USING btree (synset_rowid);

CREATE TABLE wn.synset_examples (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    synset_rowid integer NOT NULL,
    example text,
    language text,
    metadata jsonb
);

ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_synset_rowid_not_null NOT NULL synset_rowid;
ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.synset_examples ADD CONSTRAINT synset_examples_synset_rowid_fkey FOREIGN KEY (synset_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;

CREATE INDEX synset_example_rowid_index ON wn.synset_examples USING btree (synset_rowid);

CREATE TABLE wn.synset_relations (
    rowid bigint NOT NULL,
    lexicon_rowid integer NOT NULL,
    source_rowid integer NOT NULL,
    target_rowid integer NOT NULL,
    type_rowid integer NOT NULL,
    metadata jsonb
);

ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_source_rowid_not_null NOT NULL source_rowid;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_target_rowid_not_null NOT NULL target_rowid;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_type_rowid_not_null NOT NULL type_rowid;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_source_rowid_fkey FOREIGN KEY (source_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_target_rowid_fkey FOREIGN KEY (target_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;
ALTER TABLE wn.synset_relations ADD CONSTRAINT synset_relations_type_rowid_fkey FOREIGN KEY (type_rowid) REFERENCES wn.relation_types(rowid);

CREATE INDEX synset_relation_source_index ON wn.synset_relations USING btree (source_rowid);
CREATE INDEX synset_relation_target_index ON wn.synset_relations USING btree (target_rowid);

CREATE TABLE wn.synsets (
    rowid bigint NOT NULL,
    id text NOT NULL,
    lexicon_rowid integer NOT NULL,
    ili_rowid integer,
    pos text,
    lexfile_rowid integer,
    metadata jsonb
);

ALTER TABLE wn.synsets ADD CONSTRAINT synsets_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_id_not_null NOT NULL id;
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_ili_rowid_fkey FOREIGN KEY (ili_rowid) REFERENCES wn.ilis(rowid);
ALTER TABLE wn.synsets ADD CONSTRAINT synsets_lexfile_rowid_fkey FOREIGN KEY (lexfile_rowid) REFERENCES wn.lexfiles(rowid);

CREATE INDEX synset_id_index ON wn.synsets USING btree (id);
CREATE INDEX synset_ili_rowid_index ON wn.synsets USING btree (ili_rowid);

CREATE TABLE wn.syntactic_behaviour_senses (
    syntactic_behaviour_rowid integer NOT NULL,
    sense_rowid integer NOT NULL
);

ALTER TABLE wn.syntactic_behaviour_senses ADD CONSTRAINT syntactic_behaviour_senses_syntactic_behaviour_rowid_not_null NOT NULL syntactic_behaviour_rowid;
ALTER TABLE wn.syntactic_behaviour_senses ADD CONSTRAINT syntactic_behaviour_senses_sense_rowid_not_null NOT NULL sense_rowid;
ALTER TABLE wn.syntactic_behaviour_senses ADD CONSTRAINT syntactic_behaviour_senses_syntactic_behaviour_rowid_fkey FOREIGN KEY (syntactic_behaviour_rowid) REFERENCES wn.syntactic_behaviours(rowid) ON DELETE CASCADE;
ALTER TABLE wn.syntactic_behaviour_senses ADD CONSTRAINT syntactic_behaviour_senses_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;

CREATE INDEX syntactic_behaviour_sense_sb_index ON wn.syntactic_behaviour_senses USING btree (syntactic_behaviour_rowid);
CREATE INDEX syntactic_behaviour_sense_sense_index ON wn.syntactic_behaviour_senses USING btree (sense_rowid);

CREATE TABLE wn.syntactic_behaviours (
    rowid bigint NOT NULL,
    id text,
    lexicon_rowid integer NOT NULL,
    frame text NOT NULL
);

ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_rowid_not_null NOT NULL rowid;
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_frame_not_null NOT NULL frame;
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_pkey PRIMARY KEY (rowid);
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_lexicon_rowid_id_key UNIQUE (lexicon_rowid, id);
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_lexicon_rowid_frame_key UNIQUE (lexicon_rowid, frame);
ALTER TABLE wn.syntactic_behaviours ADD CONSTRAINT syntactic_behaviours_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;

CREATE INDEX syntactic_behaviour_id_index ON wn.syntactic_behaviours USING btree (id);

CREATE TABLE wn.tags (
    form_rowid integer NOT NULL,
    lexicon_rowid integer NOT NULL,
    tag text,
    category text
);

ALTER TABLE wn.tags ADD CONSTRAINT tags_form_rowid_not_null NOT NULL form_rowid;
ALTER TABLE wn.tags ADD CONSTRAINT tags_lexicon_rowid_not_null NOT NULL lexicon_rowid;
ALTER TABLE wn.tags ADD CONSTRAINT tags_form_rowid_fkey FOREIGN KEY (form_rowid) REFERENCES wn.forms(rowid) ON DELETE CASCADE;
ALTER TABLE wn.tags ADD CONSTRAINT tags_lexicon_rowid_fkey FOREIGN KEY (lexicon_rowid) REFERENCES wn.lexicons(rowid) ON DELETE CASCADE;

CREATE INDEX tag_form_index ON wn.tags USING btree (form_rowid);

CREATE TABLE wn.unlexicalized_senses (
    sense_rowid integer NOT NULL
);

ALTER TABLE wn.unlexicalized_senses ADD CONSTRAINT unlexicalized_senses_sense_rowid_not_null NOT NULL sense_rowid;
ALTER TABLE wn.unlexicalized_senses ADD CONSTRAINT unlexicalized_senses_sense_rowid_fkey FOREIGN KEY (sense_rowid) REFERENCES wn.senses(rowid) ON DELETE CASCADE;

CREATE INDEX unlexicalized_senses_index ON wn.unlexicalized_senses USING btree (sense_rowid);

CREATE TABLE wn.unlexicalized_synsets (
    synset_rowid integer NOT NULL
);

ALTER TABLE wn.unlexicalized_synsets ADD CONSTRAINT unlexicalized_synsets_synset_rowid_not_null NOT NULL synset_rowid;
ALTER TABLE wn.unlexicalized_synsets ADD CONSTRAINT unlexicalized_synsets_synset_rowid_fkey FOREIGN KEY (synset_rowid) REFERENCES wn.synsets(rowid) ON DELETE CASCADE;

CREATE INDEX unlexicalized_synsets_index ON wn.unlexicalized_synsets USING btree (synset_rowid);
