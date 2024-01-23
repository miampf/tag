use std::io::{stdout, BufRead, IsTerminal};
use std::{path::Path, process::Command};

use colored::Colorize;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use pest::Parser;

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

mod interactive_output {
    use crossterm::event;
    use crossterm::event::{Event, KeyCode};
    use itertools::Itertools;
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
    use ratatui::style::{Style, Stylize};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph, Tabs, Wrap};
    use ratatui::{symbols, Frame, Terminal};
    use std::io::{self, stdout};
    use std::rc::Rc;
    use tui_textarea::{Input, Key, TextArea};

    use tag::search::TaggedFile;

    use crate::execute_command_on_file;

    /// `InteractiveInputs` contains possible inputs for interactive mode.
    #[derive(Default)]
    struct InteractiveInputs {
        pub tab_index: usize,
        pub file_index: usize,
        pub scroll_index: u16,
        pub command_mode: bool,
        pub quit: bool,
    }

    pub fn interactive_output(files: &[TaggedFile], command_outputs: &[String]) -> io::Result<()> {
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

        // the command textarea
        let mut textarea = TextArea::default();
        textarea.set_cursor_line_style(Style::default());
        textarea.set_placeholder_text("Enter a command");
        textarea.set_block(Block::new().title("command").borders(Borders::all()));

        let mut interactive_inputs = InteractiveInputs::default();
        while !interactive_inputs.quit {
            let file = &files[interactive_inputs.file_index];
            let command_output = command_outputs[interactive_inputs.file_index].clone();

            terminal.draw(|frame| {
                interactive_output_ui(
                    file,
                    command_output.as_str(),
                    &mut interactive_inputs,
                    &mut textarea,
                    frame,
                );
            })?;
            interactive_inputs = handle_events(&interactive_inputs)?;

            // prevent an overflow of the index
            // and also handle wrapping
            interactive_inputs.tab_index %= 3;
            interactive_inputs.file_index %= files.len();
        }

        Ok(())
    }

    fn interactive_output_ui(
        file: &TaggedFile,
        command_output: &str,
        interactive_inputs: &mut InteractiveInputs,
        text_area: &mut TextArea,
        frame: &mut Frame,
    ) {
        if interactive_inputs.command_mode {
            interactive_inputs.command_mode = command_mode(file, text_area, frame).unwrap();
        } else {
            let area = layout(frame.size(), Direction::Vertical, &[1, 0, 1]);

            render_tabs(area[0], frame, interactive_inputs);

            render_tab_content(
                file,
                command_output,
                interactive_inputs.tab_index,
                interactive_inputs.scroll_index,
                area[1],
                frame,
            );

            render_help_menu(area[2], frame);
        }
    }

    fn render_tabs(area: Rect, frame: &mut Frame, interactive_inputs: &InteractiveInputs) {
        let tabs = Tabs::new(vec!["File Content", "Command Output", "Tags"])
            .style(Style::default().white())
            .highlight_style(Style::default().blue())
            .select(interactive_inputs.tab_index)
            .divider(symbols::DOT);

        frame.render_widget(tabs, area);
    }

    fn command_mode(
        file: &TaggedFile,
        text_area: &mut TextArea,
        frame: &mut Frame,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let layout =
            Layout::default().constraints([Constraint::Length(3), Constraint::Min(1)].as_slice());

        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => {
                return Ok(true);
            }
            Input {
                key: Key::Enter, ..
            } => {
                execute_command_on_file(&file.path, &text_area.lines()[0]);
                return Ok(false);
            }
            Input {
                key: Key::Char('m'),
                ctrl: true,
                ..
            } => {}
            input => {
                text_area.input(input);
            }
        }

        frame.render_widget(text_area.widget(), layout.split(frame.size())[0]);

        Ok(true)
    }

    fn render_tab_content(
        file: &TaggedFile,
        command_output: &str,
        tab_index: usize,
        scroll_index: u16,
        area: Rect,
        frame: &mut Frame,
    ) {
        let content = match tab_index {
            0 => std::fs::read_to_string(&file.path).unwrap(),
            1 => command_output.to_string(),
            2 => {
                let mut out_string = String::new();
                for tag in &file.tags {
                    out_string += tag.as_str();
                    out_string.push('\n');
                }
                out_string
            }
            _ => unreachable!(), // tabs are constrained to be between 0 and 2
        };

        #[allow(clippy::cast_possible_truncation)]
        let scroll_index = scroll_index % content.lines().collect_vec().len() as u16;

        let paragraph = Paragraph::new(content)
            .block(
                Block::new()
                    .title(file.path.to_str().unwrap())
                    .borders(Borders::all()),
            )
            .style(Style::new())
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false })
            .scroll((scroll_index, 0));

        frame.render_widget(paragraph, area);
    }

    fn render_help_menu(area: Rect, frame: &mut Frame) {
        let keys = [
            ("q", "Quit"),
            ("Up-Arrow/k", "Scroll Up"),
            ("Down-Arrow/j", "Scroll Down"),
            ("n", "Next File"),
            ("p", "Previous File"),
            ("Tab/Right-Arrow/l", "Next Tab"),
            ("Shift+Tab/Left-Arrow/h", "Previous Tab"),
            ("c", "Execute a command"),
        ];

        let spans = keys
            .iter()
            .flat_map(|(key, desc)| {
                let key = Span::styled(format!("| {key} "), Style::new().green());
                let desc = Span::styled(format!(" {desc} | "), Style::new().green());
                [key, desc]
            })
            .collect_vec();

        let paragraph = Paragraph::new(Line::from(spans)).alignment(Alignment::Center);

        frame.render_widget(paragraph, area);
    }

    /// simple helper method to split an area into multiple sub-areas
    /// copied and slightly modified from
    /// [here](https://docs.rs/ratatui/latest/src/demo2/root.rs.html#34)
    pub fn layout(area: Rect, direction: Direction, heights: &[u16]) -> Rc<[Rect]> {
        let constraints = heights
            .iter()
            .map(|&h| {
                if h > 0 {
                    Constraint::Length(h)
                } else {
                    Constraint::Min(0)
                }
            })
            .collect_vec();
        Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area)
    }

    fn handle_events(previous_inputs: &InteractiveInputs) -> io::Result<InteractiveInputs> {
        let mut interactive_inputs = InteractiveInputs {
            tab_index: previous_inputs.tab_index,
            file_index: previous_inputs.file_index,
            scroll_index: previous_inputs.scroll_index,
            command_mode: previous_inputs.command_mode,
            ..Default::default()
        };

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != event::KeyEventKind::Press {
                    return Ok(interactive_inputs);
                }

                match key.code {
                    KeyCode::Char('n') => interactive_inputs.file_index += 1,
                    KeyCode::Char('p') => {
                        if interactive_inputs.file_index == 0 {
                            interactive_inputs.file_index = usize::MAX;
                        } else {
                            interactive_inputs.file_index -= 1;
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => {
                        interactive_inputs.tab_index += 1;
                    }
                    KeyCode::Char('h') | KeyCode::Left | KeyCode::BackTab => {
                        interactive_inputs.tab_index -= 1;
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if interactive_inputs.scroll_index == 0 {
                            interactive_inputs.scroll_index = u16::MAX;
                        } else {
                            interactive_inputs.scroll_index -= 1;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => {
                        if interactive_inputs.scroll_index == u16::MAX {
                            interactive_inputs.scroll_index = u16::MIN;
                        } else {
                            interactive_inputs.scroll_index += 1;
                        }
                    }
                    KeyCode::Char('c') => interactive_inputs.command_mode = true,
                    KeyCode::Char('q') => interactive_inputs.quit = true,
                    _ => return Ok(interactive_inputs),
                }
            }
        }

        Ok(interactive_inputs)
    }
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

        if !args.interactive {
            non_interactive_output(&file, output.as_str());
        }

        file_matched_index.push(file);
        command_outputs.push(output);
    }

    if args.interactive {
        interactive_output::interactive_output(&file_matched_index, &command_outputs)?;

        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
    }

    Ok(())
}
