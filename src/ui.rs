use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::model::Snapshot;

const BAR_CHAR: &str = "|";

pub fn render(f: &mut Frame, snapshot: Option<&Snapshot>) {
    match snapshot {
        Some(s) => draw(f, s),
        None => draw_loading(f),
    }
}

fn draw_loading(f: &mut Frame) {
    let block = Block::default()
        .title("Initializing...")
        .borders(Borders::ALL);
    f.render_widget(block, f.area());
}

fn draw(f: &mut Frame, snapshot: &Snapshot) {
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(1)]).split(f.area());

    draw_meters(f, snapshot, chunks[0]);
    draw_help(f, chunks[1]);
}

fn draw_meters(f: &mut Frame, snapshot: &Snapshot, area: Rect) {
    let cpu_count = snapshot.cpu_count;
    let cols_count = 3;
    let cpu_rows = (cpu_count + cols_count - 1) / cols_count;

    let total_rows = cpu_rows + 5; // 5 - Mem, Swap, Tasks, Load, Uptime
    let row_constraints: Vec<Constraint> = vec![Constraint::Length(1); total_rows];
    let rows = Layout::vertical(row_constraints).split(area);

    let col_constraints = [
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ];

    for row_idx in 0..cpu_rows {
        let cols = Layout::horizontal(col_constraints).split(rows[row_idx]);
        let bar_width = cols[0].width.saturating_sub(1) as usize;

        for col_idx in 0..cols_count {
            let cpu_idx = row_idx + col_idx * cpu_rows;
            if cpu_idx < cpu_count {
                let cpu = &snapshot.cpu_usage[cpu_idx];
                let line = render_cpu_bar(cpu_idx, cpu.user_percent, cpu.system_percent, bar_width);
                f.render_widget(Paragraph::new(line), cols[col_idx]);
            }
        }
    }

    let mem = &snapshot.memory_stats;
    let full_width = area.width.saturating_sub(2) as usize;

    let mem_row = cpu_rows;
    f.render_widget(
        Paragraph::new(render_mem_bar(
            "Mem ",
            mem.used_gb(),
            mem.total_gb(),
            full_width,
            Color::Green,
        )),
        rows[mem_row],
    );
    f.render_widget(
        Paragraph::new(render_mem_bar(
            "Swp",
            mem.swap_used_gb(),
            mem.swap_total_gb(),
            full_width,
            Color::Red,
        )),
        rows[mem_row + 1],
    );

    f.render_widget(
        Paragraph::new(render_tasks_line(snapshot)),
        rows[mem_row + 2],
    );
    f.render_widget(
        Paragraph::new(render_load_line(snapshot)),
        rows[mem_row + 3],
    );
    f.render_widget(
        Paragraph::new(render_uptime_line(snapshot)),
        rows[mem_row + 4],
    );
}

fn render_cpu_bar(index: usize, user_pct: f64, sys_pct: f64, width: usize) -> Line<'static> {
    let label = format!("{:2}", index);
    let total_pct = (user_pct + sys_pct).clamp(0.0, 100.0);

    // Bar area is width minus label and brackets and percentage: "NN[...XXXX%]"
    // Label=2, [=1, ]=1, percentage=5 (XXX.X) + % = roughly 10 chars overhead
    let bar_space = width.saturating_sub(12);
    let user_bars = ((user_pct / 100.0) * bar_space as f64) as usize;
    let sys_bars = ((sys_pct / 100.0) * bar_space as f64) as usize;
    let empty = bar_space.saturating_sub(user_bars + sys_bars);

    let pct_str = format!("{:5.1}%", total_pct);

    Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Cyan)),
        Span::raw("["),
        Span::styled(
            BAR_CHAR.repeat(user_bars),
            Style::default().fg(Color::Green),
        ),
        Span::styled(BAR_CHAR.repeat(sys_bars), Style::default().fg(Color::Red)),
        Span::raw(" ".repeat(empty)),
        Span::styled(pct_str, Style::default().fg(Color::White)),
        Span::raw("]"),
    ])
}

fn render_mem_bar(label: &str, used: f64, total: f64, width: usize, color: Color) -> Line<'static> {
    let pct = if total > 0.0 {
        (used / total).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Format: "Mem[|||||||||      X.XXG/XX.XG]"
    let size_str = format!("{:.2}G/{:.1}G", used, total);
    let bar_space = width.saturating_sub(label.len() + 2 + size_str.len() + 1); // label + [] + size + space
    let filled = (pct * bar_space as f64) as usize;
    let empty = bar_space.saturating_sub(filled);

    Line::from(vec![
        Span::styled(label.to_string(), Style::default().fg(Color::Cyan)),
        Span::raw("["),
        Span::styled(BAR_CHAR.repeat(filled), Style::default().fg(color)),
        Span::raw(" ".repeat(empty)),
        Span::styled(size_str, Style::default().fg(Color::White)),
        Span::raw("]"),
    ])
}

fn render_tasks_line(snapshot: &Snapshot) -> Line<'static> {
    let ts = &snapshot.task_stats;
    Line::from(vec![
        Span::styled("Tasks: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!("{}, {} thr; ", ts.total_tasks, ts.total_threads)),
        Span::styled(
            format!("{}", ts.running_threads),
            Style::default().fg(Color::Green),
        ),
        Span::raw(" running"),
    ])
}

fn render_load_line(snapshot: &Snapshot) -> Line<'static> {
    let la = &snapshot.load_average;
    Line::from(vec![
        Span::styled("Load average: ", Style::default().fg(Color::Cyan)),
        Span::raw(format!(
            "{:.2} {:.2} {:.2}",
            la.one_min, la.five_min, la.fifteen_min
        )),
    ])
}

fn render_uptime_line(snapshot: &Snapshot) -> Line<'static> {
    let u = &snapshot.uptime;
    let uptime_str = if u.days > 0 {
        format!(
            "{} days, {:02}:{:02}:{:02}",
            u.days, u.hours, u.minutes, u.seconds
        )
    } else {
        format!("{:02}:{:02}:{:02}", u.hours, u.minutes, u.seconds)
    };
    Line::from(vec![
        Span::styled("Uptime: ", Style::default().fg(Color::Cyan)),
        Span::raw(uptime_str),
    ])
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_items = [("q", "Quit")];

    let spans: Vec<Span> = help_items
        .iter()
        .flat_map(|(key, desc)| {
            vec![
                Span::styled(*key, Style::default().fg(Color::Black).bg(Color::Cyan)),
                Span::raw(format!("{} ", desc)),
            ]
        })
        .collect();

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
