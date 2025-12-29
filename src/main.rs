mod model;
mod sampler;
mod ui;

use crossterm::{
    cursor::{Hide, Show},
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, SetTitle},
};
use model::Snapshot;
use ratatui::{Terminal, backend::CrosstermBackend};
use sampler::{PlatformSampler as Sampler, Sampler as _};
use std::io::{self, Write, stdout};
use std::time::{Duration, Instant};

fn parse_args() -> Duration {
    let args: Vec<String> = std::env::args().collect();
    let mut interval_ms: u64 = 1000;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-i" | "--interval" => {
                if let Some(val) = args.get(i + 1) {
                    interval_ms = val.parse().unwrap_or(1000);
                    i += 1;
                }
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    Duration::from_millis(interval_ms.max(100))
}

fn print_help() {
    println!("htop_mini - A lightweight system monitor");
    println!();
    println!("USAGE:");
    println!("    htop_mini [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -i, --interval <MS>  Sampling interval in milliseconds [default: 1000]");
    println!("    -h, --help           Print help information");
}

fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let mut stdout = stdout();

    terminal::enable_raw_mode()?;

    execute!(
        stdout,
        EnterAlternateScreen,
        Clear(ClearType::All),
        Clear(ClearType::Purge),
        Hide,
        EnableMouseCapture,
        SetTitle("htop_mini")
    )?;
    stdout.flush()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    Ok(terminal)
}

fn restore_terminal() {
    let mut stdout = stdout();

    let _ = execute!(
        stdout,
        DisableMouseCapture,
        Clear(ClearType::All),
        Show,
        LeaveAlternateScreen
    );
    let _ = stdout.flush();
    let _ = terminal::disable_raw_mode();
    let _ = println!();
}

fn main() -> io::Result<()> {
    let sample_interval = parse_args();
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, sample_interval);
    restore_terminal();
    result
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    sample_interval: Duration,
) -> io::Result<()> {
    let mut sampler = Sampler::new();
    let mut previous_sample = sampler.sample().ok();
    let mut last_sample_time = Instant::now();

    let mut snapshot: Option<Snapshot> = None;

    loop {
        terminal.draw(|f| ui::render(f, snapshot.as_ref()))?;

        let timeout = sample_interval
            .checked_sub(last_sample_time.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        if last_sample_time.elapsed() >= sample_interval {
            if let Ok(sample) = sampler.sample() {
                if let Some(ref prev) = previous_sample {
                    snapshot = Some(Snapshot::compute(&sample, prev));
                }
                previous_sample = Some(sample);
            }
            last_sample_time = Instant::now();
        }
    }
}
