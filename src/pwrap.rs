use std::{
    sync::mpsc::{channel as schannel, Receiver as SReceiver, Sender as SSender},
    thread,
    time::Duration,
};

use crate::cmds::Cmd;
use pipewire::{
    channel::channel as pchannel, channel::Receiver as PReceiver, channel::Sender as PSender,
    context::Context, core::Core, main_loop::MainLoop,
};

#[derive(Debug)]
pub struct Pipewire {
    ml: MainLoop,
    ctx: Context,
    core: Core,
}

impl Pipewire {
    fn new() -> Result<Pipewire, String> {
        let mainloop = match MainLoop::new(None) {
            Ok(ml) => Ok(ml),
            Err(e) => Err(e.to_string()),
        }?;
        let context = match Context::new(&mainloop) {
            Ok(c) => Ok(c),
            Err(e) => Err(e.to_string()),
        }?;
        let core = match context.connect(None) {
            Ok(it) => it,
            Err(err) => return Err(err.to_string()),
        };
        let registry = match core.get_registry() {
            Ok(it) => it,
            Err(err) => return Err(err.to_string()),
        };

        Ok(Pipewire {
            ml: mainloop,
            ctx: context,
            core,
        })
    }
    pub fn spawn() -> Result<(PSender<Cmd>, SReceiver<Cmd>), String> {
        // Build channels
        let (main_sender, main_receiver) = schannel();
        let (pw_sender, pw_receiver) = pchannel();

        // Spawn thread
        let pw_thread = thread::spawn(move || Pipewire::worker(main_sender, pw_receiver));

        // Wait for initialization
        match main_receiver.recv() {
            Ok(Cmd::IsUp) => Ok(()),
            Ok(Cmd::Msg(s)) => Err(s),
            Ok(cmd) => Err(format!("Unexpected message from worker: {:?}", cmd)),
            Err(e) => Err(format!("Receiver error: {}", e)),
        }?;
        Ok((pw_sender, main_receiver))
    }
    fn worker(send: SSender<Cmd>, recv: PReceiver<Cmd>) {
        let pw = Pipewire::new().expect("Failed to init pipewire");

        let _receiver = recv.attach(pw.ml.loop_(), {
            let mainloop = pw.ml.clone();
            let send = send.clone();
            move |cmd| {
                if let Cmd::Terminate = cmd {
                    mainloop.quit();
                    send.send(Cmd::IsDown).unwrap()
                }
            }
        });

        send.send(Cmd::IsUp).unwrap();
        pw.ml.run();
    }
}
