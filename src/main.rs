use std::{
    env,
    io::{self, Stdout},
    path::PathBuf,
    time::Duration,
};

use anyhow::Result;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kmutnb_phylab_plotter::{app::App, file_io, input, ui};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    let arg_path = env::args_os().nth(1).map(PathBuf::from);
    let mut app = match arg_path {
        Some(path) => match file_io::load_csv(&path) {
            Ok(data) => App::with_data(data, Some(path)),
            Err(err) => {
                let mut app = App::default();
                app.set_status(format!("Could not open file: {err}"));
                app
            }
        },
        None => App::default(),
    };

    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, &mut app);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if !input::handle_key(app, key) {
                    break;
                }
            }
        }
    }

    Ok(())
}
