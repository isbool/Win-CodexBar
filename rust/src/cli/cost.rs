//! Cost command implementation

use clap::Args;

use super::usage::{OutputFormat, ProviderSelection};

/// Arguments for the cost command
#[derive(Args, Debug, Default)]
pub struct CostArgs {
    /// Provider to query (codex, claude, cursor, gemini, copilot, all, both)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Output format: text or json
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Shorthand for --format json
    #[arg(long)]
    pub json: bool,

    /// Disable ANSI colors in text output
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Pretty-print JSON output
    #[arg(long)]
    pub pretty: bool,

    /// Force refresh (bypass cache)
    #[arg(long)]
    pub refresh: bool,
}

/// Run the cost command
pub async fn run(args: CostArgs) -> anyhow::Result<()> {
    let format = if args.json {
        OutputFormat::Json
    } else {
        args.format
    };

    let providers = ProviderSelection::from_arg(args.provider.as_deref());
    let use_color = !args.no_color && is_terminal();

    tracing::debug!(
        "Running cost command: providers={:?}, format={:?}",
        providers.as_list(),
        format
    );

    // TODO: Actually scan logs and compute costs
    // For now, output a placeholder message

    match format {
        OutputFormat::Text => {
            let providers_list = providers.as_list();
            for provider in &providers_list {
                if use_color {
                    println!("\x1b[1m{} Cost\x1b[0m", provider.display_name());
                } else {
                    println!("{} Cost", provider.display_name());
                }
                println!("  Today:     $0.00 (placeholder)");
                println!("  This week: $0.00 (placeholder)");
                println!("  Total:     $0.00 (placeholder)");
                if providers_list.len() > 1 {
                    println!();
                }
            }
        }
        OutputFormat::Json => {
            let payloads: Vec<serde_json::Value> = providers
                .as_list()
                .iter()
                .map(|p| {
                    serde_json::json!({
                        "provider": p.cli_name(),
                        "cost": {
                            "today": 0.0,
                            "this_week": 0.0,
                            "total": 0.0,
                            "currency": "USD"
                        },
                        "error": "Cost scanning not yet implemented"
                    })
                })
                .collect();

            let output = if args.pretty {
                serde_json::to_string_pretty(&payloads)?
            } else {
                serde_json::to_string(&payloads)?
            };
            println!("{}", output);
        }
    }

    Ok(())
}

/// Check if stdout is a terminal
fn is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}
