use crate::api::OpenRouterClient;
use crate::config::{Config, AVAILABLE_MODELS};
use anyhow::Result;
use console::{style, Color};
use reedline::{
    ColumnarMenu, Completer, EditCommand, Emacs, KeyCode, KeyModifiers, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, ReedlineMenu, Signal, Suggestion,
};
use std::env;
use std::fs;
use std::path::PathBuf;

pub struct ShyRepl {
    line_editor: Reedline,
    prompt: ShyPrompt,
    client: OpenRouterClient,
    config: Config,
    last_suggested_commands: Vec<String>,
    history_offset: usize,
    selected_history_source: Option<usize>,
}

#[derive(Clone)]
struct ShyPrompt;

impl Prompt for ShyPrompt {
    fn render_prompt_left(&self) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_right(&self) -> std::borrow::Cow<str> {
        "".into()
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> std::borrow::Cow<str> {
        "〉".into()
    }

    fn render_prompt_multiline_indicator(&self) -> std::borrow::Cow<str> {
        "〉".into()
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> std::borrow::Cow<str> {
        "search: ".into()
    }
}

#[derive(Clone)]
struct ShyCompleter {
    commands: Vec<CommandInfo>,
}

#[derive(Clone)]
struct CommandInfo {
    name: String,
    description: String,
}

impl ShyCompleter {
    fn new() -> Self {
        let commands = vec![
            CommandInfo {
                name: "/help".to_string(),
                description: "Show available commands".to_string(),
            },
            CommandInfo {
                name: "/exit".to_string(),
                description: "Exit the assistant".to_string(),
            },
            CommandInfo {
                name: "/model".to_string(),
                description: "Change AI model".to_string(),
            },
            CommandInfo {
                name: "/config".to_string(),
                description: "Show current configuration".to_string(),
            },
            CommandInfo {
                name: "/env".to_string(),
                description: "Show environment information".to_string(),
            },
            CommandInfo {
                name: "/run".to_string(),
                description: "Execute a shell command".to_string(),
            },
            CommandInfo {
                name: "/history".to_string(),
                description: "Show recent bash history".to_string(),
            },
        ];

        Self { commands }
    }
}

impl Completer for ShyCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        if line.starts_with('/') {
            self.commands
                .iter()
                .filter(|cmd| cmd.name.starts_with(line.trim()))
                .map(|cmd| Suggestion {
                    value: cmd.name.clone(),
                    description: Some(cmd.description.clone()),
                    extra: None,
                    span: reedline::Span::new(0, pos),
                    append_whitespace: true,
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl ShyRepl {
    pub fn new(config: Config) -> Result<Self> {
        let mut line_editor = Reedline::create();

        // Set up completer with instant menu display
        let completer = ShyCompleter::new();
        let completion_menu = Box::new(
            ColumnarMenu::default()
                .with_name("completion_menu")
                .with_columns(1)
                .with_column_width(Some(80))
                .with_column_padding(2),
        );

        // Configure keybindings to show menu on typing
        let mut keybindings = reedline::default_emacs_keybindings();

        // Add keybinding to show completion menu after typing / characters
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Char('/'),
            ReedlineEvent::Multiple(vec![
                ReedlineEvent::Edit(vec![EditCommand::InsertChar('/')]),
                ReedlineEvent::Menu("completion_menu".to_string()),
            ]),
        );

        // Don't bind regular letters - only "/" should trigger menu

        // Tab autocompletes (fills in text)
        keybindings.add_binding(
            KeyModifiers::NONE,
            KeyCode::Tab,
            ReedlineEvent::UntilFound(vec![
                ReedlineEvent::Edit(vec![EditCommand::Complete]),
                ReedlineEvent::Menu("completion_menu".to_string()),
            ]),
        );

        // Let reedline handle Enter naturally:
        // - In completion menu: selects completion + submits
        // - My input handler will execute the completed command

        line_editor = line_editor
            .with_completer(Box::new(completer))
            .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
            .with_edit_mode(Box::new(Emacs::new(keybindings)))
            .with_quick_completions(true)
            .with_partial_completions(true);

        let prompt = ShyPrompt;
        let client = OpenRouterClient::new(config.api_key.clone(), config.default_model.clone());

        Ok(Self {
            line_editor,
            prompt,
            client,
            config,
            last_suggested_commands: Vec::new(),
            history_offset: 0,
            selected_history_source: None,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        println!(
            "{} {}",
            style("Shy - SHell AI Assistant").bold().fg(Color::Cyan),
            style(format!("(using {})", self.config.default_model)).fg(Color::Yellow)
        );
        println!(
            "{}",
            style("Don't be shy, just ask your shell. Type /help for commands").dim()
        );
        println!();

        loop {
            let sig = self.line_editor.read_line(&self.prompt)?;

            match sig {
                Signal::Success(buffer) => {
                    let input = buffer.trim();

                    if input.is_empty() {
                        continue;
                    }

                    // All commands starting with / should be executed immediately
                    // since they're either typed manually or selected from completion
                    if let Err(e) = self.handle_input(input).await {
                        eprintln!(
                            "{} Error: {}",
                            style("✗").fg(Color::Red),
                            style(e).fg(Color::Red)
                        );
                    }
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("{} Goodbye!", style("👋").fg(Color::Cyan));
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
                println!("{}", style("Available Commands").bold().fg(Color::Cyan));
                println!(
                    "  {}  {}",
                    style("/help").fg(Color::Green),
                    style("Show this help message").dim()
                );
                println!(
                    "  {}  {}",
                    style("/exit").fg(Color::Green),
                    style("Exit the assistant").dim()
                );
                println!(
                    "  {}  {}",
                    style("/model").fg(Color::Green),
                    style("Change AI model").dim()
                );
                println!(
                    "  {}  {}",
                    style("/config").fg(Color::Green),
                    style("Show current configuration").dim()
                );
                println!(
                    "  {}  {}",
                    style("/env").fg(Color::Green),
                    style("Show environment information").dim()
                );
                println!(
                    "  {}  {}",
                    style("/run").fg(Color::Green),
                    style("Execute a shell command or show suggested commands").dim()
                );
                println!(
                    "  {}  {}",
                    style("/history").fg(Color::Green),
                    style("Show recent shell history with navigation").dim()
                );
                println!();
                println!(
                    "{}",
                    style("Or just type your message to chat with the AI.").dim()
                );
                println!();
            }
            "/exit" => {
                println!("{} Goodbye!", style("👋").fg(Color::Cyan));
                std::process::exit(0);
            }
            "/model" => {
                self.change_model().await?;
            }
            "/config" => {
                println!();
                println!("{}", style("Current Configuration").bold().fg(Color::Cyan));
                println!(
                    "  {}: {}",
                    style("Model").fg(Color::Green),
                    style(&self.config.default_model).fg(Color::White)
                );
                println!(
                    "  {}: {}",
                    style("Config file").fg(Color::Green),
                    style(format!("{:?}", Config::config_path()?)).dim()
                );
                println!();
            }
            "/env" => {
                self.show_environment();
            }
            "/run" => {
                if parts.len() > 1 {
                    // Direct command execution
                    let command = parts[1..].join(" ");
                    self.execute_command(&command).await?;
                } else {
                    // Show interactive menu if we have suggested commands
                    if !self.last_suggested_commands.is_empty() {
                        println!();
                        println!(
                            "{}",
                            style("📋 Available Suggested Commands:")
                                .bold()
                                .fg(Color::Cyan)
                        );
                        self.display_interactive_commands();
                        // Note: menu will be shown after chat response, not here
                    } else {
                        println!("{}", style("Usage:").bold().fg(Color::Cyan));
                        println!(
                            "  {} {}",
                            style("/run").fg(Color::Green),
                            style("<command>").dim()
                        );
                        println!("{}", style("Example:").bold().fg(Color::Cyan));
                        println!(
                            "  {} {}",
                            style("/run").fg(Color::Green),
                            style("ls -la").dim()
                        );
                    }
                }
            }
            "/history" => {
                self.show_bash_history_interactive().await?;
            }
            _ => {
                println!(
                    "{} Unknown command: {}. Type {} for available commands.",
                    style("⚠").fg(Color::Yellow),
                    style(cmd).fg(Color::Red),
                    style("/help").fg(Color::Green)
                );
            }
        }

        Ok(())
    }

    fn show_environment(&self) {
        println!();
        println!(
            "{}",
            style("Environment Information").bold().fg(Color::Cyan)
        );

        // Current working directory
        if let Ok(pwd) = env::current_dir() {
            println!(
                "  {}: {}",
                style("Working Directory").fg(Color::Green),
                style(pwd.display()).fg(Color::White)
            );
        }

        // Shell type
        if let Ok(shell) = env::var("SHELL") {
            println!(
                "  {}: {}",
                style("Shell").fg(Color::Green),
                style(&shell).fg(Color::White)
            );
        }

        // List files (capped at 10)
        println!(
            "  {}:",
            style("Files in current directory").fg(Color::Green)
        );
        if let Ok(entries) = fs::read_dir(".") {
            let mut files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect();
            files.sort();

            let display_count = files.len().min(10);
            for file in files.iter().take(display_count) {
                println!("    {} {}", style("•").fg(Color::Cyan), style(file).dim());
            }

            if files.len() > 10 {
                println!(
                    "    {} {}",
                    style("•").fg(Color::Cyan),
                    style(format!("and {} more files", files.len() - 10)).dim()
                );
            }
        }

        // System info
        println!(
            "  {}: {}",
            style("OS").fg(Color::Green),
            style(env::consts::OS).fg(Color::White)
        );
        println!(
            "  {}: {}",
            style("Architecture").fg(Color::Green),
            style(env::consts::ARCH).fg(Color::White)
        );
        println!();
    }

    async fn execute_command(&self, command: &str) -> Result<()> {
        self.execute_command_with_confirmation(command, true).await
    }

    async fn execute_command_with_confirmation(
        &self,
        command: &str,
        ask_confirmation: bool,
    ) -> Result<()> {
        use dialoguer::{Confirm, Input};
        use std::process::Command;

        let mut current_command = command.to_string();

        if ask_confirmation {
            loop {
                println!();
                println!("{}", style("Command Execution").bold().fg(Color::Cyan));
                println!(
                    "{} {}",
                    style("•").fg(Color::Green),
                    style("Executing shell command as requested").dim()
                );
                println!();
                println!("{}", style("Command:").bold().fg(Color::Green));
                println!("  {}", self.format_command_with_syntax(&current_command));
                println!();

                let should_run = Confirm::new()
                    .with_prompt("Do you want to execute this command?")
                    .default(false)
                    .interact()?;

                if !should_run {
                    let modify = Confirm::new()
                        .with_prompt("Would you like to modify the command?")
                        .default(false)
                        .interact()?;

                    if modify {
                        current_command = Input::new()
                            .with_prompt("Enter modified command")
                            .with_initial_text(&current_command)
                            .interact_text()?;
                        continue;
                    }

                    println!("{}", style("Command cancelled.").fg(Color::Yellow));
                    return Ok(());
                }

                break;
            }
        }

        println!(
            "{} {}",
            style("▸").fg(Color::Green),
            style(&current_command).bold()
        );

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").args(["/C", &current_command]).output()
        } else {
            Command::new("sh").arg("-c").arg(&current_command).output()
        };

        match output {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    println!("{}", String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                }
                if !output.status.success() {
                    println!(
                        "{} Command exited with status: {}",
                        style("⚠").fg(Color::Yellow),
                        style(output.status).fg(Color::Red)
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "{} Failed to execute command: {}",
                    style("✗").fg(Color::Red),
                    style(e).fg(Color::Red)
                );
            }
        }

        Ok(())
    }

    async fn handle_chat(&mut self, message: &str) -> Result<()> {
        use std::time::Instant;

        // Start timing
        let start_time = Instant::now();

        // Create enriched context with environment info
        let context = self.create_context(message);
        let response = self
            .client
            .stream_chat_with_timing(&context, start_time, message)
            .await?;

        // Extract commands from response for quick execution
        self.extract_and_store_commands(&response);

        // Auto-trigger interactive menu if commands were suggested
        if !self.last_suggested_commands.is_empty() {
            if let Err(e) = self.prompt_command_selection().await {
                eprintln!(
                    "{} Error in command selection: {}",
                    style("✗").fg(Color::Red),
                    style(e).fg(Color::Red)
                );
            }
        }

        Ok(())
    }

    fn create_context(&self, message: &str) -> String {
        let mut context = String::new();

        // Add environment context
        context.push_str("Environment context:\n");

        if let Ok(pwd) = env::current_dir() {
            context.push_str(&format!("Current directory: {}\n", pwd.display()));
        }

        if let Ok(shell) = env::var("SHELL") {
            context.push_str(&format!("Shell: {}\n", shell));
        }

        // Add some files for context (limited)
        if let Ok(entries) = fs::read_dir(".") {
            let files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .take(5)
                .collect();

            if !files.is_empty() {
                context.push_str("Files in current directory: ");
                context.push_str(&files.join(", "));
                context.push('\n');
            }
        }

        // Add recent shell history for context
        if let Ok((recent_commands, _)) = self.get_recent_bash_history(10) {
            if !recent_commands.is_empty() {
                context.push_str("Recent shell history:\n");
                for (i, cmd) in recent_commands.iter().enumerate() {
                    context.push_str(&format!("  {}: {}\n", i + 1, cmd));
                }
            }
        }

        context.push_str(&format!("OS: {}\n", env::consts::OS));
        context.push('\n');
        context.push_str("Instructions: You are a professional shell assistant. Provide concise, helpful responses.\n");
        context.push_str("Response format:\n");
        context.push_str("- NUMBER your suggestions as 1., 2., 3. to match the execution menu\n");
        context.push_str("- Suggest 1-3 different solutions with varied approaches\n");
        context.push_str("- Vary your language - don't repeat the same starting phrases\n");
        context.push_str("- Be more descriptive about what each command accomplishes\n");
        context.push_str("- Examples: '1. Show basic listing', '2. Display detailed file info', '3. View hidden files and permissions'\n");
        context.push_str("- Put commands and flags in backticks: `ls`, `-la`, `git status`\n");
        context.push_str("- NO emojis - maintain professional CLI aesthetic\n");
        context.push_str("- Keep explanations brief but informative\n");
        context
            .push_str("- Consider the user's recent command history when suggesting solutions\n\n");
        context.push_str("User request: ");
        context.push_str(message);

        context
    }

    fn extract_and_store_commands(&mut self, response: &str) {
        use regex::Regex;

        let mut commands = Vec::new();

        // Extract from code blocks
        if let Ok(code_block_regex) = Regex::new(r"```(?:bash|sh|shell)?\n([^`]+)```") {
            for cap in code_block_regex.captures_iter(response) {
                if let Some(command) = cap.get(1) {
                    let cmd = command.as_str().trim();
                    if !cmd.is_empty() && Self::looks_like_command(cmd) {
                        commands.push(cmd.to_string());
                    }
                }
            }
        }

        // Extract from inline code
        if let Ok(inline_code_regex) = Regex::new(r"`([^`]+)`") {
            for cap in inline_code_regex.captures_iter(response) {
                if let Some(command) = cap.get(1) {
                    let cmd = command.as_str().trim();
                    if Self::looks_like_command(cmd) {
                        commands.push(cmd.to_string());
                    }
                }
            }
        }

        // Limit to 3 commands max
        commands.truncate(3);
        self.last_suggested_commands = commands;

        // Commands will be shown in the interactive menu
    }

    fn display_interactive_commands(&self) {
        println!();
        println!("{}", style("Suggested Commands").bold().fg(Color::Cyan));

        for (i, cmd) in self.last_suggested_commands.iter().enumerate() {
            let formatted_cmd = self.format_command_with_syntax(cmd);
            println!(
                "{}  {}",
                style(format!("[{}]", i + 1)).bold().fg(Color::Green),
                formatted_cmd
            );
        }
        println!();
    }

    fn format_command_with_syntax(&self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return cmd.to_string();
        }

        let mut result = String::new();

        // Command name in cyan
        result.push_str(&style(&parts[0]).fg(Color::Cyan).to_string());

        // Flags and arguments
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
    }

    async fn prompt_command_selection(&mut self) -> Result<()> {
        use dialoguer::{theme::ColorfulTheme, Select};

        if self.last_suggested_commands.is_empty() {
            return Ok(());
        }

        // Create menu options with "Do nothing" as first option
        let mut menu_options = vec!["Do nothing".to_string()];

        for (i, cmd) in self.last_suggested_commands.iter().enumerate() {
            let formatted_cmd = self.format_command_with_syntax(cmd);
            menu_options.push(format!("Execute {}: {}", i + 1, formatted_cmd));
        }

        menu_options.push("Enter custom command".to_string());

        println!(); // Add spacing before menu
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to do?")
            .default(0) // Default to "Do nothing" for safety
            .items(&menu_options)
            .interact()?;
        println!(); // Add spacing after selection

        match selection {
            0 => {
                // Do nothing - safe default (dialoguer already shows selection)
            }
            i if i <= self.last_suggested_commands.len() => {
                // Execute suggested command (i-1 because index 0 is "Do nothing")
                let command = &self.last_suggested_commands[i - 1];
                self.execute_command_with_confirmation(command, false)
                    .await?;
            }
            _ => {
                // Custom command
                use dialoguer::Input;
                let custom_command: String = Input::new()
                    .with_prompt("Enter your command")
                    .interact_text()?;

                if !custom_command.trim().is_empty() {
                    self.execute_command(&custom_command).await?;
                } else {
                    println!("{}", style("No command entered.").fg(Color::Green));
                }
            }
        }

        Ok(())
    }

    fn looks_like_command(text: &str) -> bool {
        let text = text.trim();

        // Skip if it's too long (probably not a single command)
        if text.len() > 200 {
            return false;
        }

        // Skip if it contains newlines (multi-line, probably not a single command)
        if text.contains('\n') {
            return false;
        }

        // Common command patterns
        let command_patterns = [
            r"^(ls|cd|pwd|mkdir|rmdir|rm|cp|mv|cat|less|more|head|tail|grep|find|which|whereis)",
            r"^(git|npm|yarn|cargo|pip|docker|kubectl|ssh|scp|rsync|curl|wget)",
            r"^(sudo|su|chmod|chown|ps|kill|top|htop|df|du|free|mount|umount)",
            r"^(systemctl|service|journalctl|crontab|at|nohup|screen|tmux)",
            r"^(vim|nano|emacs|code|subl)",
            r"^[a-zA-Z0-9_-]+\s+", // Generic command with arguments
        ];

        command_patterns
            .iter()
            .any(|pattern| regex::Regex::new(pattern).is_ok_and(|re| re.is_match(text)))
    }

    async fn change_model(&mut self) -> Result<()> {
        use dialoguer::{theme::ColorfulTheme, Select};

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
            self.client = OpenRouterClient::new(self.config.api_key.clone(), new_model.clone());
            self.config.default_model = new_model;

            println!(
                "{} Model changed successfully!",
                style("✓").fg(Color::Green)
            );
        } else {
            println!("{} Model unchanged.", style("•").fg(Color::Cyan));
        }
        println!();

        Ok(())
    }

    async fn show_bash_history_interactive(&mut self) -> Result<()> {
        let mut current_offset = self.history_offset;
        let page_size = 20;

        loop {
            // Get paginated history
            let (commands, source_info, total_count) =
                self.get_paginated_history(current_offset, page_size)?;

            if commands.is_empty() && current_offset == 0 {
                println!();
                println!("{}", style("No shell history found").fg(Color::Yellow));
                println!("{}", style("History may be empty or not accessible").dim());
                println!();
                return Ok(());
            }

            // Display history
            println!();
            println!("{}", style("Shell History").bold().fg(Color::Cyan));
            println!(
                "  {}: {}",
                style("Source").fg(Color::Green),
                style(&source_info).fg(Color::White)
            );
            println!(
                "  {}: {}",
                style("Total commands").fg(Color::Green),
                style(total_count).fg(Color::White)
            );

            let start_num = current_offset + 1;
            let end_num = (current_offset + commands.len()).min(total_count);
            println!(
                "  {}: {}-{}",
                style("Showing").fg(Color::Green),
                style(start_num).fg(Color::White),
                style(end_num).fg(Color::White)
            );
            println!();

            for (i, cmd) in commands.iter().enumerate() {
                let formatted_cmd = self.format_command_with_syntax(cmd);
                println!(
                    "{}  {}",
                    style(format!("{:2}.", start_num + i)).fg(Color::Green),
                    formatted_cmd
                );
            }
            println!();

            // Navigation menu
            use dialoguer::{theme::ColorfulTheme, Select};

            let mut menu_options = vec!["Exit history".to_string()];

            // Add navigation options
            if current_offset > 0 {
                menu_options.push("← Previous 20".to_string());
            }
            if current_offset + page_size < total_count {
                menu_options.push("Next 20 →".to_string());
            }

            menu_options.push("Change history source".to_string());

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Navigation")
                .default(0)
                .items(&menu_options)
                .interact()?;

            match menu_options[selection].as_str() {
                "Exit history" => break,
                "← Previous 20" => {
                    current_offset = current_offset.saturating_sub(page_size);
                }
                "Next 20 →" => {
                    current_offset += page_size;
                }
                "Change history source" => {
                    if self.select_history_source().await? {
                        // Reset offset when switching sources
                        current_offset = 0;
                    }
                }
                _ => {}
            }
        }

        // Update stored offset
        self.history_offset = current_offset;
        Ok(())
    }

    fn get_paginated_history(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<String>, String, usize)> {
        let history_paths = self.get_shell_history_paths();

        for (path, shell_type) in history_paths {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    let all_commands = if shell_type == "Fish" {
                        self.parse_fish_history(&contents)
                    } else {
                        self.parse_standard_history(&contents)
                    };

                    let total_count = all_commands.len();
                    let commands: Vec<String> = all_commands
                        .into_iter()
                        .rev() // Most recent first
                        .skip(offset)
                        .take(limit)
                        .collect();

                    let source_info = format!("{} ({})", shell_type, path.display());
                    return Ok((commands, source_info, total_count));
                }
            }
        }

        Ok((Vec::new(), "No history found".to_string(), 0))
    }

    async fn select_history_source(&mut self) -> Result<bool> {
        // Get all possible paths, not just the prioritized ones
        let mut all_paths = Vec::new();

        if let Ok(histfile) = env::var("HISTFILE") {
            all_paths.push((PathBuf::from(histfile), "Custom"));
        }

        if let Ok(home) = env::var("HOME") {
            let home_path = PathBuf::from(home);
            let all_files = [
                (".local/share/fish/fish_history", "Fish"),
                (".zsh_history", "Zsh"),
                (".bash_history", "Bash"),
                (".history", "Shell"),
                (".sh_history", "Shell"),
            ];

            for (file, shell_type) in &all_files {
                let path = home_path.join(file);
                if !all_paths.iter().any(|(p, _)| p == &path) {
                    all_paths.push((path, *shell_type));
                }
            }
        }

        let mut available_sources = Vec::new();
        let mut available_indices = Vec::new();

        for (i, (path, shell_type)) in all_paths.iter().enumerate() {
            if path.exists() {
                let last_modified = if let Ok(metadata) = fs::metadata(path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                            let timestamp = duration.as_secs();
                            self.format_file_timestamp(timestamp as i64)
                        } else {
                            "unknown".to_string()
                        }
                    } else {
                        "unknown".to_string()
                    }
                } else {
                    "unknown".to_string()
                };

                available_sources.push(format!(
                    "{} ({}) - last modified: {}",
                    shell_type,
                    path.display(),
                    last_modified
                ));
                available_indices.push(i);
            }
        }

        if available_sources.is_empty() {
            println!();
            println!("{}", style("No history sources found").fg(Color::Yellow));
            println!();
            return Ok(false);
        }

        if available_sources.len() == 1 {
            println!();
            println!(
                "{}",
                style("Only one history source available").fg(Color::Cyan)
            );
            println!("  {}", style(&available_sources[0]).fg(Color::White));
            println!();
            return Ok(false);
        }

        use dialoguer::{theme::ColorfulTheme, Select};

        // Add option to use auto-detection
        let mut menu_options = vec!["Auto-detect (default behavior)".to_string()];
        menu_options.extend(available_sources);

        println!();
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select history source")
            .default(0)
            .items(&menu_options)
            .interact()?;

        if selection == 0 {
            // Reset to auto-detection
            self.selected_history_source = None;
            println!();
            println!("{}", style("Reset to auto-detection").fg(Color::Green));
            println!();
        } else {
            // Set specific source
            let source_index = available_indices[selection - 1];
            self.selected_history_source = Some(source_index);
            println!();
            println!(
                "{} {}",
                style("Selected source:").fg(Color::Green),
                style(&menu_options[selection]).fg(Color::White)
            );
            println!();
        }

        Ok(true) // Source was changed
    }

    fn format_file_timestamp(&self, timestamp: i64) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        if let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let now_secs = now.as_secs() as i64;
            let diff = now_secs - timestamp;

            if diff < 3600 {
                // Less than an hour ago
                let minutes = diff / 60;
                format!("{}m ago", minutes)
            } else if diff < 86400 {
                // Less than a day ago
                let hours = diff / 3600;
                format!("{}h ago", hours)
            } else {
                // Days ago
                let days = diff / 86400;
                if days == 1 {
                    "1 day ago".to_string()
                } else {
                    format!("{} days ago", days)
                }
            }
        } else {
            "unknown".to_string()
        }
    }

    fn get_recent_bash_history(&self, limit: usize) -> Result<(Vec<String>, String)> {
        let history_paths = self.get_shell_history_paths();

        for (path, shell_type) in history_paths {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    let commands = if shell_type == "Fish" {
                        self.parse_fish_history(&contents)
                    } else {
                        self.parse_standard_history(&contents)
                    };

                    let recent_commands: Vec<String> = commands
                        .into_iter()
                        .rev() // Get most recent first
                        .take(limit)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev() // Reverse back to chronological order
                        .collect();

                    let source_info = format!("{} ({})", shell_type, path.display());
                    return Ok((recent_commands, source_info));
                }
            }
        }

        Ok((Vec::new(), "No history found".to_string()))
    }

    fn get_all_bash_history(&self) -> Result<Vec<String>> {
        let history_paths = self.get_shell_history_paths();

        for (path, shell_type) in history_paths {
            if path.exists() {
                if let Ok(contents) = fs::read_to_string(&path) {
                    let commands = if shell_type == "Fish" {
                        self.parse_fish_history(&contents)
                    } else {
                        self.parse_standard_history(&contents)
                    };

                    return Ok(commands);
                }
            }
        }

        Ok(Vec::new())
    }

    fn parse_standard_history(&self, contents: &str) -> Vec<String> {
        contents
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .map(|line| line.trim().to_string())
            .filter(|cmd| !cmd.is_empty() && cmd.len() < 200)
            .collect()
    }

    fn parse_fish_history(&self, contents: &str) -> Vec<String> {
        let mut commands = Vec::new();
        let mut current_command = String::new();
        let mut in_command = false;

        for line in contents.lines() {
            if line.starts_with("- cmd: ") {
                // Save previous command if exists
                if in_command && !current_command.trim().is_empty() {
                    commands.push(current_command.trim().to_string());
                }
                // Start new command
                current_command = line.strip_prefix("- cmd: ").unwrap_or("").to_string();
                in_command = true;
            } else if line.starts_with("  when: ") || line.starts_with("  paths:") {
                // End of command definition
                if in_command && !current_command.trim().is_empty() {
                    commands.push(current_command.trim().to_string());
                    current_command.clear();
                }
                in_command = false;
            } else if in_command && line.starts_with("  ") {
                // Multi-line command continuation
                current_command.push('\n');
                current_command.push_str(line.trim());
            }
        }

        // Don't forget the last command
        if in_command && !current_command.trim().is_empty() {
            commands.push(current_command.trim().to_string());
        }

        // Filter out very long commands and clean up
        commands
            .into_iter()
            .filter(|cmd| !cmd.is_empty() && cmd.len() < 200)
            .collect()
    }

    fn get_shell_history_paths(&self) -> Vec<(PathBuf, &'static str)> {
        let mut all_paths = Vec::new();

        // Build list of all available paths
        if let Ok(histfile) = env::var("HISTFILE") {
            all_paths.push((PathBuf::from(histfile), "Custom"));
        }

        if let Ok(home) = env::var("HOME") {
            let home_path = PathBuf::from(home);

            let all_files = [
                (".local/share/fish/fish_history", "Fish"),
                (".zsh_history", "Zsh"),
                (".bash_history", "Bash"),
                (".history", "Shell"),
                (".sh_history", "Shell"),
            ];

            for (file, shell_type) in &all_files {
                let path = home_path.join(file);
                if !all_paths.iter().any(|(p, _)| p == &path) {
                    all_paths.push((path, *shell_type));
                }
            }
        }

        // If a specific source is selected, return only that one
        if let Some(selected_index) = self.selected_history_source {
            if selected_index < all_paths.len() {
                return vec![all_paths[selected_index].clone()];
            }
        }

        // Otherwise, return prioritized list
        let mut paths = Vec::new();

        // Detect current shell and prioritize its history
        let current_shell = self.detect_current_shell();

        // Add current shell's history first if no manual selection
        for (path, shell_type) in &all_paths {
            match current_shell.as_str() {
                "fish" if *shell_type == "Fish" => {
                    paths.push((path.clone(), *shell_type));
                    break;
                }
                "zsh" if *shell_type == "Zsh" => {
                    paths.push((path.clone(), *shell_type));
                    break;
                }
                "bash" if *shell_type == "Bash" => {
                    paths.push((path.clone(), *shell_type));
                    break;
                }
                _ => {}
            }
        }

        // Add remaining paths as fallbacks
        for (path, shell_type) in all_paths {
            if !paths.iter().any(|(p, _)| p == &path) {
                paths.push((path, shell_type));
            }
        }

        paths
    }

    fn detect_current_shell(&self) -> String {
        // Check if fish is running by looking at parent processes
        if let Ok(output) = std::process::Command::new("pgrep")
            .args(["-f", "fish"])
            .output()
        {
            if !output.stdout.is_empty() {
                return "fish".to_string();
            }
        }

        // Check if the parent process is fish
        if let Ok(ppid) = env::var("PPID") {
            if let Ok(output) = std::process::Command::new("ps")
                .args(["-p", &ppid, "-o", "comm="])
                .output()
            {
                let parent_process = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_lowercase();
                if parent_process.contains("fish") {
                    return "fish".to_string();
                } else if parent_process.contains("zsh") {
                    return "zsh".to_string();
                } else if parent_process.contains("bash") {
                    return "bash".to_string();
                }
            }
        }

        // Check what shell is currently running
        if let Ok(output) = std::process::Command::new("ps")
            .args(["-p", &std::process::id().to_string(), "-o", "comm="])
            .output()
        {
            let shell_process = String::from_utf8_lossy(&output.stdout)
                .trim()
                .to_lowercase();
            if shell_process.contains("fish") {
                return "fish".to_string();
            } else if shell_process.contains("zsh") {
                return "zsh".to_string();
            } else if shell_process.contains("bash") {
                return "bash".to_string();
            }
        }

        // Fallback: check SHELL environment variable
        if let Ok(shell) = env::var("SHELL") {
            if shell.contains("fish") {
                return "fish".to_string();
            } else if shell.contains("zsh") {
                return "zsh".to_string();
            } else if shell.contains("bash") {
                return "bash".to_string();
            }
        }

        "unknown".to_string()
    }
}
