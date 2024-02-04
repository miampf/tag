use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame, Terminal,
};

/// `InteractiveInputs` is updated with all inputs done in the TUI.
#[derive(Default)]
struct InteractiveInputs {
    quit: bool,
}

/// `ui` renders the UI of the explore mode.
///
/// # Errors
///
/// This function errors if it fails to draw the output
/// or get the input.
pub fn ui() -> Result<(), Box<dyn std::error::Error>> {
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let mut interactive_inputs = InteractiveInputs::default();
    while !interactive_inputs.quit {
        terminal.draw(render)?;

        interactive_inputs = handle_events()?;
    }

    Ok(())
}

fn render(frame: &mut Frame) {
    let main_layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(70), Constraint::Percentage(30)],
    )
    .split(frame.size());

    frame.render_widget(
        Block::new().title("main").borders(Borders::all()),
        main_layout[0],
    );
    frame.render_widget(
        Block::new().title("sub").borders(Borders::all()),
        main_layout[1],
    );
}

fn handle_events() -> std::io::Result<InteractiveInputs> {
    let mut interactive_inputs = InteractiveInputs::default();
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind != event::KeyEventKind::Press {
                return Ok(interactive_inputs);
            }

            match key.code {
                KeyCode::Char('q') => interactive_inputs.quit = true,
                _ => return Ok(interactive_inputs),
            }
        }
    }

    Ok(interactive_inputs)
}
