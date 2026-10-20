use std::io::{self, Stdout};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Terminal guard: manages raw mode, alternate screen, and mouse capture.
/// Ensures terminal is restored to normal state even on panic or error.
pub struct Screen {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Screen {
    pub fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        if let Err(error) = execute!(stdout, EnterAlternateScreen, EnableMouseCapture) {
            restore_terminal();
            return Err(error);
        }

        // Install a panic hook so that panics restore the terminal first,
        // allowing the panic message to be read clearly on the normal screen.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore_terminal();
            previous(info);
        }));

        match Terminal::new(CrosstermBackend::new(stdout)) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(error) => {
                restore_terminal();
                Err(error)
            }
        }
    }

    /// Temporarily hand over the terminal to an external process (e.g. `sisho-desk`),
    /// and restore alternate screen and raw mode afterwards.
    pub fn execute_handover<F, R>(&mut self, f: F) -> io::Result<R>
    where
        F: FnOnce() -> R,
    {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        let _ = self.terminal.show_cursor();

        let result = f();

        enable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(result)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        restore_terminal();
        let _ = self.terminal.show_cursor();
    }
}

/// Restore the terminal to cooked mode and exit alternate screen. Safe to call multiple times.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}
