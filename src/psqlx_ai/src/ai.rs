use std::env;
use std::error::Error;

use serde::Deserialize;
use ureq::serde_json::{json, Value};

#[derive(Deserialize)]
struct OpenAiCompletionChoice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiCompletionResponse {
    choices: Vec<OpenAiCompletionChoice>,
}

#[derive(Deserialize)]
struct AnthropicCompletionContent {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicCompletionResponse {
    content: Vec<AnthropicCompletionContent>,
}

#[derive(Debug, PartialEq)]
enum AIProvider {
    OpenAI,
    Anthropic,
}

impl AIProvider {
    /// Detect the AI provider from the environment
    fn detect() -> Result<Self, Box<dyn Error>> {
        match env::var("PSQLX_AI_PROVIDER").as_deref() {
            Ok("openai") | Ok("OpenAI") => Ok(AIProvider::OpenAI),
            Ok("anthropic") | Ok("Anthropic") => Ok(AIProvider::Anthropic),
            Ok(provider) => Err(format!("Unknown AI provider: {}", provider).into()),

            // On err, fallback to OpenAI
            Err(_) => Ok(AIProvider::OpenAI),
        }
    }

    fn api_key() -> Result<String, Box<dyn Error>> {
        match AIProvider::detect()? {
            AIProvider::OpenAI => {
                env::var("OPENAI_API_KEY")
                    .map_err(|_| "OPENAI_API_KEY environment variable not set. Set the environment variable (OPENAI_API_KEY=...) before usage to enable AI meta-commands".into())
            },
            AIProvider::Anthropic => {
                env::var("ANTHROPIC_API_KEY")
                    .map_err(|_| "ANTHROPIC_API_KEY environment variable not set. Set the environment variable (ANTHROPIC_API_KEY=...) before usage to enable AI meta-commands".into())
            }
        }
    }

    fn model() -> Result<String, Box<dyn Error>> {
        match env::var("PSQLX_AI_MODEL") {
            Ok(model) => Ok(model),
            Err(_) => match AIProvider::detect()? {
                AIProvider::OpenAI => Ok("gpt-4o-mini".to_string()),
                AIProvider::Anthropic => Ok("claude-3-5-haiku-latest".to_string()),
            },
        }
    }

    fn url() -> Result<String, Box<dyn Error>> {
        match AIProvider::detect()? {
            AIProvider::OpenAI => Ok("https://api.openai.com/v1/chat/completions".to_string()),
            AIProvider::Anthropic => Ok("https://api.anthropic.com/v1/messages".to_string()),
        }
    }

    fn headers() -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let api_key = AIProvider::api_key()?;
        match AIProvider::detect()? {
            AIProvider::OpenAI => Ok(vec![
                ("Authorization".to_string(), format!("Bearer {}", api_key)),
                ("Content-Type".to_string(), "application/json".to_string()),
            ]),
            AIProvider::Anthropic => Ok(vec![
                ("x-api-key".to_string(), api_key),
                ("Content-Type".to_string(), "application/json".to_string()),
                ("anthropic-version".to_string(), "2023-06-01".to_string()),
            ]),
        }
    }

    fn max_tokens() -> Result<i32, Box<dyn Error>> {
        let max_tokens_str = env::var("PSQLX_AI_MAX_TOKENS").unwrap_or_else(|_| "4096".to_string());
        max_tokens_str.parse::<i32>().map_err(|e| e.into())
    }
}

/// Sends a chat completion request to the OpenAI API and retrieves the response.
///
/// # Arguments
/// - `payload`: A `serde_json::Value` representing the request payload, typically containing
///   the model, messages, temperature, and other parameters.
///
/// # Returns
/// - `Ok(String)`: The content of the first choice in the response.
/// - `Err(Box<dyn Error>)`: An error if the request fails, the response cannot be parsed,
///   or no valid content is returned.
///
/// # Errors
/// - Returns an error if the `OPENAI_API_KEY` or `ANTHROPIC_API_KEY` environment variable is not set.
/// - Returns an error if the request fails or the response cannot be deserialized.
/// - Returns an error if the response does not contain a valid choice or message content.
///
/// # HTTP Client Choice
/// We use `ureq` over `reqwest` due to the simplicity of the building/integration process.
/// Integrating `reqwest` required additional configuration that I preferred to avoid.
///
pub fn completion(
    messages: &mut Vec<Value>,
    system_message: &str,
) -> Result<String, Box<dyn Error>> {
    let provider = AIProvider::detect()?;
    let model = AIProvider::model()?;
    let url = AIProvider::url()?;
    let mut payload = json!({
        "temperature": 0.0,
        "model": model,
    });

    match provider {
        AIProvider::OpenAI => {
            let mut new_messages = vec![json!({
                "role": "system",
                "content": system_message
            })];
            messages
                .into_iter()
                .for_each(|msg| new_messages.push(msg.clone()));
            payload["messages"] = json!(new_messages);
            payload["max_completion_tokens"] = json!(AIProvider::max_tokens()?);
        }
        AIProvider::Anthropic => {
            payload["system"] = json!(system_message);
            payload["messages"] = json!(messages);
            payload["max_tokens"] = json!(AIProvider::max_tokens()?);
        }
    }

    let mut request = ureq::post(&url);
    for (header_name, header_value) in AIProvider::headers()? {
        request = request.set(&header_name, &header_value);
    }
    let response = request.send_json(payload)?;
    let content = match provider {
        AIProvider::OpenAI => {
            let response_data: OpenAiCompletionResponse = response.into_json()?;
            match &response_data.choices.first() {
                Some(choice) => match &choice.message.content {
                    Some(content) => content.clone(),
                    None => return Err("No content in response".into()),
                },
                None => return Err("No choices in response".into()),
            }
        }
        AIProvider::Anthropic => {
            let response_data: AnthropicCompletionResponse = response.into_json()?;
            match &response_data.content.first() {
                Some(content) => content.text.clone(),
                None => return Err("No content in response".into()),
            }
        }
    };

    Ok(content)
}
