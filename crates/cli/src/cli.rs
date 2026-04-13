use anyhow::Result;
use clap::{Parser, Subcommand};
use hermes_gateway::start_server;

#[derive(Parser)]
#[command(name = "hermes")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Chat {
        #[arg(long, default_value = "anthropic/claude-4")]
        model: String,
        #[arg(long)]
        session: Option<String>,
    },
    Gateway {
        #[command(subcommand)]
        action: GatewayAction,
    },
    Ui,
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    Model {
        model: Option<String>,
    },
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
    New,
    Reset,
    HelpCmd,
}

#[derive(Subcommand)]
enum GatewayAction {
    Start,
    Stop,
    Status,
}

#[derive(Subcommand)]
enum ConfigAction {
    Get { key: Option<String> },
    Set { key: String, value: String },
    List,
}

#[derive(Subcommand)]
enum ToolsAction {
    List,
    Enable { name: String },
    Disable { name: String },
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat { model, session }) => {
            run_chat(model, session).await?;
        }
        Some(Commands::Gateway { action }) => {
            run_gateway(action).await?;
        }
        Some(Commands::Ui) => {
            run_ui().await?;
        }
        Some(Commands::Config { action }) => {
            run_config(action).await?;
        }
        Some(Commands::Model { model }) => {
            if let Some(m) = model {
                println!("Switching to model: {}", m);
            } else {
                println!("Current model: anthropic/claude-4");
            }
        }
        Some(Commands::Tools { action }) => {
            run_tools(action).await?;
        }
        Some(Commands::New) | Some(Commands::Reset) => {
            println!("Starting new session...");
        }
        Some(Commands::HelpCmd) => {
            println!("Hermes AI Agent - Type 'hermes chat' to start");
        }
        None => {
            println!("Hermes AI Agent v0.1.0");
            println!("Usage: hermes <command>");
            println!("Commands:");
            println!("  chat     Start a new chat session");
            println!("  gateway  Manage the messaging gateway");
            println!("  ui       Launch the desktop GUI");
            println!("  config   View or modify configuration");
            println!("  model    Switch LLM provider/model");
            println!("  tools    Manage enabled tools");
        }
    }

    Ok(())
}

async fn run_chat(model: String, session: Option<String>) -> Result<()> {
    println!("Starting chat with model: {}", model);
    if let Some(s) = session {
        println!("Resuming session: {}", s);
    }
    println!("(Chat UI not yet implemented - this is a stub)");
    Ok(())
}

async fn run_gateway(action: GatewayAction) -> Result<()> {
    match action {
        GatewayAction::Start => {
            println!("Starting Hermes Gateway server on http://0.0.0.0:3848 ...");
            start_server(3848).await?;
        }
        GatewayAction::Stop => println!("Stopping gateway..."),
        GatewayAction::Status => println!("Gateway status: stopped"),
    }
    Ok(())
}

async fn run_ui() -> Result<()> {
    use std::process::Command;

    let ws_root = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let ui_exe = ws_root.join("crates/ui/src-tauri/target/release/hermes-ui-bin.exe");

    if !ui_exe.exists() {
        eprintln!("Error: hermes-ui-bin.exe not found at {}", ui_exe.display());
        eprintln!("Build it with: cd crates/ui/src-tauri && npm run tauri build");
        std::process::exit(1);
    }

    let backend_exe = ws_root.join("target/release/hermes.exe");

    println!("Starting Hermes backend server on http://0.0.0.0:3847 ...");
    let _backend = Command::new(&backend_exe)
        .args(["gateway", "start"])
        .spawn()
        .expect("Failed to start backend server");

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("Launching Hermes Agent UI...");
    Command::new(&ui_exe).spawn()?;
    Ok(())
}

async fn run_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key } => {
            if let Some(k) = key {
                println!("{} = <value>", k);
            }
        }
        ConfigAction::Set { key, value } => {
            println!("Set {} = {}", key, value);
        }
        ConfigAction::List => {
            println!("Available config keys:");
        }
    }
    Ok(())
}

async fn run_tools(action: ToolsAction) -> Result<()> {
    match action {
        ToolsAction::List => {
            println!("Enabled tools:");
        }
        ToolsAction::Enable { name } => {
            println!("Enabled: {}", name);
        }
        ToolsAction::Disable { name } => {
            println!("Disabled: {}", name);
        }
    }
    Ok(())
}
