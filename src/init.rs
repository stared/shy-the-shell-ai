use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use crate::config::{Config, AVAILABLE_MODELS};

pub fn run_init() -> Result<()> {
    println!("🎯 Welcome to Shy - AI Shell Assistant Setup");
    println!();

    // Get API key
    let api_key: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your OpenRouter API key")
        .interact_text()?;

    if api_key.trim().is_empty() {
        anyhow::bail!("API key cannot be empty");
    }

    // Select model
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose your default AI model")
        .default(0)
        .items(AVAILABLE_MODELS)
        .interact()?;

    let default_model = AVAILABLE_MODELS[selection].to_string();

    // Create and save config
    let config = Config {
        api_key: api_key.trim().to_string(),
        default_model,
    };

    config.save()?;

    println!();
    println!("✅ Configuration saved successfully!");
    println!("   Config location: {:?}", Config::config_path()?);
    println!("   Default model: {}", config.default_model);
    println!();
    println!("You can now run 'shy' to start the AI assistant.");

    Ok(())
}