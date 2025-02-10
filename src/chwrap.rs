use std::marker::Send;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::thread;

#[derive(Debug)]
pub enum Cmd {
    Terminate,
    IsUp,
    IsDown,
    Msg(String),
}

unsafe impl Send for Cmd {}

fn recvthread<A>(recv: Receiver<A>, send: Sender<A>, errsend: Sender<RecvError>) {
    loop {
        match recv.recv() {
            Ok(msg) => send.send(msg).unwrap(),
            Err(err) => {
                errsend.send(err).unwrap();
                break;
            }
        }
    }
}

fn combine_receivers<A>(
    recv_1: Receiver<A>,
    recv_2: Receiver<A>,
) -> (Receiver<A>, Receiver<RecvError>)
where
    A: Send,
{
    let (send, recv) = channel::<A>();
    let (errsend, errrecv) = channel::<RecvError>();

    thread::spawn({
        let send = send.clone();
        let errsend = errsend.clone();
        move || recvthread(recv_1, send, errsend.clone())
    });
    thread::spawn({
        let send = send.clone();
        let errsend = errsend.clone();
        move || recvthread(recv_2, send, errsend.clone())
    });
    (recv, errrecv)
}
