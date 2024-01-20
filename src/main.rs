use std::{path::PathBuf, process::Command};

use pest::Parser;
use tag::{
    parsers::query::{construct_query_ast, evaluate_ast, QueryParser, Rule},
    search::get_tags_from_files,
};

mod cli {
    use clap::Parser;

    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    pub struct Cli {
        #[clap(value_name = "QUERY")]
        /// Search query for the tags.
        pub query: String,

        #[clap(value_name = "PATH")]
        /// The path that will be searched.
        pub path: String,

        #[arg(short, long)]
        /// Only print the paths of matched files.
        pub silent: bool,

        #[arg(short, long)]
        /// A command that will be executed on matched files.
        pub command: String,
    }

    impl Cli {
        pub fn new_and_parse() -> Cli {
            Cli::parse()
        }
    }
}

fn execute_command_on_file(path: PathBuf, command: String) -> String {
    let command = command.replace("#FILE#", path.to_str().unwrap());

    let output = Command::new("bash").arg("-c").arg(command.clone()).output();

    if let Err(e) = &output {
        println!("Wasn't able to execute command {}: {}", command, e);
    }

    let output = output.unwrap();
    let output_string = std::str::from_utf8(output.stdout.as_slice());

    if let Err(e) = &output_string {
        println!("Failed to get output from command {}: {}", command, e);
    }

    output_string.unwrap().to_string()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::new_and_parse();

    let file_index = get_tags_from_files(args.path.as_str())?;
    let query = QueryParser::parse(Rule::tagsearch, args.query.as_str());

    if let Err(e) = &query {
        println!("Error: {}", e);
        std::process::exit(1);
    }

    let query = query.unwrap();

    for file in file_index.iter() {
        let ast = construct_query_ast(
            query.clone().next().unwrap().into_inner(),
            file.tags.iter().map(|tag| tag.as_str()).collect(),
        );

        if !evaluate_ast(ast) {
            continue;
        }

        println!("{}", file.path.display());

        let mut output = String::new();
        if !args.command.is_empty() {
            output = execute_command_on_file(file.path.clone(), args.command.clone());
        }

        if !args.silent {
            println!("\ttags:{:?} ", file.tags);
            println!("\tOutput of command:\n{}", output);
        }
    }

    Ok(())
}
