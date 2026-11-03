use anyhow::Error;
use clap::Parser;
use dotenv::dotenv;
use figlet_rs::Toilet;
use futures::StreamExt;
use nu_ansi_term::Color;
use rig::agent::MultiTurnStreamItem;
use rig::prelude::*;
use rig::providers::openai::{self, OpenAIConfig};
use rig::streaming::{Item, StreamEvent};
use rig::tool::DynamicTool;
use rig_rmcp::McpConnection;
use rig_rmcp::McpTool;
use rmcp::ServiceExt;
use std::env;
use std::io::Write;
use tokio::process::Command;
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct ArgsCli {
    ///Choose between api or local muse model
    #[arg(long, short = 'L')]
    local: bool,
    ///prompt system that give agent role
    #[arg(long = "system", short = 's')]
    sys: String,
    ///Temperature 0.0 to 1.0
    #[arg(long, short = 't')]
    temp: f64,
    ///Maximum windows context tokens
    #[arg(long = "max_token", short = 'x')]
    max_t: u64,
}
#[tokio::main]
async fn main() -> anyhow::Result<(), Error> {
    let future_font = Toilet::future().unwrap();
    let a = future_font
        .convert("Welcom to Rust AI hackaton")
        .unwrap()
        .to_string();
    println!("{}", Color::Blue.paint(a));
    println!("------------------------");
    println!("{}", Color::Green.paint("Empower by Meta muse-ai"));
    println!("------------------------");
    println!("------------------------");
    println!("%%%%%%%%%%%%%%%%%%%%%%%%");
    println!("{}", Color::Cyan.paint("Design by gaetan musinde in DRC"));
    println!("%%%%%%%%%%%%%%%%%%%%%%%%");
    let option = ArgsCli::parse();
    let model = if option.local {
        println!("connect to llamacpp server....");
        use rig::providers::llamacpp;
        llamacpp::from_env()?.completion(llamacpp::LLAMA_CPP)
    } else {
        dotenv().ok();

        let option_gateway = vec![
            "htts://api.meta.ai/v1",
            "https://integrate.api.nvidia.com/v1/",
        ];
        let option_api_key = vec!["META_API_KEY", "NVIVIA_API_KEY"];

        let option_model = vec![
            "muse-wonder-1.0",
            "muse-spart-1.3",
            "muse-spark-1.2",
            "muse-spark-1.1",
            "muse-spark1.3-contributor",
            "muse-spark-contributor",
            "meta/muse-glimmer-30b",
        ];

        let gateway =
            inquire::Select::new("What's your gatway favorite Llm model ?", option_gateway)
                .prompt()
                .unwrap();
        let api_var = inquire::Select::new("What's your api key?", option_api_key)
            .prompt()
            .unwrap();
        let model = inquire::Select::new("What's your favorite meta model?", option_model)
            .prompt()
            .unwrap(); // define  client du model heberger sur le cloud nvidia

        //replace your nvidia api key here
        let api_2 = "--------------------------------".to_string(); // make your api key here
        let api_key = env::var(api_var).unwrap_or(api_2);
        let dialect = openai::wire::Dialect::gateway("api", gateway, "");

        // define  open model
        OpenAIConfig::with_key(&dialect, api_key)
            .client()
            .completion(model)
    };

    //>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> Client MCP>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>

    // replace your path here
    let transport = rmcp::transport::TokioChildProcess::new(Command::new(
        "------------------------", // call mcp server here
    ))?;
    let client = ().serve(transport).await?;
    let c = client.clone();
    let connection = McpConnection::new(client);
    let t = c.list_all_tools().await?;

    let tools = t
        .into_iter()
        .map(|tool| DynamicTool::try_from(McpTool::new(tool, &connection)))
        .collect::<Result<Vec<_>, _>>()?;

    //>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>Agent AI >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
    let agent = AgentBuilder::new(model)
        .preamble(option.sys)
        .temperature(option.temp)
        .max_tokens(option.max_t)
        .dynamic_tools(tools)
        .default_max_turns(10)
        .build();
    //>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>Prompt>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
    let mut input_prompt = String::new();

    loop {
        println!("input prompt >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
        std::io::stdin()
            .read_line(&mut input_prompt)
            .expect("fail to get input prompt");
        println!("output >>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>");
        if input_prompt.is_empty() {
            break;
        } else {
            let mut stream = agent.prompt(&input_prompt).stream();

            while let Some(item) = stream.next().await {
                match item? {
                    MultiTurnStreamItem::StreamAssistantItem(Item::Event(StreamEvent::Text {
                        text,
                        ..
                    })) => {
                        print!("{text}");
                        std::io::stdout().flush()?;
                    }
                    MultiTurnStreamItem::FinalResponse(response) => {
                        println!(
                            "\n\n{} {}",
                            Color::Red
                                .paint(response.usage().total_tokens.unwrap_or(0).to_string()),
                            Color::Yellow.paint("tokens used")
                        );
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
