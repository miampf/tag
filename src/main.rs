use std::io::{stdout, BufRead, IsTerminal};

use colored::Colorize;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use pest::Parser;

use tag::commands::{execute_command_on_file, execute_filter_command_on_file};
use tag::inspect;
use tag::search::TaggedFile;
use tag::{
    parsers::searchquery::{construct_query_ast, evaluate_ast, QueryParser, Rule},
    search::get_tags_from_files,
};

mod cli {
    use clap::Parser;

    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    #[allow(clippy::struct_excessive_bools)]
    pub struct Cli {
        #[clap(value_name = "PATH")]
        /// The path that will be searched.
        pub path: String,

        #[clap(value_name = "QUERY", group = "q-input")]
        /// Search query for the tags.
        pub query: Option<String>,

        #[arg(short, long, group = "output")]
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

        #[arg(short, long, group = "q-input")]
        /// Receive a query from the standard input.
        pub query_stdin: bool,

        #[arg(short, long, group = "output")]
        /// Enter an interactive inspection mode to view each file individually.
        pub inspect: bool,
    }

    impl Cli {
        pub fn new_and_parse() -> Self {
            Self::parse()
        }
    }
}

fn non_interactive_output(file: &TaggedFile, command_output: &str) {
    println!("\t{}", format!("tags: {:?}", file.tags).blue());

    if !command_output.is_empty() {
        println!(
            "\tOutput of command:\n{}",
            textwrap::indent(command_output, "\t\t")
        );
    }
}

fn log_error(msg: &str, e: Box<dyn std::error::Error>) {
    eprintln!(
        "{} {} {}",
        "[ERROR]".red().bold(),
        msg.red(),
        e.to_string().red().underline()
    );
}

fn main() {
    let mut args = cli::Cli::new_and_parse();

    // detect if output is in a terminal or not
    if !stdout().is_terminal() {
        args.silent = true;
        args.no_color = true;
    }

    if args.no_color {
        colored::control::set_override(false);
    }

    if !args.query_stdin && args.query.is_none() {
        eprintln!(
            "{} {}",
            "[ERROR]".red().bold(),
            "Please provide a query, either through stdin or by manually adding it.".red()
        );
        std::process::exit(1);
    }

    // fetch the query
    let query = if args.query.is_some() {
        args.query.unwrap()
    } else {
        let mut query = String::new();
        if let Err(e) = std::io::stdin().lock().read_line(&mut query) {
            log_error("Failed to read query from stdin:", Box::new(e));
            std::process::exit(1);
        }
        query
    };

    let file_index = match get_tags_from_files(args.path.as_str()) {
        Ok(index) => index,
        Err(e) => {
            log_error("Failed to build file index:", e);
            std::process::exit(1);
        }
    };
    let query = match QueryParser::parse(Rule::tagsearch, query.as_str()) {
        Ok(query) => query,
        Err(e) => {
            eprintln!(
                "{} {}\n{}",
                "[ERROR]".red().bold(),
                "Invalid query: ".red(),
                e.to_string().red()
            );
            std::process::exit(1);
        }
    };

    if args.inspect {
        if let Err(e) = enable_raw_mode() {
            log_error("Failed to enable raw mode:", Box::new(e));
            std::process::exit(1);
        }
        if let Err(e) = stdout().execute(EnterAlternateScreen) {
            log_error("Failed to enter alternate screen: ", Box::new(e));
        }
    }

    let mut file_matched_index = Vec::new();
    let mut command_outputs = Vec::new();

    for file in file_index {
        let ast = construct_query_ast(
            query.clone().next().unwrap().into_inner(),
            &file.tags.iter().map(std::string::String::as_str).collect(),
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

        if !args.inspect {
            println!("{}", file.path.display().to_string().green());
        }

        let output = if args.command.is_some() {
            execute_command_on_file(&file.path, &args.command.clone().unwrap())
        } else {
            String::new()
        };

        // don't print any more information in silent mode
        if args.silent {
            continue;
        }

        if !args.inspect {
            non_interactive_output(&file, output.as_str());
        }

        file_matched_index.push(file);
        command_outputs.push(output);
    }

    if args.inspect {
        if let Err(e) = inspect::interactive_output(&file_matched_index, &command_outputs) {
            log_error("Failed to enter interactive output mode:", Box::new(e));
            std::process::exit(1);
        }

        if let Err(e) = disable_raw_mode() {
            log_error("Failed to disable raw mode:", Box::new(e));
        }
        if let Err(e) = stdout().execute(LeaveAlternateScreen) {
            log_error("Failed to leave alternate screen:", Box::new(e));
            std::process::exit(1);
        }
    }
}
