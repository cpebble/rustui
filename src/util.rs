use std::{cmp::max, io, iter::zip, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Borders, List, Paragraph, Widget},
    DefaultTerminal, Frame,
};

#[derive(Debug)]
pub struct App {
    counter: u8,
    exit: bool,
    idle: bool,
    sources: Vec<usize>,
}
impl App {
    pub fn new() -> App {
        App {
            counter: 1,
            idle: true,
            exit: false,
            sources: Vec::new(),
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            self.update();
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn update(&mut self) {
        self.sources = (0..self.counter).map(|c| c as usize).collect::<Vec<usize>>();
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        let dur = Duration::from_secs(if self.idle { 10 } else { 0 });
        if !event::poll(dur)? {
            return Ok(());
        }
        while event::poll(Duration::from_secs(0))? {
            match event::read()? {
                // it's important to check that the event is a key press event as
                // crossterm also emits key release and repeat events on Windows.
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('i') => self.idle = !self.idle,
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn increment_counter(&mut self) {
        self.counter += 1;
    }

    fn decrement_counter(&mut self) {
        self.counter -= 1;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let minl = 5;
        let constraints = self.sources.iter().map(|_| Constraint::Length(minl));
        let layout = Layout::new(
            Direction::Vertical,
            constraints,
        );
        // Draw block around
        let title = Line::from(" Pulse Outputs ".bold());
        let instructions = Line::from(vec![
            " Decrement ".into(),
            "<Left>".blue().bold(),
            " Increment ".into(),
            "<Right>".blue().bold(),
            " Quit ".into(),
            "<Q> ".blue().bold(),
        ]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        // Draw each source
        let innerb = Block::default().borders(Borders::TOP);
        let ars = layout.split(block.inner(area));
        for (source, a) in zip(&self.sources, ars.iter()) {
            Paragraph::new(format!("Hello {}", source))
                .block(innerb.clone())
                .render(*a, buf);
        }
        // Draw block
        block.render(area, buf);
    }
}
