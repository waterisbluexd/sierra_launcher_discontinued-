// Handles all daemon/client socket logic

use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, channel};

fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", unsafe { libc::getuid() }));
    PathBuf::from(runtime_dir).join("sierra-launcher.sock")
}

// Global channel for signaling show events to the main thread
lazy_static::lazy_static! {
    pub static ref SHOW_RECEIVER: Arc<Mutex<Option<std::sync::mpsc::Receiver<()>>>> = Arc::new(Mutex::new(None));
}

/// Called at startup. Returns true if we sent to an existing daemon (client mode).
/// Returns false if no daemon exists — caller should become the daemon.
pub fn try_send_to_daemon() -> bool {
    let path = socket_path();
    if !path.exists() {
        return false;
    }
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            let _ = stream.write_all(b"show");
            true
        }
        Err(_) => {
            // Stale socket — clean it up
            let _ = fs::remove_file(&path);
            false
        }
    }
}

/// Spawn a thread that listens on the socket.
/// Calls `on_show` whenever a client sends "show".
pub fn start_listener<F>(on_show: F)
where
    F: Fn() + Send + 'static,
{
    let path = socket_path();
    // Clean up any stale socket from a previous crash
    let _ = fs::remove_file(&path);

    // Create channel for IPC
    let (tx, rx) = channel();
    
    // Store receiver globally so subscription can poll it
    {
        let mut receiver = SHOW_RECEIVER.lock().unwrap();
        *receiver = Some(rx);
    }

    let listener = UnixListener::bind(&path)
        .expect("Failed to bind sierra-launcher socket");

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut s) => {
                    let mut buf = [0u8; 16];
                    if let Ok(n) = s.read(&mut buf) {
                        if &buf[..n] == b"show" {
                            on_show();
                            let _ = tx.send(());
                        }
                    }
                }
                Err(_) => break,
            }
        }
        // Clean up socket on exit
        let _ = fs::remove_file(&path);
    });
}

/// Check if a show signal was received (non-blocking)
pub fn poll_show() -> bool {
    let receiver = SHOW_RECEIVER.lock().unwrap();
    if let Some(ref rx) = *receiver {
        rx.try_recv().is_ok()
    } else {
        false
    }
}

/// Remove socket on clean shutdown
pub fn cleanup() {
    let _ = fs::remove_file(socket_path());
}
