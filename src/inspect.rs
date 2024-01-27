use crossterm::event;
use crossterm::event::{Event, KeyCode};
use itertools::Itertools;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap};
use ratatui::{symbols, Frame, Terminal};
use std::io::{self, stdout};
use std::rc::Rc;
use tui_textarea::{Input, Key, TextArea};

use crate::search::TaggedFile;

use crate::commands::execute_command_on_file;

/// `InteractiveInputs` contains possible inputs for interactive mode.
#[derive(Default)]
struct InteractiveInputs {
    pub tab_index: usize,
    pub file_index: usize,
    pub scroll_index: u16,
    pub command_mode: bool,
    pub quit: bool,
}

/// `interactive_output` handles the interactive UI of the inspect mode.
///
/// # Errors
///
/// This function returns an error if rendering or handling inputs fails.
pub fn interactive_output(files: &[TaggedFile], command_outputs: &[String]) -> io::Result<()> {
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // the command textarea
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(Style::default());
    textarea.set_placeholder_text("Enter a command");
    textarea.set_block(
        Block::new()
            .title("command")
            .borders(Borders::all())
            .border_style(Style::default().red().on_black())
            .style(Style::default().black().on_white()),
    );

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

/// `interactive_output_ui` renders the UI.
fn interactive_output_ui(
    file: &TaggedFile,
    command_output: &str,
    interactive_inputs: &mut InteractiveInputs,
    text_area: &mut TextArea,
    frame: &mut Frame,
) {
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

    if interactive_inputs.command_mode {
        interactive_inputs.command_mode = command_mode_input(file, text_area).unwrap();
        command_mode_render(text_area, frame);
    }
}

/// `render_tabs` renders the tabs at the top of the screen.
fn render_tabs(area: Rect, frame: &mut Frame, interactive_inputs: &InteractiveInputs) {
    let tabs = Tabs::new(vec!["File Content", "Command Output", "Tags"])
        .style(Style::default().white())
        .highlight_style(Style::default().blue())
        .select(interactive_inputs.tab_index)
        .divider(symbols::DOT);

    frame.render_widget(tabs, area);
}

/// `render_tab_content` renders the main content of the current tab.
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
    let scroll_index = if content.is_empty() {
        0
    } else {
        scroll_index % content.lines().collect_vec().len() as u16
    };

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

/// `render_help_menu` renders the help menu at the bottom of the screen.
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
fn layout(area: Rect, direction: Direction, heights: &[u16]) -> Rc<[Rect]> {
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

/// `handle_events` handles inputs in normal mode.
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
                    if interactive_inputs.tab_index != 0 {
                        interactive_inputs.tab_index -= 1;
                    } else {
                        interactive_inputs.tab_index = 2; // 2 = last tab
                    }
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

/// `command_mode_render` renders the popup for a command in the middle of the screen.
fn command_mode_render(text_area: &mut TextArea, frame: &mut Frame) {
    let layout =
        Layout::default().constraints([Constraint::Length(3), Constraint::Min(1)].as_slice());

    let area = Rect::new(0, frame.size().height / 2, frame.size().width, 10);

    frame.render_widget(Clear, layout.split(area)[0]);
    frame.render_widget(text_area.widget(), layout.split(area)[0]);
}

/// `command_mode_input` handles inputs in command mode.
fn command_mode_input(file: &TaggedFile, text_area: &mut TextArea) -> Result<bool, std::io::Error> {
    match crossterm::event::read()?.into() {
        Input { key: Key::Esc, .. } => {
            return Ok(false);
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

    Ok(true)
}
