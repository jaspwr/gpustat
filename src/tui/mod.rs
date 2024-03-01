use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use eframe::App;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols,
    widgets::{block::Title, Axis, Block, Borders, Chart, Dataset, GraphType, LegendPosition},
    Frame, Terminal,
};

use crate::{
    data::{GpuDeviceMonitor, GpuMonitoringData},
    utils::bytes_to_mib_gib,
};

use self::views::{render_memory_chart, render_temperature_chart, render_usage_chart};

mod views;

#[derive(Default, Debug, PartialEq, Eq)]
enum SelectedProcessTab {
    #[default]
    Graphics,
    Compute,
}

#[derive(Default, Debug, PartialEq, Eq)]
enum SelectedSection {
    #[default]
    Graphics,
    Compute,
}

pub struct TuiApp {
    data: GpuMonitoringData,

    selected_process_tab: SelectedProcessTab,
    updated_style: bool,

    selected_gpu: usize,
}

impl Default for TuiApp {
    fn default() -> Self {
        Self {
            data: GpuMonitoringData::new(),
            updated_style: false,
            selected_process_tab: SelectedProcessTab::Graphics,
            selected_gpu: 0,
        }
    }
}

pub fn run_tui_app() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let tick_rate = Duration::from_millis(250);
    let app = TuiApp::default();
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn ui(frame: &mut Frame, app: &mut TuiApp) {
    let area = frame.size();

    app.data.update();

    let vertical = Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)]);
    let horizontal = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]);
    let [chart1, bottom] = vertical.areas(area);
    let [line_chart, scatter] = horizontal.areas(bottom);

    let gpu = &mut app.data.gpus()[0];

    render_usage_chart(frame, chart1, gpu);
    render_memory_chart(frame, line_chart, gpu);
    render_temperature_chart(frame, scatter, gpu);
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: TuiApp,
    tick_rate: Duration,
) -> io::Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers & KeyModifiers::CONTROL != KeyModifiers::empty() {
                    if key.code == KeyCode::Char('c') {
                        return Ok(());
                    }
                }
            }
        }
        if last_tick.elapsed() >= tick_rate {
            // app.on_tick();
            last_tick = Instant::now();
        }
    }
}
