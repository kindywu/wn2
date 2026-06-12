use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTaskType {
    Generate,
    Evaluate,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn generate(&self, prompt: &str, task: LlmTaskType) -> anyhow::Result<String>;
}

/// Helper: call generate() and parse the result as JSON
pub async fn generate_json<C: LlmClient + ?Sized, T: serde::de::DeserializeOwned>(
    client: &C, prompt: &str, task: LlmTaskType,
) -> anyhow::Result<T> {
    let full_prompt = format!("{}\n\nReturn ONLY valid JSON, no markdown code blocks.", prompt);
    let raw = client.generate(&full_prompt, task).await?;
    let json_str = raw.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    Ok(serde_json::from_str(json_str)?)
}

pub struct DeepSeekClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model_generate: String,
    model_evaluate: String,
    timeout_secs: u64,
}

impl DeepSeekClient {
    pub fn new(
        base_url: String,
        api_key: String,
        model_generate: String,
        model_evaluate: String,
        timeout_secs: u64,
    ) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model_generate,
            model_evaluate,
            timeout_secs,
        }
    }
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[async_trait]
impl LlmClient for DeepSeekClient {
    async fn generate(&self, prompt: &str, task: LlmTaskType) -> anyhow::Result<String> {
        let model = match task {
            LlmTaskType::Generate => &self.model_generate,
            LlmTaskType::Evaluate => &self.model_evaluate,
        };

        let system_prompt = match task {
            LlmTaskType::Generate => "You are a professional lexicographer. Output only the requested data in the exact format specified. Do not add explanations or markdown.",
            LlmTaskType::Evaluate => "You are a strict dictionary quality reviewer. Evaluate the given data and respond ONLY with a valid JSON object. No markdown, no preamble.",
        };

        let req = ChatRequest {
            model: model.clone(),
            messages: vec![
                ChatMessage { role: "system".into(), content: system_prompt.into() },
                ChatMessage { role: "user".into(), content: prompt.to_string() },
            ],
            temperature: 0.3,
            max_tokens: Some(4096),
        };

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            // Retryable errors
            if status.as_u16() == 429 || status.is_server_error() {
                return Err(anyhow::anyhow!("LLM API error ({status}): {body}"));
            }
            return Err(anyhow::anyhow!("LLM API error ({status}): {body}"));
        }

        let chat_resp: ChatResponse = resp.json().await?;
        let content = chat_resp.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(content)
    }

}
