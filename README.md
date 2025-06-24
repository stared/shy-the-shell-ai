# Shy - SHell AI Assistant

Don't be shy, just ask your shell. Beautiful, animated AI responses with intelligent command suggestions.

## Quick Start

```bash
cargo build --release
./target/release/shy init    # Setup API key + model
./target/release/shy         # Start chatting
```

## Example Usage

### Basic Interaction
```
〉List all files with 'config'
 ⠹ (0.8s)

1. Find files containing 'config': find . -name "*config*"
2. List files with detailed info: ls -la | grep config  
3. Search file contents: grep -r "config" .

✔ What would you like to do? › Execute 2: ls -la | grep config

▸ ls -la | grep config
-rw-r--r--  1 user  staff  245 Jun 24 config.toml
```

### Features Showcase
- **Animated Progress**: Live spinner with timing `⠋ (0.2s)` → `⠙ (0.4s)` → `(1.8s)`
- **Syntax Highlighting**: Commands in cyan, flags in yellow, args in white
- **Interactive Menus**: Choose from AI suggestions or enter custom commands
- **Clean Output**: No duplicate messages, proper spacing, minimal design

### Command Examples
```bash
shy init                    # Interactive setup
shy                        # Start AI shell
shy completions zsh        # Generate shell completions
```

### REPL Commands
- `/help` - Show available commands
- `/model` - Change AI model  
- `/config` - Show configuration
- `/env` - Show environment info
- `/run <cmd>` - Execute shell command
- `/exit` - Quit

## Requirements
- Rust 1.70+
- OpenRouter API key ([get one here](https://openrouter.ai/))

Works with modern AI models such as GPT-4.1, Claude 4 Sonnet, Gemini 2.5 Pro and Flash, and o4-mini.