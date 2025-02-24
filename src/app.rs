use core::error::Error;
use std::{
    cmp::max,
    io::{self},
    iter::zip,
    sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender},
    thread::{self, sleep},
    time::Duration,
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use pipewire::{channel::Sender as PSender, context::Context, core::Core, main_loop::MainLoop};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Borders, List, ListState, Paragraph, StatefulWidget, Widget},
    DefaultTerminal, Frame,
};

use crate::cmds::{combine_receivers, Cmd};
use crate::pwrap::Pipewire;

static UPS: usize = 1;
static MS_PER_UPD: Duration = Duration::from_millis(1000 / UPS as u64);

pub struct App {
    counter: u8,
    want_exit: bool,
    exit: bool,
    idle: bool,
    sources: Vec<usize>,
    pw_send: PSender<Cmd>,
    receiver: Receiver<Cmd>,
    messages: Vec<String>,
}

impl App {
    pub fn new() -> App {
        // Setup a pipewire instance
        let (pw_send, pw_recv) = Pipewire::spawn().expect("Pw init failed");
        // Setup a channel to receive ui events
        let (ui_send, ui_recv) = channel();
        terminal_eventthread(ui_send);
        // Tie the receivers together
        let recver = combine_receivers(pw_recv, ui_recv);
        App {
            counter: 1,
            idle: true,
            want_exit: true,
            exit: false,
            sources: Vec::new(),
            pw_send,
            receiver: recver,
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
        }
        ratatui::restore();
        for m in &self.messages {
            println!("message: {}", m);
        }
        Ok(())
    }

    fn update(&mut self) -> io::Result<()> {
        match self.receiver.recv_timeout(MS_PER_UPD) {
            Ok(c) => Ok(self.handle_cmd(c)),
            Err(RecvTimeoutError::Timeout) => Ok(()),
            // TODO: Proper error bubbling
            Err(_) => panic!("Receiver closed unexpectedly"),
        }
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_cmd(&mut self, cmd: Cmd) {
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
            Cmd::KeyPress(kp) => self.handle_key_event(kp),
            Cmd::Msg(s) => self.messages.push(s),
        }
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
        let listoffset =
            clamped_subtraction(clamped_subtraction(self.messages.len(), nmsg as usize), 1);
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

fn terminal_eventthread(sendchannel: Sender<Cmd>) {
    thread::spawn(move || loop {
        let Ok(ev) = event::read() else {
            break;
        };
        if let Event::Key(key_event) = ev {
            sendchannel.send(Cmd::KeyPress(key_event)).unwrap()
        }
    });
}
