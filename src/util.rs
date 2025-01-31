use core::error::Error;
use std::{
    cmp::max,
    io::{self},
    iter::zip,
    sync::mpsc::Receiver,
    thread::{self, sleep},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use pipewire::{channel::Sender, context::Context, core::Core, main_loop::MainLoop};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};

use crate::pwrap::{Cmd, Pipewire};

static UPS: usize = 10;
static MS_PER_UPD: Duration = Duration::from_millis(1000/UPS as u64);

pub struct App {
    counter: u8,
    want_exit: bool,
    exit: bool,
    idle: bool,
    sources: Vec<usize>,
    pw_send: Sender<Cmd>,
    pw_recv: Receiver<Cmd>,
    messages: Vec<String>,
}

impl App {
    pub fn new() -> App {
        let (pw_send, pw_recv) = Pipewire::spawn().expect("Pw init failed");
        App {
            counter: 1,
            idle: true,
            want_exit: true,
            exit: false,
            sources: Vec::new(),
            pw_send,
            pw_recv,
            messages: vec!["App initialized".to_string()],
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            // Update
            self.update()?;
            // Draw ui
            terminal.draw(|frame| self.draw(frame))?;

            // Note: We can't just sleep. We should probably have some sort of interrupt handler.
            sleep(MS_PER_UPD);
        }
        ratatui::restore();
        for m in &self.messages {
            println!("message: {}", m);
        }
        Ok(())
    }

    fn update(&mut self) -> io::Result<()> {
        // Check for msgs
        match self.pw_recv.try_recv() {
            Ok(msg) => self.handle_pw_cmd(msg),
            Err(std::sync::mpsc::TryRecvError::Empty) => (),
            Err(e) => self.handle_pw_cmd(Cmd::IsDown)
        }
        // Source stuff
        self.sources = (0..self.counter)
            .map(|c| c as usize)
            .collect::<Vec<usize>>();

        // Handle keyboard events
        self.handle_events()?;
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_pw_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Terminate => (),
            Cmd::IsUp => (),
            Cmd::IsDown => {
                if self.want_exit {
                    self.messages.push("Pipewire went down properly".into());
                    self.exit = true;
                } else {
                    panic!("Pipewire wen't down unexpectedly")
                }
            }
            Cmd::Msg(s) => self.messages.push(s),
        }
    }
    fn handle_events(&mut self) -> io::Result<()> {
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
            KeyCode::Char('m') => self.messages.push("Pressed a key".to_string()),
            KeyCode::Char('z') => self.pw_send.send(Cmd::Terminate).unwrap(),
            KeyCode::Left => self.decrement_counter(),
            KeyCode::Right => self.increment_counter(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.pw_send.send(Cmd::Terminate);
        self.want_exit = true;
    }

    fn increment_counter(&mut self) {
        self.counter += 1;
    }

    fn decrement_counter(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        }
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
        let nmsg = 8;

        // Initialize block
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

        // Split layout to accommodate messages *and* sources
        let innerlayout = Layout::new(
            Direction::Vertical,
            [Constraint::Length(nmsg + 3), Constraint::Min(0)],
        );
        let innerars = innerlayout.split(block.inner(area));

        // Rendering Messages
        let msgblock = Block::bordered()
            .title(Line::from("-*Messages*"))
            .borders(Borders::all());
        //let msgs = self.messages.iter().rev().take(nmsg as usize).collect::<List>();
        let msgs = self
            .messages
            .iter()
            .map(|s| s.as_str())
            .collect::<List>()
            .block(msgblock)
            .direction(ratatui::widgets::ListDirection::TopToBottom);
        let listoffset = clamped_subtraction(
            clamped_subtraction(self.messages.len() , nmsg as usize), 1
        );
        //let listoffset = 0;
        StatefulWidget::render(
            msgs,
            innerars[0],
            buf,
            &mut ListState::default().with_offset(listoffset),
        );

        // Rendering sources
        let innerb = Block::default().borders(Borders::TOP);
        // message Layout constraints
        let constraints = self.sources.iter().map(|_| Constraint::Length(minl));
        let sourcelayout = Layout::new(Direction::Vertical, constraints);
        let ars = sourcelayout.split(innerars[1]);
        // Draw each source
        for (source, a) in zip(&self.sources, ars.iter()) {
            Paragraph::new(format!("Hello {}", source))
                .block(innerb.clone())
                .render(*a, buf);
        }
        // Draw block
        block.render(area, buf);
    }
}

pub fn clamped_subtraction(a: usize, b: usize) -> usize {
    if a < b {
        0
    } else {
        a - b
    }
}

pub fn clamp(n: usize, ma: usize, mi: usize) {}
