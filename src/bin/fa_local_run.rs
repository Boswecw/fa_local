//! FA Local - minimal diagnostic and intake CLI
//!
//! Usage:
//!
//! ```bash
//! # Validate an execution request from a file
//! fa-local-run validate --request request.json
//!
//! # Validate an execution request from stdin
//! cat request.json | fa-local-run validate
//!
//! # Run the admission gate end-to-end over a bundle of contract inputs
//! fa-local-run admit --inputs admit-inputs.json
//!
//! # Check FA Local contract posture and emit a structured status report
//! fa-local-run status
//! ```
//!
//! Exit codes:
//! - 0 - operation succeeded
//! - 1 - validation failed or operational error

use std::io::{self, Read};
use std::process;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;

use fa_local::domain::execution::ExecutionRequest;
use fa_local::{AdmissionContext, AdmissionInputs, AdmissionService, EnvironmentMode, now_utc};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `admit` input bundle: the four contract envelopes plus optional evaluation
/// context. `environment` defaults to prod; `now` defaults to the current time.
#[derive(Debug, Deserialize)]
struct AdmitInput {
    requester_trust: Value,
    policy: Value,
    capability_registry: Value,
    execution_request: Value,
    #[serde(default)]
    environment: Option<EnvironmentMode>,
    #[serde(default)]
    now: Option<DateTime<Utc>>,
}

fn read_input_bytes(flag: &str, args: &[String]) -> Vec<u8> {
    let path = args
        .windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].as_str());
    match path {
        Some(path) => match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("error: could not read input file {path:?}: {e}");
                process::exit(1);
            }
        },
        None => {
            let mut buf = Vec::new();
            if let Err(e) = io::stdin().read_to_end(&mut buf) {
                eprintln!("error: could not read from stdin: {e}");
                process::exit(1);
            }
            buf
        }
    }
}

fn run_admit(args: &[String]) -> ! {
    let bytes = read_input_bytes("--inputs", args);
    let parsed: AdmitInput = match serde_json::from_slice(&bytes) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("{{\n  \"status\": \"invalid_input\",\n  \"error\": \"{e}\"\n}}");
            process::exit(1);
        }
    };

    let context = AdmissionContext::new(
        parsed.environment.unwrap_or(EnvironmentMode::Prod),
        parsed.now.unwrap_or_else(now_utc),
    );
    let inputs = AdmissionInputs {
        requester_trust: parsed.requester_trust,
        policy: parsed.policy,
        capability_registry: parsed.capability_registry,
        execution_request: parsed.execution_request,
    };

    match AdmissionService.admit(&inputs, context) {
        Ok(outcome) => {
            let body = serde_json::json!({
                "status": "decided",
                "execution_allowed": outcome.route_decision.execution_allowed,
                "resolved_approval_posture": outcome.route_decision.resolved_approval_posture,
                "review_required": outcome.route_decision.review_required,
                "explicit_approval_required": outcome.route_decision.explicit_approval_required,
                "route_decision": outcome.route_decision,
                "review_package": outcome.review_package.as_ref().map(|p| &p.package),
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&body).unwrap_or_else(|e| format!(
                    "{{\"status\":\"serialize_error\",\"error\":\"{e}\"}}"
                ))
            );
            process::exit(0);
        }
        Err(e) => {
            println!("{{\n  \"status\": \"error\",\n  \"error\": \"{e}\"\n}}");
            process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("validate") => {
            let request_path = args
                .windows(2)
                .find(|w| w[0] == "--request")
                .map(|w| w[1].as_str());

            let bytes = match request_path {
                Some(path) => match std::fs::read(path) {
                    Ok(b) => b,
                    Err(e) => {
                        eprintln!("error: could not read request file {path:?}: {e}");
                        process::exit(1);
                    }
                },
                None => {
                    let mut buf = Vec::new();
                    if let Err(e) = io::stdin().read_to_end(&mut buf) {
                        eprintln!("error: could not read from stdin: {e}");
                        process::exit(1);
                    }
                    buf
                }
            };

            let value = match serde_json::from_slice(&bytes) {
                Ok(value) => value,
                Err(e) => {
                    eprintln!("{{");
                    eprintln!("  \"status\": \"invalid\",");
                    eprintln!("  \"error\": \"{e}\"");
                    eprintln!("}}");
                    process::exit(1);
                }
            };

            match ExecutionRequest::load_contract_value(&value) {
                Ok(request) => {
                    println!("{{");
                    println!("  \"status\": \"valid\",");
                    println!("  \"request_id\": \"{}\",", request.request_id);
                    println!("  \"correlation_id\": \"{}\",", request.correlation_id);
                    println!("  \"environment_mode\": \"{:?}\"", request.environment_mode);
                    println!("}}");
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("{{");
                    eprintln!("  \"status\": \"invalid\",");
                    eprintln!("  \"error\": \"{e}\"");
                    eprintln!("}}");
                    process::exit(1);
                }
            }
        }

        Some("admit") => run_admit(&args),

        Some("status") => {
            println!("{{");
            println!("  \"service\": \"fa-local-operator\",");
            println!("  \"version\": \"{VERSION}\",");
            println!("  \"posture\": \"policy_first_admission\",");
            println!("  \"execution_enabled\": false,");
            println!("  \"writeback_wired\": false,");
            println!(
                "  \"note\": \"bounded local execution consumer - execution bridge v1 pending Phase X4 wiring\""
            );
            println!("}}");
            process::exit(0);
        }

        Some("--version") | Some("-V") => {
            println!("fa-local-run {VERSION}");
            process::exit(0);
        }

        Some("--help") | Some("-h") | None => {
            eprintln!("FA Local - bounded local execution control service");
            eprintln!("");
            eprintln!("USAGE:");
            eprintln!("  fa-local-run <COMMAND>");
            eprintln!("");
            eprintln!("COMMANDS:");
            eprintln!(
                "  validate    Validate a bounded execution request against FA Local contract schema"
            );
            eprintln!(
                "  admit       Run the admission gate end-to-end over a bundle of contract inputs"
            );
            eprintln!("  status      Emit a structured FA Local posture and readiness report");
            eprintln!("");
            eprintln!("OPTIONS FOR validate:");
            eprintln!("  --request <FILE>   Read request JSON from file (default: stdin)");
            eprintln!("");
            eprintln!("OPTIONS FOR admit:");
            eprintln!(
                "  --inputs <FILE>    Read the admission input bundle from file (default: stdin)"
            );
            eprintln!("");
            eprintln!("EXIT CODES:");
            eprintln!("  0   Success");
            eprintln!("  1   Validation failure or error");
            let code = if args.get(1).map(String::as_str) == Some("--help")
                || args.get(1).map(String::as_str) == Some("-h")
            {
                0
            } else {
                1
            };
            process::exit(code);
        }

        Some(unknown) => {
            eprintln!("error: unknown command {unknown:?}");
            eprintln!("Run 'fa-local-run --help' for usage.");
            process::exit(1);
        }
    }
}
