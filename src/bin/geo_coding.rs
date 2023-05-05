use std::io::Write;

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use poi_cli::{
    cli::{parse_key_value, KeyVal},
    process_error_output, ExtraArgs, GeoCodingConfig, LoadConfig,
};

/// Diff two http requests and compare the difference of the responses
#[derive(Parser, Debug, Clone)]
#[clap(version = "0.1.0", author = "Misky <fengwei5@foxmail.com>")]
pub struct Args {
    #[clap(subcommand)]
    action: Action,
}

#[derive(Subcommand, Debug, Clone)]
#[non_exhaustive]
enum Action {
    /// Diff two API response based on given profile.
    Query(RunArgs),
}

#[derive(Parser, Debug, Clone)]
struct RunArgs {
    /// The profile name
    #[clap(short, long, value_parser)]
    profile: String,

    /// Overrides args. Could be used to override the query, headers and body of the request.
    /// for query params, use `-e key=value`
    /// for headers, use `-e %key=value`
    /// for body, use `-e @key=value`
    #[clap(short, long, value_parser = parse_key_value, number_of_values = 1)]
    extra_params: Vec<KeyVal>,

    /// Configuration to use
    #[clap(short, long, value_parser)]
    config: Option<String>,

    #[clap(short, long, value_parser)]
    output: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let result = match args.action {
        Action::Query(args) => query(args).await,
        // _ => panic!("Not implemented yet"),
    };

    process_error_output(result)
}

async fn query(args: RunArgs) -> Result<()> {
    let config_file = args.config.unwrap_or_else(|| "./poi.yaml".to_string());
    let config = GeoCodingConfig::load_yaml(&config_file).await?;

    let profile = config.get_profile(&args.profile).ok_or_else(|| {
        anyhow::anyhow!(
            "Profile {} not found in config file {}",
            args.profile,
            config_file
        )
    })?;

    let extra_args = ExtraArgs::from(args.extra_params);

    let result = profile.query(extra_args, &args.output).await?;

    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    write!(stdout, "{}", result)?;

    Ok(())
}
