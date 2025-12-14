mod model;
mod sampler;
mod ui;

use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use model::Snapshot;
use sampler::{PlatformSampler as Sampler, Sampler as _};
use std::io::Result;
use std::time::{Duration, Instant};

const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut sampler = Sampler::new();
    let mut previous_sample = sampler.sample().ok();
    let mut last_sample_time = Instant::now();

    let mut snapshot: Option<Snapshot> = None;

    loop {
        terminal.draw(|f| ui::render(f, snapshot.as_ref()))?;

        let timeout = SAMPLE_INTERVAL
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

        if last_sample_time.elapsed() >= SAMPLE_INTERVAL {
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
