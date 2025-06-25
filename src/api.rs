use anyhow::Result;
use console::{style, Color};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};

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

    pub async fn stream_chat_with_timing(
        &self,
        message: &str,
        start_time: std::time::Instant,
        _user_input: &str,
    ) -> Result<String> {
        use std::io::{self, Write};
        use std::time::Duration;

        // Show animated thinking (user input already displayed by REPL)
        print!(" ");
        io::stdout().flush().unwrap();

        // Animate spinner
        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut spinner_index = 0;

        // Start the API call in a separate task
        let api_future = self.stream_chat_internal(message);
        let mut api_future = Box::pin(api_future);

        loop {
            // Update spinner with continuous time display
            let elapsed = start_time.elapsed().as_secs_f32();
            print!(
                " {} {}",
                style(spinner_chars[spinner_index]).fg(Color::Cyan),
                style(format!("({:.1}s)", elapsed)).fg(Color::Yellow)
            );
            io::stdout().flush().unwrap();

            // Check if API call is done
            match tokio::time::timeout(Duration::from_millis(80), &mut api_future).await {
                Ok(result) => {
                    // API call completed
                    let response = result?;

                    // Clear the entire spinner line completely and show clean final timing
                    let final_time = start_time.elapsed().as_secs_f32();
                    print!(
                        "\r{}\r {}\n",
                        " ".repeat(50), // Clear the entire line first
                        style(format!("({:.1}s)", final_time)).fg(Color::Yellow)
                    );

                    // Print response
                    println!();
                    self.print_with_syntax_highlighting(&response);
                    println!(); // Move to next line
                    
                    // Ensure output is flushed and terminal is ready for interactive elements
                    use std::io::{self, Write};
                    io::stdout().flush().unwrap();

                    return Ok(response);
                }
                Err(_) => {
                    // Timeout, continue spinning - clear the line for next update
                    print!("\r");
                    spinner_index = (spinner_index + 1) % spinner_chars.len();
                }
            }
        }
    }

    #[allow(dead_code)]
    pub async fn stream_chat(&self, message: &str) -> Result<String> {
        self.stream_chat_internal(message).await
    }

    async fn stream_chat_internal(&self, message: &str) -> Result<String> {
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

                    if let Some(content) = self.extract_content_from_json(data) {
                        if first_token {
                            first_token = false;
                        }
                        full_response.push_str(&content);
                    }
                }
            }
        }

        Ok(full_response)
    }

    fn extract_content_from_json(&self, data: &str) -> Option<String> {
        let json = serde_json::from_str::<Value>(data).ok()?;
        let choices = json["choices"].as_array()?;
        let choice = choices.first()?;
        let delta = choice["delta"].as_object()?;
        delta["content"].as_str().map(|s| s.to_string())
    }

    fn print_with_syntax_highlighting(&self, text: &str) {
        let mut result = String::new();
        let chars = text.chars().peekable();
        let mut in_backticks = false;
        let mut current_word = String::new();

        for ch in chars {
            if ch == '`' {
                if in_backticks {
                    // End of backticked content - apply syntax highlighting
                    result.push_str(&self.format_code_element(&current_word));
                    current_word.clear();
                    in_backticks = false;
                } else {
                    // Start of backticked content
                    if !current_word.is_empty() {
                        result.push_str(&current_word);
                        current_word.clear();
                    }
                    in_backticks = true;
                }
            } else if in_backticks {
                current_word.push(ch);
            } else if ch == ' ' || ch == '\n' || ch == '\t' {
                if !current_word.is_empty() {
                    result.push_str(&current_word);
                    current_word.clear();
                }
                result.push(ch);
            } else {
                current_word.push(ch);
            }
        }

        // Handle any remaining content
        if !current_word.is_empty() {
            if in_backticks {
                result.push_str(&self.format_code_element(&current_word));
            } else {
                result.push_str(&current_word);
            }
        }

        print!("{}", result);
    }

    fn format_code_element(&self, text: &str) -> String {
        let trimmed = text.trim();

        // Handle pipe commands specially
        if trimmed.contains('|') {
            return self.format_pipe_command(trimmed);
        }

        // Check if it's a multi-part command
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() > 1 {
            // Multi-part command - format each part without backticks
            let mut result = String::new();

            // First part (command) in cyan
            result.push_str(&style(&parts[0]).fg(Color::Cyan).to_string());

            for part in &parts[1..] {
                result.push(' ');
                if part.starts_with('-') {
                    // Flags in yellow
                    result.push_str(&style(part).fg(Color::Yellow).to_string());
                } else {
                    // Arguments in white
                    result.push_str(&style(part).fg(Color::White).to_string());
                }
            }
            result
        } else {
            // Single element without backticks
            if trimmed.starts_with('-') {
                // Command flags in yellow
                style(trimmed).fg(Color::Yellow).to_string()
            } else if self.looks_like_command(trimmed) {
                // Commands in cyan
                style(trimmed).fg(Color::Cyan).to_string()
            } else {
                // General code in white (consistent with arguments)
                style(trimmed).fg(Color::White).to_string()
            }
        }
    }

    fn format_pipe_command(&self, text: &str) -> String {
        let pipe_parts: Vec<&str> = text.split('|').collect();
        let mut result = String::new();

        for (i, pipe_part) in pipe_parts.iter().enumerate() {
            if i > 0 {
                result.push_str(&style(" | ").fg(Color::White).to_string());
            }

            let trimmed_part = pipe_part.trim();
            let parts: Vec<&str> = trimmed_part.split_whitespace().collect();

            if !parts.is_empty() {
                // First part (command) in cyan
                result.push_str(&style(&parts[0]).fg(Color::Cyan).to_string());

                for part in &parts[1..] {
                    result.push(' ');
                    if part.starts_with('-') {
                        // Flags in yellow
                        result.push_str(&style(part).fg(Color::Yellow).to_string());
                    } else {
                        // Arguments in white
                        result.push_str(&style(part).fg(Color::White).to_string());
                    }
                }
            }
        }

        result
    }

    fn looks_like_command(&self, text: &str) -> bool {
        let common_commands = [
            "ls",
            "cd",
            "pwd",
            "mkdir",
            "rmdir",
            "rm",
            "cp",
            "mv",
            "cat",
            "less",
            "more",
            "head",
            "tail",
            "grep",
            "find",
            "which",
            "whereis",
            "git",
            "npm",
            "yarn",
            "cargo",
            "pip",
            "docker",
            "kubectl",
            "ssh",
            "scp",
            "rsync",
            "curl",
            "wget",
            "sudo",
            "su",
            "chmod",
            "chown",
            "ps",
            "kill",
            "top",
            "htop",
            "df",
            "du",
            "free",
            "mount",
            "umount",
            "systemctl",
            "service",
            "vim",
            "nano",
            "emacs",
        ];

        // Check if it's a known command or contains command-like patterns
        common_commands.contains(&text)
            || text
                .chars()
                .all(|c| c.is_ascii_lowercase() || c == '-' || c == '_')
    }
}
