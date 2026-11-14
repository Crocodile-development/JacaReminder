// Makes sure only one daemon and one dashboard are open at a time.
//
// The first program listens on a port of this computer (only reachable from inside it). A program
// that starts later finds the port taken. For the dashboard it sends a line to the first one,
// which brings its window to the front.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::Duration;

const DAEMON_PORT: u16 = 47_841;
const DASHBOARD_PORT: u16 = 47_842;
const SHOW_MESSAGE: &[u8] = b"JACA-REMINDER SHOW\n";

fn address(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// Claims the daemon port. `None` means a daemon is already running.
pub fn claim_daemon() -> Option<TcpListener> {
    TcpListener::bind(address(DAEMON_PORT)).ok()
}

/// True when a daemon is running (the port is taken).
pub fn daemon_running() -> bool {
    TcpListener::bind(address(DAEMON_PORT)).is_err()
}

pub enum Dashboard {
    /// This is the first dashboard. The receiver gets a message each time another one is asked.
    First(Receiver<()>),
    /// Another dashboard exists and was asked to come to the front.
    Forwarded,
    /// The port is used by something else: open the window anyway.
    Alone,
}

pub fn claim_dashboard(on_request: impl Fn() + Send + 'static) -> Dashboard {
    match TcpListener::bind(address(DASHBOARD_PORT)) {
        Ok(listener) => {
            let (sender, receiver) = channel();
            thread::spawn(move || {
                for stream in listener.incoming().flatten() {
                    let mut stream = stream;
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let mut buffer = [0u8; 64];
                    if let Ok(read) = stream.read(&mut buffer) {
                        if &buffer[..read] == SHOW_MESSAGE && sender.send(()).is_ok() {
                            on_request();
                        }
                    }
                }
            });
            Dashboard::First(receiver)
        }
        Err(_) => {
            let sent = TcpStream::connect_timeout(&address(DASHBOARD_PORT), Duration::from_secs(1))
                .and_then(|mut stream| stream.write_all(SHOW_MESSAGE))
                .is_ok();
            if sent { Dashboard::Forwarded } else { Dashboard::Alone }
        }
    }
}
