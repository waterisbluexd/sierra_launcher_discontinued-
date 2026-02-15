use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy)]
pub enum IpcCommand {
    Show,
    Hide,
    Toggle,
}

impl IpcCommand {
    fn as_bytes(&self) -> &[u8] {
        match self {
            IpcCommand::Show => b"SHOW",
            IpcCommand::Hide => b"HIDE",
            IpcCommand::Toggle => b"TOGGLE",
        }
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let trimmed: Vec<u8> = bytes.iter()
            .copied()
            .filter(|&b| b != b'\n' && b != b'\r' && b != b' ')
            .collect();
        
        match trimmed.as_slice() {
            b"SHOW" => Some(IpcCommand::Show),
            b"HIDE" => Some(IpcCommand::Hide),
            b"TOGGLE" => Some(IpcCommand::Toggle),
            _ => None,
        }
    }
}

static SHOW_PENDING: AtomicBool = AtomicBool::new(false);

pub fn get_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(runtime_dir).join("sierra-launcher.sock")
}

pub fn is_daemon_running() -> bool {
    let socket_path = get_socket_path();
    socket_path.exists() && UnixStream::connect(&socket_path).is_ok()
}

pub fn send_command(cmd: IpcCommand) -> Result<(), Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    let mut stream = UnixStream::connect(&socket_path)?;
    stream.write_all(cmd.as_bytes())?;
    stream.flush()?;
    Ok(())
}

pub fn create_server() -> Result<UnixListener, Box<dyn std::error::Error>> {
    let socket_path = get_socket_path();
    
    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
    }
    
    let listener = UnixListener::bind(&socket_path)?;
    eprintln!("[IPC] Server created at {:?}", socket_path);
    
    Ok(listener)
}

pub fn store_command(cmd: IpcCommand) {
    match cmd {
        IpcCommand::Show => SHOW_PENDING.store(true, Ordering::Relaxed),
        IpcCommand::Hide => SHOW_PENDING.store(false, Ordering::Relaxed),
        IpcCommand::Toggle => {
            let current = SHOW_PENDING.load(Ordering::Relaxed);
            SHOW_PENDING.store(!current, Ordering::Relaxed);
        }
    }
}

pub fn poll_show() -> bool {
    SHOW_PENDING.swap(false, Ordering::Relaxed)
}

pub fn listen_for_commands<F>(listener: UnixListener, mut handler: F) 
where
    F: FnMut(IpcCommand) + Send + 'static,
{
    eprintln!("[IPC] Listening for commands...");
    
    for stream in listener.incoming() {
        eprintln!("[IPC] Incoming connection attempt...");
        match stream {
            Ok(mut stream) => {
                eprintln!("[IPC] Connection accepted");
                let mut buffer = [0u8; 16];
                match stream.read(&mut buffer) {
                    Ok(n) => {
                        eprintln!("[IPC] Read {} bytes: {:?}", n, &buffer[..n]);
                        if let Some(cmd) = IpcCommand::from_bytes(&buffer[..n]) {
                            eprintln!("[IPC] Received command: {:?}", cmd);
                            handler(cmd);
                        } else {
                            eprintln!("[IPC] Unknown command: {:?}", &buffer[..n]);
                        }
                    }
                    Err(e) => {
                        eprintln!("[IPC] Read error: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[IPC] Connection error: {}", e);
            }
        }
    }
    eprintln!("[IPC] Listener loop ended");
}
