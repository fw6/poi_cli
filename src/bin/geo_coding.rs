use std::{
    io::stdout,
    sync::{Arc, Mutex},
};

use anyhow::{Ok, Result};
use clap::{Parser, Subcommand};
use poi_cli::{
    cli::{parse_key_value, KeyVal},
    process_error_output, ExtraArgs, GeoCodingConfig, LoadConfig,
};
use serde_json::{json, Map};
use serde_json_lodash::get;
use tokio::task::JoinSet;

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
    /// Query the given address
    Query(QueryArgs),

    /// Query all
    QueryAll(QueryAllArgs),
}

#[derive(Parser, Debug, Clone)]
struct QueryArgs {
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

#[derive(Parser, Debug, Clone)]
struct QueryAllArgs {
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
    input: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let result = match args.action {
        Action::Query(args) => query(args).await,
        Action::QueryAll(args) => query_all(args).await,
        // _ => panic!("Not implemented yet"),
    };

    process_error_output(result)
}

async fn query(args: QueryArgs) -> Result<()> {
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

    let result = profile.query(extra_args).await?;

    let query = &profile.req.params.clone().unwrap_or_else(|| json!({}));

    let address = get!(json!(query), json!("address"));
    let address = address.as_str().unwrap_or("北京市人民政府");

    let mut wtr = csv::Writer::from_path(&args.output.clone().unwrap_or("result.csv".into()))?;

    let obj = result.as_object().unwrap();
    let first_record = String::from("地址");
    let mut records = obj.keys().collect::<Vec<_>>();
    records.insert(0, &first_record);
    wtr.write_record(records)?;

    write_records_to_csv(&mut wtr, &obj, address)?;

    wtr.flush()?;
    println!("Done");
    Ok(())
}

async fn query_all(args: QueryAllArgs) -> Result<()> {
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

    let mut rdr =
        csv::Reader::from_path(args.input.unwrap_or("fixtures/resident_city.csv".into()))?;
    let records = rdr.records();

    let mut set = JoinSet::new();

    let wtr = csv::Writer::from_writer(stdout());
    let wtr = Arc::new(Mutex::new(wtr));

    let has_head = Arc::new(Mutex::new(false));

    for record in records {
        let record = record?;
        let address = record.get(0).unwrap().to_owned();
        let extra_args = extra_args.clone();
        let profile = profile.clone();

        let wtr = wtr.clone();
        let has_head = has_head.clone();

        set.spawn(async move {
            let result = profile
                .query_with_city(extra_args, &address)
                .await
                .expect(format!("failed to query {}", address).as_str());

            let mut has_head = has_head.lock().unwrap();
            let obj = result.as_object().unwrap();
            let mut wtr = wtr.lock().unwrap();

            if has_head.eq(&false) {
                *has_head = true;

                let first_record = String::from("地址");
                let mut records = obj.keys().collect::<Vec<_>>();
                records.insert(0, &first_record);
                wtr.write_record(records).unwrap();
            }

            write_records_to_csv(&mut wtr, &obj, &address).unwrap();
        });
    }

    while let Some(_) = set.join_next().await {}
    let mut wtr = wtr.lock().unwrap();
    wtr.flush()?;

    Ok(())
}

fn write_records_to_csv<T: std::io::Write>(
    wtr: &mut csv::Writer<T>,
    obj: &Map<String, serde_json::Value>,
    address: &str,
) -> Result<()> {
    let mut record = csv::StringRecord::new();

    record.push_field(address);

    for v in obj.values() {
        if let Some(v) = v.as_str() {
            record.push_field(v);
        } else if let Some(_) = v.as_null() {
            record.push_field("");
        } else {
            record.push_field(v.to_string().as_str());
        }
    }

    wtr.write_record(record.as_byte_record())?;

    Ok(())
}
