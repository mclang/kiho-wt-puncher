//
// Rust command-line app for using Kiho v3 worktime (punch) API.
//

// https://docs.rs/confy/latest/confy/index.html
// https://github.com/rust-cli/confy
extern crate confy;

// https://docs.rs/reqwest/latest/reqwest/
// https://github.com/seanmonstar/reqwest
extern crate reqwest;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

// For getting user input:
use std::io;
use std::io::Write;

use chrono::prelude::*;


const APP_NAME:     &str = "Kiho Worktime Puncher";
const CONFIG_NAME:  &str = "kiho-worktime-puncher";
const APP_VERSION:  &str = env!("CARGO_PKG_VERSION");
const STAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

// Documentation: http://developers.kiho.fi/api
// Examples:
//  https://v3.kiho.fi/api/v1/punch?mode=latest
//  https://v3.kiho.fi/api/v1/punch?orderBy=timestamp+DESC&pageSize=10&type=LOGIN
// TODO: Maybe `const_format` could be used to generate USER_AGENT with VERSION information?
const KIHO_API_URL: &str = "https://v3.kiho.fi/api/v1/punch";
const USER_AGENT:   &str = "Kiho Worktime Puncher/reqwest";

// https://docs.rs/crate/clap/latest
// https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(about, version)]
/// Command line Rust application for keeping track of your Kiho worktime.
struct CliArgs {
    /// Main command to execute
    #[command(subcommand)]
    command: CliCommands,
    /// Skip doing anything concrete, e.g HTTP GET/POST requests,
    /// which MIGHT have some side effects. (default: false)
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
    /// Print additional information during program execution.
    /// Use `-vv` to get even more detailed output.
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum CliCommands {
    /// Get things like current configuration or latest worktime lines.
    Get {
        #[command(subcommand)]
        what: CliGetWhat,
    },
    /// Add worktime break (NOT IMPLEMENTED)
    Break,
    // TODO: Throw an error if something has been started already
    /// Start working on something work related
    Start(PunchDesc),
    // TODO: Throw an error if nothing has been started
    /// Stop whatever worktime task was active
    Stop,
}

#[derive(Subcommand)]
enum CliGetWhat {
    /// Get current loaded configuration
    Config,
    /// Get 'customer cost centers' that are available in configuration
    CCC,
    /// Get list of configured 'recurring tasks'
    Tasks,
    /// Print example login/logout JSONs
    JSON,
    /// Get latest COUNT worktime BREAK/LOGIN/LOGOUT punch lines
    Latest {
        /// Number of punch lines to get
        #[arg(value_name = "count")]
        cnt: u32,
        /// Punch type to get. (default: all types)
        #[arg(value_enum, value_name="type")]
        typ: Option<PunchType>,
    },
}

#[derive(Args, Clone)]
struct PunchDesc {
    #[arg(value_name = "description")]
    desc: Option<String>,
}
impl std::fmt::Display for PunchDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self.desc {
            None       => panic!("ERROR: Punch description cannot be empty!"),
            Some(desc) => write!(f, "{}", desc),
        }
    }
}

#[derive(ValueEnum, Clone, Copy)]
enum PunchType {
    BREAK,
    LOGIN,
    LOGOUT,
}
impl std::fmt::Display for PunchType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PunchType::BREAK => write!(f, "BREAK"),
            PunchType::LOGIN => write!(f, "LOGIN"),
            PunchType::LOGOUT => write!(f, "LOGOUT"),
        }
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct KihoWtConfig {
    title:   String,
    api_key: String,
    updated: String,
    // NOTE
    // - Putting `recurring_tasks` after `cost_centres` result in `SerializeTomlError(ValueAfterTable)` error :/
    // - HashMap KEY has to be also `String` b/c TOML keys are always interpreted as strings.
    recurring_tasks: Vec<String>,
    cost_centres: std::collections::HashMap<String,String>,
}
impl Default for KihoWtConfig {
    fn default() -> Self {
        KihoWtConfig {
            title:   format!("Configuration file for '{}'", APP_NAME),
            api_key: "Ask API Key from administrator".to_string(),
            updated: Local::now().format("%d.%m.%Y").to_string(),
            cost_centres: std::collections::HashMap::from([
                (String::from("000000"), String::from("Dummy example cost centre")),
            ]),
            recurring_tasks: vec![
                String::from("Dummy example recurring task description"),
            ],
        }
    }
}

fn load_config() -> KihoWtConfig {
    let cfg_name = CONFIG_NAME;
    let cfg_path = confy::get_configuration_file_path(cfg_name, None)
        .expect("Getting confy configuration file path failed");
    let cfg: KihoWtConfig = confy::load(cfg_name, None).unwrap_or_else(|err| {
        println!("ERROR: {:?}", err);
        panic!("Loading configuration from '{}' failed!", cfg_path.display());
    });
    cfg
}

fn print_config(cfg: KihoWtConfig) {
    println!("{:#?}", cfg); // Using `:#?` gives pretty-formatted JSON style output
}


fn ask_recurring_desc(tasks: Vec<String>) -> PunchDesc {
    let tasks_cnt = tasks.len();
    println!("{} :: No punch description given.\nPlease select one from the available recurring ones:", Local::now().format(STAMP_FORMAT));
    for idx in 0..tasks_cnt {
        println!("{:>4}: {}", (idx+1), tasks[idx]);
    }

    let mut user_choice = String::new();
    let description: &String = loop {
        user_choice.clear();
        print!("Which task you want to start [1-{tasks_cnt}, or (c)ancel]? ");
        io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut user_choice)
            .expect("Error reading user's choice");
        user_choice = user_choice.trim().to_lowercase();
        if user_choice == "c" {
            println!("EXITING...");
            std::process::exit(0);
        }
        match user_choice.parse::<usize>() {
            Ok(idx) if idx > 0 && idx <= tasks_cnt => break &tasks[idx-1],
            _                                      => continue,
        };
    };
    PunchDesc { desc: Some(String::from(description)) }
}


fn create_punch_json(punch_type: PunchType, punch_desc: Option<PunchDesc>, _cc_id: Option<u32>) -> serde_json::Value {
    let timestamp: String = Local::now().format("%Y-%m-%dT%H:%M:%S%Z").to_string();
    // TODO: Implement 'Customer Cost Centre' -block (struct?) if it is given
    let json = match punch_type {
        PunchType::BREAK => panic!("Starting a BREAK not supported!"),
        PunchType::LOGIN => {
            json!({
                "newPunch": {
                    "type": punch_type.to_string(),
                    "description": punch_desc.expect("START PUNCH HAS TO HAVE DESCRIPTION").to_string(),
                    "customerCostcentre": null,
                    "timestamp": timestamp,
                    "realTimestamp": timestamp
                }
            })
        },
        PunchType::LOGOUT => {
            json!({
                "newPunch": {
                    "type": punch_type.to_string(),
                    "timestamp": timestamp,
                    "realTimestamp": timestamp
                }
            })
        },
    };
    // TODO: Verbosity check
    // println!("PUNCH JSON: {:#}", json); // Using `:#` gives pretty-formated JSON output
    json
}

fn print_example_jsons() {
    let json_login = json!({
        "newPunch": {
            "type": "LOGIN",
            "description": "Rusting it out",
            "customerCostcentre": { "id": 101124 },
            "timestamp": "2023-08-22T14:09:09+03:00",
            "realTimestamp": "2023-08-22T14:09:09+03:00"
        }
    });
    let json_logout = json!({
        "newPunch": {
            "type": "LOGOUT",
            "customerCostcentre": null,
            "timestamp": "2023-08-22T14:08:55+03:00",
            "realTimestamp": "2023-08-22T14:08:55+03:00"
        }
    });
    println!("JSON BODY FOR LOGIN (NOTE: With 'CustomerCostcentre!):\n{}\n",  serde_json::to_string_pretty(&json_login).unwrap());
    println!("JSON BODY FOR LOGOUT:\n{}\n", serde_json::to_string_pretty(&json_logout).unwrap());
    println!("EXAMPLE JSON LOGIN/LOGOUT RESPONSES");
    let json_punch_login_resp  = json!({"result":{"address":null,"checkEventId":null,"customerCostcentre":{"code":9006,"costcenter":{"code":"21","deleted":false,"description":"Kiho AI Business Platform","id":30654,"name":"Palvelinympäristön kehitys","vismaCode":""},"customer":{"code":7001,"id":4410,"identity":"1862344-1","name":"Kiho Oy","nameExtra":"","nickname":""},"deleted":true,"description":null,"favourited":0,"id":101124,"name":"Palvelinympäristön kehitys","project":{"active":false,"code":"272","deleted":false,"description":"272/31/2019 / Tekes","id":109,"name":"Kiho AI Business Platform"},"workOrderNumber":null,"worksite":null},"description":"Rusting it out","device_sn":"","id":13586650,"labels":[],"location":null,"locationValidationEvent":null,"realTimestamp":"2023-08-24T08:02:12+03:00","source":"UNKNOWN","timestamp":"2023-08-24T08:02:12+03:00","type":"LOGIN","user":{"id":27874,"name":"Lång Jani","personNumber":"","teams":[{"id":4442,"isDefaultTeam":true,"name":"Team Sysadmin"}]},"wagecode":{"code":"0001","id":1268,"name":"Kuukausipalkka","type":"WORK"},"worklabel":null}});
    let json_punch_logout_resp = json!({"result": {"address": null,"checkEventId": null,"customerCostcentre": null,"description": "","device_sn": "","id": 13587416, "labels": [], "location": null, "locationValidationEvent": null, "realTimestamp": "2023-08-24T09:44:40+03:00", "source": "UNKNOWN", "timestamp": "2023-08-24T09:44:40+03:00", "type": "LOGOUT", "user": {"id": 27874,"name": "Lång Jani","personNumber": "", "teams": [{"id": 4442, "isDefaultTeam": true, "name": "Team Sysadmin" }]}, "wagecode": null, "worklabel": null}});
    for json_resp in &[json_punch_login_resp, json_punch_logout_resp] {
        let punch_desc = &json_resp["result"]["description"].as_str().unwrap();
        let punch_id   = &json_resp["result"]["id"];
        let punch_time = &json_resp["result"]["timestamp"].as_str().unwrap();
        let punch_type = &json_resp["result"]["type"].as_str().unwrap();
        println!("{punch_time} {punch_type} '{punch_desc}' (id: {punch_id})");
        let ccc = &json_resp["result"]["customerCostcentre"];
        println!("'CustomerCostcentre'\n{:#}", ccc);
    }
}


fn get_latest_punch(api_key: String, punch_type: Option<PunchType>, punch_count: u32) {
    println!("Starting HTTP GET request...");
    let mut params = vec![
        // ("mode",  String::from("latest")),           // Returns SINGLE `result` object instead of an ARRAY :/
        ("orderBy",  String::from("timestamp DESC")),   // NOTE: Nowadays `+` means SPACE in URLs like `%20` used to be !
        ("pageSize", punch_count.to_string()),
    ];
    let punch_list_header = match punch_type {
        None     => {
            format!("Latest {} worktime punch line(s)", punch_count)
        },
        Some(pt) => {
            params.push(("type", pt.to_string()));
            format!("Latest {} worktime {} punch line(s)", punch_count, pt)
        },
    };
    // TODO: Global cacheable client with default headers
    // TODO: Use `ClientBuilder` and add `.gzip(true)`
    let client = reqwest::blocking::Client::new()
        .get(KIHO_API_URL)
        .query(&params)
        .header(reqwest::header::AUTHORIZATION, api_key)
        // .header(reqwest::header::CONTENT_TYPE, "application/json") HTTP GET does NOT work if this is set!
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, USER_AGENT);
        // .version(reqwest::Version::HTTP_2);
    // TODO: Verbosity check for ALL
    println!("URL: {}", KIHO_API_URL);
    // println!("Query parameters:");
    // for (k,v) in params {
    //     println!("{k:>10}={v}")
    // }
    // println!("{:#?}", client);
    // TODO: Check for `dry-run`
    let resp = client
        .send()
        .expect("FAILED TO MAKE HTTP GET");
    // TODO: `match resp.status()`...
    println!("HTTP response: {}", resp.status());
    // TODO: Verbosity check for BOTH
    // println!("{:#?}", resp.headers());
    // println!("{:#?}", resp);
    let json: serde_json::Value = resp
        .json()
        .expect("FAILED TO PARSE JSON RESPONSE");
    // TODO: Verbosity check
    // println!("{:#}", json);
    let punch_lines = json["result"].as_array()
        .expect("FAILED TO PARSE `result` FROM THE RETURNED JSON");
    println!("\n{}:", punch_list_header);
    if punch_lines.len() == 0 {
        println!("NONE FOUND!");
    }
    // TODO: Sort array in ascending order by timestamp so that the newest punch is the bottom most
    for pl in punch_lines {
        let punch_id   = &pl["id"];
        let punch_desc = &pl["description"].as_str().unwrap_or_else(|| "desc: N/A");
        let punch_time = &pl["timestamp"].as_str().unwrap_or_else(||   "time: N/A");
        let punch_type = &pl["type"].as_str().unwrap_or_else(||        "type: N/A");
        println!("{punch_time} {punch_type:<6} [id: {punch_id}] {punch_desc}");
    }
}


fn http_punch_post(api_key: String, json_body: serde_json::Value) {
    println!("Starting HTTP POST request...");
    // TODO: Global cacheable client with default headers
    let client = reqwest::blocking::Client::new()
        .post(KIHO_API_URL)
        .json(&json_body)
        .header(reqwest::header::AUTHORIZATION, api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::USER_AGENT, USER_AGENT);
        // .version(reqwest::Version::HTTP_2);
    // TODO: Verbosity check for BOTH
    println!("URL: {}", KIHO_API_URL);
    // println!("{:#?}", client);
    // TODO: Check for `dry-run`
    let resp = client
        .send()
        .expect("FAILED TO MAKE HTTP POST");
    // TODO: `match resp.status()`...
    println!("HTTP response: {}", resp.status());
    // TODO: Verbosity check for BOTH
    // println!("{:#?}", resp.headers());
    // println!("{:#?}", resp);
    let json: serde_json::Value = resp
        .json()
        .expect("FAILED TO PARSE JSON RESPONSE");
    // TODO: Verbosity check
    // println!("{:#}", json);
    let punch_id   = &json["result"]["id"];
    let punch_desc = &json["result"]["description"].as_str().unwrap_or_else(|| "desc: N/A");
    let punch_time = &json["result"]["timestamp"].as_str().unwrap_or_else(||   "time: N/A");
    let punch_type = &json["result"]["type"].as_str().unwrap_or_else(||        "type: N/A");
    // TODO: Looks too much like normal log line
    println!("\nNew punch line created:");
    println!("{punch_time} {punch_type:<6} [id: {punch_id}] {punch_desc}");
}


fn main() {
    let time_start = Local::now();
    let header     = format!("    {} v{}    ", APP_NAME, APP_VERSION);
    println!("+{:-<1$}+", "", header.len());
    println!("|{}|", header);
    println!("+{:-<1$}+", "", header.len());
    let args = CliArgs::parse();
    if args.verbose > 0 {
        println!("API URL:     {}", KIHO_API_URL);
        println!("Config path: {}", confy::get_configuration_file_path(CONFIG_NAME, None)
                 .expect("Getting configuration file path failed").display());
        println!("Dry-run:     {}", args.dry_run);
        println!("Verbosity:   {}", args.verbose);
        println!("Start time:  {}", time_start.format(STAMP_FORMAT));
        println!();
        println!("{} :: Loading default configuration", Local::now().format(STAMP_FORMAT));
    }
    if args.dry_run && args.verbose == 0 {
        println!("NOTE: This is a DRY-RUN!");
    }
    let config = load_config();
    match &args.command {
        CliCommands::Get { what } => match what {
            CliGetWhat::CCC     => println!("Available 'Customer Cost Centres': {:#?}", config.cost_centres),
            CliGetWhat::Tasks   => println!("Available 'Recurring Tasks': {:#?}", config.recurring_tasks),
            CliGetWhat::Config  => print_config(config),
            CliGetWhat::JSON    => print_example_jsons(),
            CliGetWhat::Latest { cnt, typ } => get_latest_punch(config.api_key, *typ, *cnt),
        },
        CliCommands::Break => {
            println!("{} :: Starting a BREAK", Local::now().format(STAMP_FORMAT));
            let _json = create_punch_json(PunchType::BREAK, None, None);
        },
        CliCommands::Start(desc) => {
            let punch_desc = match &desc.desc {
                None    => ask_recurring_desc(config.recurring_tasks),
                Some(_) => desc.clone(),
            };
            println!("{} :: Starting '{}'", Local::now().format(STAMP_FORMAT), punch_desc);
            // TODO: Get latest worktime punch line and ERROR OUT if it is 'LOGIN'
            // TODO: List and ask cost centre
            let json = create_punch_json(PunchType::LOGIN, Some(punch_desc), None);
            if !args.dry_run { http_punch_post(config.api_key, json) }
        },
        CliCommands::Stop => {
            // TODO: Get latest worktime description and error out if it is NOT of type 'LOGIN'
            println!("{} :: Stopping worktime", Local::now().format(STAMP_FORMAT));
            let json = create_punch_json(PunchType::LOGOUT, None, None);
            if !args.dry_run { http_punch_post(config.api_key, json) }
        },
    }

    if args.verbose > 0 {
        let time_stop = Local::now();
        println!("");
        println!("Stop time: {}", time_stop.format(STAMP_FORMAT));
        println!("Elapsed:   {}", time_stop-time_start);
    }
    println!("");
}

