use std::marker::Send;
use std::sync::mpsc::{channel, Receiver, RecvError, Sender};
use std::sync::Arc;
use std::thread;

use crossterm::event::KeyEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum Cmd {
    Terminate,
    IsUp,
    IsDown,
    KeyPress(KeyEvent),
    Msg(String),
}

pub fn combine_receivers(recv_1: Receiver<Cmd>, recv_2: Receiver<Cmd>) -> Receiver<Cmd> {
    let (send, recv) = channel::<Cmd>();

    thread::spawn({
        let send = send.clone();
        move || {
            for r in recv_1 {
                send.send(r).unwrap();
            }
        }
    });
    thread::spawn(move || {
        for r in recv_2 {
            send.send(r).unwrap();
        }
    });
    recv
}

pub fn combine_multiple_receivers(mut recvs: Vec<Receiver<Cmd>>) -> Receiver<Cmd> {
    let n = recvs.len();
    match n.cmp(&2) {
        std::cmp::Ordering::Less => {
            panic!("combine_multiple_receivers needs at least two Receivers")
        }
        std::cmp::Ordering::Equal => combine_receivers(recvs.remove(1), recvs.remove(0)),
        std::cmp::Ordering::Greater => {
            combine_receivers(recvs.remove(n - 1), combine_multiple_receivers(recvs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_combine_recvs() {
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let rec = combine_receivers(r1, r2);
        s1.send(Cmd::IsUp).unwrap();
        s2.send(Cmd::IsUp).unwrap();
        assert_eq!(rec.recv().unwrap(), Cmd::IsUp);
        assert_eq!(rec.recv().unwrap(), Cmd::IsUp);
    }

    #[test]
    fn test_combine_multiple() {
        let (senders, receivers): (Vec<Sender<Cmd>>, Vec<Receiver<Cmd>>) =
            (0..5).map(|n| channel()).unzip();
        let combined = combine_multiple_receivers(receivers);
        for s in senders {
            s.send(Cmd::IsUp).unwrap();
            assert_eq!(combined.recv().unwrap(), Cmd::IsUp);
        }
    }
}
