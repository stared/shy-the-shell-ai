# Shy - SHell AI Assistant

A Rust CLI tool that provides an AI-powered shell assistant with interactive setup and streaming responses. Don't be shy, just ask your shell.

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

## Development
After making changes, always compile to test:
```bash
cargo build --release
```

## Dependencies
- `clap` - CLI argument parsing and completions
- `dialoguer` - Interactive prompts
- `serde` + `toml` - Configuration serialization
- `reedline` - REPL with history and editing
- `reqwest` - HTTP client for API calls
- `tokio` - Async runtime
- `console` - Terminal styling and colors

## UX/UI Design Principles

### Visual Design
- **Minimal yet beautiful**: Clean, professional CLI aesthetic without unnecessary visual clutter
- **Consistent coloring**: Standardized color scheme across all interfaces (cyan for commands, yellow for flags, white for arguments, green for success, red for errors)
- **Progressive feedback**: Live, animated progress indicators with timing to show system responsiveness
- **Proper spacing**: Strategic use of whitespace and line breaks to improve readability and reduce cognitive load

### Interaction Design  
- **No duplicate messages**: Each piece of information shown only once to avoid confusion
- **Clear visual hierarchy**: Important information (commands, timing, status) stands out through styling
- **Immediate feedback**: Real-time updates during operations (animated spinners, live timing)
- **Consistent behavior**: Similar actions have similar visual and interaction patterns

### Technical Implementation
- **Graceful state management**: Clean transitions between states (thinking → completed → menu)
- **Proper line clearing**: Terminal output management to prevent visual artifacts
- **Semantic coloring**: Colors convey meaning (syntax highlighting for commands, status colors for outcomes)

## REPL Commands
- `/help` - Show available commands
- `/exit` - Exit the REPL
- `/model` - Change AI model
- `/config` - Show current configuration
- `/env` - Show environment information
- `/run` - Execute shell commands

## Configuration
Config stored at `~/.config/shy/config.toml`:
```toml
api_key = "your-openrouter-key"
default_model = "gemini-2.5-flash"
```

Available models: GPT-4.1, Claude-4, Gemini 2.5, o4-mini variants.