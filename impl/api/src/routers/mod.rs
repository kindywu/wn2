use axum::{Router, routing::{get, patch, post}};
use crate::state::AppState;

mod words;
mod definitions;
mod translations;
mod examples;
mod relations;
mod forms;
mod pending;
mod evaluations;
mod logs;

pub fn build_router() -> Router<AppState> {
    Router::new()
        // Health check
        .route("/healthz", get(healthz))
        // Words
        .route("/api/v1/words", get(words::search_words))
        .route("/api/v1/words/{id}", get(words::get_word))
        .route("/api/v1/words/{id}/curate", patch(words::curate_word))
        .route("/api/v1/words/{id}/definitions", get(words::get_word_definitions))
        .route("/api/v1/words/{id}/translations", get(words::get_word_translations))
        .route("/api/v1/words/{id}/examples", get(words::get_word_examples))
        .route("/api/v1/words/{id}/relations", get(words::get_word_relations))
        .route("/api/v1/words/{id}/forms", get(words::get_word_forms))
        // Definitions CRUD
        .route("/api/v1/definitions/{id}/review", patch(definitions::review_definition))
        .route("/api/v1/definitions/{id}", patch(definitions::update_definition))
        .route("/api/v1/words/{id}/definitions", post(definitions::create_definition))
        // Translations CRUD
        .route("/api/v1/translations/{id}/review", patch(translations::review_translation))
        .route("/api/v1/translations/{id}", patch(translations::update_translation))
        .route("/api/v1/words/{id}/translations", post(translations::create_translation))
        // Examples CRUD
        .route("/api/v1/examples/{id}/review", patch(examples::review_example))
        .route("/api/v1/examples/{id}", patch(examples::update_example))
        .route("/api/v1/words/{id}/examples", post(examples::create_example))
        // Relations CRUD
        .route("/api/v1/word-relations/{id}/review", patch(relations::review_relation))
        .route("/api/v1/word-relations/{id}", patch(relations::update_relation))
        .route("/api/v1/words/{id}/relations", post(relations::create_relation))
        // Forms CRUD
        .route("/api/v1/word-forms/{id}/review", patch(forms::review_form))
        .route("/api/v1/word-forms/{id}", patch(forms::update_form))
        // Pending changes
        .route("/api/v1/pending-changes", get(pending::list_pending))
        .route("/api/v1/pending-changes/{id}/approve", patch(pending::approve_pending))
        .route("/api/v1/pending-changes/{id}/reject", patch(pending::reject_pending))
        // Evaluations
        .route("/api/v1/evaluations", get(evaluations::list_evaluations))
        .route("/api/v1/evaluations/{id}/agree", patch(evaluations::agree_evaluation))
        .route("/api/v1/evaluations/{id}/dismiss", patch(evaluations::dismiss_evaluation))
        // Logs
        .route("/api/v1/import-logs", get(logs::list_import_logs))
        .route("/api/v1/import-logs/{id}", get(logs::get_import_log))
        .route("/api/v1/change-log", get(logs::list_change_logs))
}

async fn healthz() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok"}))
}
