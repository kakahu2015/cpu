use anyhow::{Context, Result};
use reqwest::Client;
use reqwest::tls::Version;
use reqwest::ClientBuilder;
use rustyline::Editor;
use rustyline::config::Config as RustylineConfig;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::process::Command;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
mod completer;
use completer::LinuxCommandCompleter;
use rustyline::error::ReadlineError;
use std::time::Instant;
use std::env;
use std::path::PathBuf;
//
//mod plugin_system;
//use plugin_system::{Plugin, PluginManager, PluginCall, WeatherPlugin};

const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";

#[derive(Debug, Deserialize)]
struct Config {
    openai: OpenAIConfig,
    system_prompt: String,
    max_recent_interactions: usize,
    max_openai_context: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIConfig {
    api_key: String,
    api_base: String,
    model: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

struct LinuxCommandAssistant {
    config: Config,
    client: Client,
    context: Vec<Message>,
    recent_interactions: VecDeque<String>,
    command_history: Vec<String>,
    is_command_mode: bool,
    //plugin_manager: PluginManager,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

impl LinuxCommandAssistant {
    fn new(config: Config) -> Result<Self> {
        //
        //let mut plugin_manager = PluginManager::new();
        //plugin_manager.register_plugin(Box::new(WeatherPlugin));
        ////////////////////////////////////////////
         let client = Client::builder()
            .use_rustls_tls()
            .build()
            .context("Failed to build HTTP client")?;
        
        let context = vec![Message {
            role: "system".to_string(),
            content: config.system_prompt.clone(),
        }];
        let max_recent_interactions = config.max_recent_interactions;
        Ok(Self {
            config,
            client,
            context,
            recent_interactions: VecDeque::with_capacity(max_recent_interactions),
            command_history: Vec::new(),
            is_command_mode: false,
            //plugin_manager,
        })
    }

async fn get_ai_response(&mut self, prompt: &str) -> Result<String> {
    let mut messages = self.context.clone();
    if messages.is_empty() {
        messages.push(Message {
            role: "system".to_string(),
            content: self.config.system_prompt.clone(),
        });
    }
    
    // 添加最近的Linux命令历史
    if !self.recent_interactions.is_empty() {
        let recent_history = self.recent_interactions.iter()
            .filter(|interaction| interaction.starts_with("Executed command:"))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        if !recent_history.is_empty() {
            messages.push(Message {
                role: "user".to_string(),
                content: format!("Recent Linux commands:\n{}\nPlease consider this context for the following question.", recent_history),
            });
        }
    }
    
    messages.push(Message {
        role: "user".to_string(),
        content: prompt.to_string(),
    });

    let request = serde_json::json!({
        "model": self.config.openai.model,
        "messages": messages,
    });

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", self.config.openai.api_key))?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = self.client.post(&self.config.openai.api_base)
        .headers(headers)
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let response: ChatCompletionResponse = response.json().await?;
        if let Some(choice) = response.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!("No response content from AI"))
        }
    } else {
        Err(anyhow::anyhow!("API request failed with status {}", response.status()))
    }
}
/////////////////////////////////////////////
fn execute_command(&mut self, command: &str) -> Result<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .context("Failed to execute command")?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    
    let result = if output.status.success() {
        if stdout.is_empty() {
            stderr
        } else {
            if command.trim().starts_with("ls") && command.contains("-l") {
                self.colorize_ls_output(&stdout)
            } else {
                stdout
            }
        }
    } else {
        if stderr.is_empty() {
            stdout
        } else {
            stderr
        }
    };

    self.add_to_recent_interactions(format!("Executed command: {}\nOutput: {}", command, result));

    Ok(result)
}
////////////////////////////
    fn colorize_ls_output(&self, output: &str) -> String {
        output.lines().map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 9 {
                let permissions = parts[0];
                let filename = parts[8..].join(" ");
                let colored_filename = if permissions.starts_with('d') {
                    format!("{}{}{}", BLUE, filename, RESET)
                } else if permissions.contains('x') {
                    format!("{}{}{}", GREEN, filename, RESET)
                } else {
                    format!("{}{}{}", RED, filename, RESET)
                };
                let mut colored_line = parts[..8].join(" ");
                colored_line.push_str(" ");
                colored_line.push_str(&colored_filename);
                colored_line
            } else {
                line.to_string()
            }
        }).collect::<Vec<String>>().join("\n")
    }

    fn update_context(&mut self, user_input: &str, response: &str) {
        self.context.push(Message {
            role: "user".to_string(),
            content: user_input.to_string(),
        });
        self.context.push(Message {
            role: "assistant".to_string(),
            content: response.to_string(),
        });
        if self.context.len() > self.config.max_openai_context {
            self.context = self.context.split_off(self.context.len() - self.config.max_openai_context);
        }
    }

    fn add_to_recent_interactions(&mut self, interaction: String) {
        self.recent_interactions.push_back(interaction);
        if self.recent_interactions.len() > self.config.max_recent_interactions {
            self.recent_interactions.pop_front();
        }
    }

    fn add_to_history(&mut self, command: String) {
        self.command_history.push(command);
    }
///////////////////////////////////////////////////////////
async fn run(&mut self) -> Result<()> {
    let config = RustylineConfig::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(LinuxCommandCompleter));

    loop {
        let prompt = if self.is_command_mode { 
            format!("{}$ {}", BLUE, RESET) 
        } else { 
            format!("{}kaka-ai> {}", YELLOW, RESET) 
        };
        let readline = rl.readline(&prompt);

        match readline {
            Ok(line) => {
                let line = line.trim();
                if line.eq_ignore_ascii_case("exit") {
                    break;
                }

                if line.eq_ignore_ascii_case("reset") {
                    self.context.clear();
                    self.recent_interactions.clear();
                    println!("Context and recent interactions have been reset.");
                    continue;
                }

                if !line.is_empty() && !line.starts_with('#') {
                    self.add_to_history(line.to_string());
                    rl.add_history_entry(line);
                }

                if line == "!" {
                    self.is_command_mode = !self.is_command_mode;
                    if self.is_command_mode {
                        println!("Entered Linux command mode. Type '!' to exit.");
                    } else {
                        println!("Exited Linux command mode.");
                    }
                    continue;
                }

                if self.is_command_mode {
                    match self.execute_command(line) {
                        Ok(output) => {
                            println!("{}", output);
                            self.update_context(&format!("Executed command: {}\nOutput: {}", line, output), "");
                        }
                        Err(e) => println!("Error executing command: {}", e),
                    }
                } else {
                    match self.get_ai_response(line).await {
                        Ok(response) => {
                            println!("kaka-AI: {}", response);
                            self.update_context(line, &response);
                            self.add_to_recent_interactions(format!("User: {}\nAI: {}", line, response));
                        }
                        Err(e) => println!("Error getting AI response: {}", e),
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
    /////////////////////////////
}

fn load_config() -> Result<Config> {
    let exe_path = env::current_exe().context("Failed to get executable path")?;
    let exe_dir = exe_path.parent().context("Failed to get executable directory")?;
    let config_path = exe_dir.join("config.yml");

    let config_str = std::fs::read_to_string(&config_path)
        .context("Failed to read config.yml")?;
    let config: Config = serde_yaml::from_str(&config_str)
        .context("Failed to parse config.yml")?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config()?;
    let mut assistant = LinuxCommandAssistant::new(config)?;
    assistant.run().await
}
