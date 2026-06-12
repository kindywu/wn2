export interface WordOut {
  id: number
  word: string
  pos: 'n' | 'v' | 'a' | 'r' | 's' | null
  phonetic: string | null
  collins: number
  oxford: boolean
  bnc: number | null
  frq: number | null
  tags: string[]
  curated: boolean
  curated_at: string | null
  created_at: string
  updated_at: string
}

export interface DefinitionOut {
  id: number
  word_id: number
  text: string
  source: string
  reviewed: boolean
  modified: boolean
  created_at: string
  updated_at: string
}

export interface TranslationOut {
  id: number
  word_id: number
  text: string
  language: string
  source: string
  reviewed: boolean
  modified: boolean
  created_at: string
  updated_at: string
}

export interface ExampleOut {
  id: number
  word_id: number
  text: string
  translation: string | null
  source: string
  reviewed: boolean
  modified: boolean
  trans_source: string | null
  trans_reviewed: boolean
  trans_modified: boolean
  created_at: string
  updated_at: string
}

export interface WordRelationOut {
  id: number
  word_id: number
  related_word: string
  relation_type: string
  source: string
  reviewed: boolean
  modified: boolean
  created_at: string
  updated_at: string
}

export interface WordFormOut {
  id: number
  word_id: number
  form: string
  form_type: string
  source: string
  reviewed: boolean
  modified: boolean
  created_at: string
  updated_at: string
}

export interface PendingChangeOut {
  id: number
  table_name: string
  row_id: number | null
  word_id: number
  field: string | null
  old_value: string | null
  new_value: string | null
  source: string
  status: string
  created_at: string
}

export interface EvaluationOut {
  id: number
  target_table: string
  target_id: number
  word_id: number
  dimension: string
  score: number
  comment: string | null
  suggestion: string | null
  severity: string
  reviewed: boolean
  created_at: string
}

export interface ImportLogOut {
  id: number
  source: string
  source_version: string | null
  mode: string
  started_at: string
  finished_at: string | null
  new_words: number
  updated_words: number
  conflicts: number
  status: string
}

export interface ChangeLogOut {
  id: number
  table_name: string
  row_id: number
  field: string | null
  old_value: string | null
  new_value: string | null
  action: string
  operator: string
  changed_at: string
}

export interface ApiResponse<T> {
  data: T
  meta?: { total: number; limit: number; offset: number }
}

export interface ApiError {
  error: { code: string; message: string }
}
