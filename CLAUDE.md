# Shy - AI Shell Assistant

A Rust CLI tool that provides an AI-powered shell assistant with interactive setup and streaming responses.

## Project Structure
- `src/main.rs` - Entry point with CLI setup
- `src/config.rs` - Configuration management (TOML)
- `src/repl.rs` - REPL implementation with Reedline
- `src/api.rs` - OpenRouter API integration
- `src/init.rs` - Interactive initialization flow
- `Cargo.toml` - Dependencies and project metadata

## Commands
- `cargo run` - Start the REPL
- `cargo run -- init` - Interactive setup (API key + model selection)
- `cargo run -- completions <shell>` - Generate shell completions
- `cargo build --release` - Build optimized binary
- `cargo test` - Run tests
- `cargo clippy` - Lint code
- `cargo fmt` - Format code

## Dependencies
- `clap` - CLI argument parsing and completions
- `dialoguer` - Interactive prompts
- `serde` + `toml` - Configuration serialization
- `reedline` - REPL with history and editing
- `reqwest` - HTTP client for API calls
- `tokio` - Async runtime
- `spinners` - Loading animations

## REPL Commands
- `/help` - Show available commands
- `/exit` - Exit the REPL
- `/model` - Change AI model
- `/config` - Show current configuration

## Configuration
Config stored at `~/.config/shy/config.toml`:
```toml
api_key = "your-openrouter-key"
default_model = "gpt-4.1-nano"
```

## Available Models
- gpt-4.1-nano
- gpt-4.1-mini  
- o4-mini
- gemini-2.5-flash
- gemini-2.5-pro
- claude-4