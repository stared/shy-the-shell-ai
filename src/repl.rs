use anyhow::Result;
use reedline::{DefaultPrompt, Reedline, Signal};
use spinners::{Spinner, Spinners};
use crate::api::OpenRouterClient;
use crate::config::{Config, AVAILABLE_MODELS};

pub struct ShyRepl {
    line_editor: Reedline,
    prompt: DefaultPrompt,
    client: OpenRouterClient,
    config: Config,
}

impl ShyRepl {
    pub fn new(config: Config) -> Result<Self> {
        let line_editor = Reedline::create();
        let prompt = DefaultPrompt::default();
        let client = OpenRouterClient::new(config.api_key.clone(), config.default_model.clone());

        Ok(Self {
            line_editor,
            prompt,
            client,
            config,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        println!("🤖 Shy AI Shell Assistant");
        println!("Type /help for commands, /exit to quit");
        println!();

        let rt = tokio::runtime::Runtime::new()?;

        loop {
            let sig = self.line_editor.read_line(&self.prompt)?;
            
            match sig {
                Signal::Success(buffer) => {
                    let input = buffer.trim();
                    
                    if input.is_empty() {
                        continue;
                    }

                    if let Err(e) = rt.block_on(self.handle_input(input)) {
                        eprintln!("Error: {}", e);
                    }
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("Goodbye! 👋");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_input(&mut self, input: &str) -> Result<()> {
        if input.starts_with('/') {
            self.handle_command(input).await
        } else {
            self.handle_chat(input).await
        }
    }

    async fn handle_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        let cmd = parts[0];

        match cmd {
            "/help" => {
                println!();
                println!("Available commands:");
                println!("  /help     - Show this help message");
                println!("  /exit     - Exit the assistant");
                println!("  /model    - Change AI model");
                println!("  /config   - Show current configuration");
                println!();
                println!("Or just type your message to chat with the AI.");
                println!();
            }
            "/exit" => {
                println!("Goodbye! 👋");
                std::process::exit(0);
            }
            "/model" => {
                self.change_model().await?;
            }
            "/config" => {
                println!();
                println!("Current configuration:");
                println!("  Model: {}", self.config.default_model);
                println!("  Config file: {:?}", Config::config_path()?);
                println!();
            }
            _ => {
                println!("Unknown command: {}. Type /help for available commands.", cmd);
            }
        }

        Ok(())
    }

    async fn handle_chat(&self, message: &str) -> Result<()> {
        let mut spinner = Spinner::new(Spinners::Dots, "Thinking...".into());
        
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            spinner.stop();
        });

        self.client.stream_chat(message).await?;

        Ok(())
    }

    async fn change_model(&mut self) -> Result<()> {
        use dialoguer::{Select, theme::ColorfulTheme};

        let current_index = AVAILABLE_MODELS
            .iter()
            .position(|&model| model == self.config.default_model)
            .unwrap_or(0);

        println!();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose new default model")
            .default(current_index)
            .items(AVAILABLE_MODELS)
            .interact()?;

        let new_model = AVAILABLE_MODELS[selection].to_string();
        
        if new_model != self.config.default_model {
            self.config.default_model = new_model.clone();
            self.config.save()?;
            
            // Update client with new model
            self.client = OpenRouterClient::new(self.config.api_key.clone(), new_model);
            
            println!("✅ Model changed successfully!");
        } else {
            println!("Model unchanged.");
        }
        println!();

        Ok(())
    }
}