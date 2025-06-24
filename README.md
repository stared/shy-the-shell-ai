# Shy - AI Shell Assistant

AI-powered shell assistant with streaming responses.

## Install & Test

```bash
cargo build --release
./target/release/shy init    # Setup API key + model
./target/release/shy         # Start chatting
```

## Commands

- `shy init` - Setup
- `shy` - Chat 
- `shy completions <shell>` - Generate completions

## REPL Commands

- `/help` - Show help
- `/model` - Change model
- `/exit` - Quit

Requires OpenRouter API key.