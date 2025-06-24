use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, Write};

pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenRouterClient {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub async fn stream_chat(&self, message: &str) -> Result<String> {
        let payload = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": message
                }
            ],
            "stream": true
        });

        let response = self
            .client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("API request failed: {}", error_text);
        }

        let mut stream = response.bytes_stream();
        let mut first_token = true;
        let mut full_response = String::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let chunk_str = String::from_utf8_lossy(&chunk);
            
            for line in chunk_str.lines() {
                if line.starts_with("data: ") {
                    let data = line.strip_prefix("data: ").unwrap();
                    
                    if data == "[DONE]" {
                        break;
                    }
                    
                    if let Ok(json) = serde_json::from_str::<Value>(data) {
                        if let Some(choices) = json["choices"].as_array() {
                            if let Some(choice) = choices.first() {
                                if let Some(delta) = choice["delta"].as_object() {
                                    if let Some(content) = delta["content"].as_str() {
                                        if first_token {
                                            print!("\n");
                                            first_token = false;
                                        }
                                        print!("{}", content);
                                        full_response.push_str(content);
                                        io::stdout().flush()?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        println!("\n");
        
        Ok(full_response)
    }
}