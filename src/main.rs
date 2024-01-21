use std::{path::Path, process::Command};

use colored::Colorize;
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
        pub command: Option<String>,

        #[arg(short, long)]
        /// A command that must run successfully for a file to be accepted.
        pub filter_command: Option<String>,

        #[arg(short, long)]
        /// Disable coloring.
        pub no_color: bool,
    }

    impl Cli {
        pub fn new_and_parse() -> Self {
            Self::parse()
        }
    }
}

fn execute_command_on_file(path: &Path, command: &str) -> String {
    let command = command.replace("#FILE#", path.to_str().unwrap());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(command.clone()).output()
    } else {
        Command::new("bash").arg("-c").arg(command.clone()).output()
    };

    if let Err(e) = &output {
        eprintln!(
            "{} Wasn't able to execute command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    let output = output.unwrap();
    let output_string = std::str::from_utf8(output.stdout.as_slice());

    if let Err(e) = &output_string {
        eprintln!(
            "{} Failed to get output from command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    output_string.unwrap().to_string()
}

fn execute_filter_command_on_file(path: &Path, command: &str) -> bool {
    let command = command.replace("#FILE#", path.to_str().unwrap());

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").arg("/C").arg(command.clone()).output()
    } else {
        Command::new("bash").arg("-c").arg(command.clone()).output()
    };

    if let Err(e) = &output {
        eprintln!(
            "{} Wasn't able to execute command {}: {}",
            "[ERROR]".red().bold(),
            command.blue().underline(),
            e.to_string().red()
        );
    }

    output.unwrap().status.success()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = cli::Cli::new_and_parse();

    if args.no_color {
        colored::control::set_override(false);
    }

    let file_index = get_tags_from_files(args.path.as_str())?;
    let query = QueryParser::parse(Rule::tagsearch, args.query.as_str());

    if let Err(e) = &query {
        eprintln!(
            "{} {}\n{}",
            "[ERROR]".red().bold(),
            "Invalid query".red(),
            e.to_string().red()
        );
        std::process::exit(1);
    }

    let query = query.unwrap();

    for file in file_index {
        let ast = construct_query_ast(
            query.clone().next().unwrap().into_inner(),
            file.tags.iter().map(std::string::String::as_str).collect(),
        );

        // skip the file if tags don't match query
        if !evaluate_ast(ast) {
            continue;
        }

        // skip the file if filter command is unsuccessful
        if args.filter_command.is_some()
            && !execute_filter_command_on_file(&file.path, &args.filter_command.clone().unwrap())
        {
            continue;
        }

        println!("{}", file.path.display().to_string().green());

        let output = if args.command.is_some() {
            execute_command_on_file(&file.path, &args.command.clone().unwrap())
        } else {
            String::new()
        };

        if !args.silent {
            println!("\t{}", format!("tags: {:?}", file.tags).blue());

            if !output.is_empty() {
                println!(
                    "\tOutput of command:\n{}",
                    textwrap::indent(output.as_str(), "\t\t")
                );
            }
        }
    }

    Ok(())
}
