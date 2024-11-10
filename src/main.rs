use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{
    env,
    io::{stdin, Read},
};

const SYSPROMPT: &str = "Consider the users input on the given data. Respond directly to the question, do not provide any other information. Respond in only on sentence if possible. Hint: the data might often be the output of running a linux command with the --help argument. In this case, do not mention the name of the program";
const FREE_MODEL: &str = "google/gemini-flash-1.5-8b";
const FREE_MODEL_CUTOFF: usize = 8000;
const PAIED_MODEL: &str = "google/gemini-flash-1.5-8b";
const API_KEY: &str = env!("GPTPIPE_KEY");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OrRequest {
    messages: Vec<OrMessage>,
    model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OrMessage {
    role: OrRole,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrRole {
    User,
    System,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OrResponse {
    usage: OrUsage,
    choices: Vec<OrChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OrUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OrChoice {
    message: OrMessage,
}

/// idk how accurate this is but from some testing it seems
/// to be overly optimistic, but that's what we want in this case
/// (to avoid running out of input tokens)
fn token_estimate(input: &str) -> usize {
    input.split_ascii_whitespace().count() * 2
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();

    let prompt = env::args()
        .skip(1)
        .map(|mut x| {
            x.push(' ');
            x
        })
        .collect::<String>();
    let prompt = prompt.trim();

    eprintln!("{}", "Waiting for input on stdin...".white());

    let mut input = String::new();
    let _ = stdin()
        .read_to_string(&mut input)
        .expect("stdin contained invalid UTF-8");

    let token_count = token_estimate(&input);
    println!(
        "{}",
        format!("Recieved ~{} tokens as input.", token_count).white()
    );

    let model = if token_count > FREE_MODEL_CUTOFF {
        println!(
            "{}",
            format!(
                "Using paied model ({} > {})",
                token_count, FREE_MODEL_CUTOFF
            )
            .white()
        );

        FREE_MODEL
    } else {
        PAIED_MODEL
    };

    eprintln!("{}", "Waiting for response...".white());
    let res = client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .json(&OrRequest {
            model: model.into(),
            messages: vec![
                OrMessage {
                    role: OrRole::System,
                    content: format!("DATA: {}\nEND DATA\n\n{}", input, SYSPROMPT),
                },
                OrMessage {
                    role: OrRole::User,
                    content: prompt.into(),
                },
            ],
        })
        .bearer_auth(API_KEY)
        .send()
        .await
        .unwrap();

    if !res.status().is_success() {
        panic!(
            "Request failed ({}): Is your GPTPIPE_KEY valid and does it have enough credits?",
            res.status()
        )
    }

    let res = res
        .json::<OrResponse>()
        .await
        .expect("Recieved unexpected response content");

    let ai_res = res.choices[0].message.content.as_str();

    println!("\n{}", ai_res.trim().green());
}
