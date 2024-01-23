use std::io::{self, stdout, BufRead, IsTerminal};
use std::{path::Path, process::Command};

use colored::Colorize;
use crossterm::terminal::{
    self, disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use pest::Parser;
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};
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
        pub interactive: bool,
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

fn non_interactive_output(file: &TaggedFile, command_output: &str) {
    println!("\t{}", format!("tags: {:?}", file.tags).blue());

    if !command_output.is_empty() {
        println!(
            "\tOutput of command:\n{}",
            textwrap::indent(command_output, "\t\t")
        );
    }
}

fn interactive_output(file: &TaggedFile, command_output: &str) -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut show_next = false;
    while !show_next {
        terminal.draw(|frame| interactive_output_ui(file, command_output, frame))?;
        show_next = handle_events()?;
    }

    Ok(())
}

fn interactive_output_ui(file: &TaggedFile, command_output: &str, frame: &mut Frame) {
    todo!()
}

fn handle_events() -> io::Result<bool> {
    todo!()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        std::io::stdin().lock().read_line(&mut query)?;
        query
    };

    let file_index = get_tags_from_files(args.path.as_str())?;
    let query = QueryParser::parse(Rule::tagsearch, query.as_str());

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

    if args.interactive {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
    }

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

        if !args.interactive {
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

        if args.interactive {
            todo!()
        } else {
            non_interactive_output(&file, output.as_str());
        }
    }

    if args.interactive {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
    }

    Ok(())
}
