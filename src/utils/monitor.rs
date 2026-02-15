use super::data;
use super::item::ClipboardContent;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use tracing::{debug, info};
use wl_clipboard_rs::paste::{get_contents, ClipboardType, Seat, MimeType};

lazy_static::lazy_static! {
    static ref IGNORE_TEXT: Mutex<Option<String>> = Mutex::new(None);
}

pub fn set_ignore_next(text: String) {
    let mut ignore = IGNORE_TEXT.lock().unwrap();
    *ignore = Some(text);
    debug!("Set ignore text: {} chars", ignore.as_ref().unwrap().len());
}

#[allow(dead_code)]
pub fn clear_ignore() {
    let mut ignore = IGNORE_TEXT.lock().unwrap();
    *ignore = None;
    debug!("Cleared ignore text");
}

fn should_ignore(text: &str) -> bool {
    let mut ignore = IGNORE_TEXT.lock().unwrap();
    if let Some(ignore_text) = ignore.as_ref() {
        if ignore_text == text {
            debug!("Ignoring clipboard text (matches ignore list)");
            *ignore = None;
            return true;
        }
    }
    false
}

pub fn start_monitor() -> Arc<AtomicBool> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();

    thread::spawn(move || {
        info!("Starting clipboard monitor (wl-clipboard-rs polling mode)");
        
        let mut last_text = String::new();
        
        loop {
            if !running_clone.load(Ordering::Relaxed) {
                info!("Clipboard monitor stopped");
                break;
            }
            
            thread::sleep(Duration::from_millis(500));
            
            match get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text) {
                Ok((mut pipe, _mime_type)) => {
                    use std::io::Read;
                    let mut contents = Vec::new();
                    
                    if let Ok(_) = pipe.read_to_end(&mut contents) {
                        if let Ok(text) = String::from_utf8(contents) {
                            let text = text.trim().to_string();
                            
                            if !text.is_empty() && text != last_text {
                                if should_ignore(&text) {
                                    last_text = text;
                                    continue;
                                }
                                
                                debug!("New clipboard content detected: {} chars", text.len());
                                data::add_item(ClipboardContent::Text(text.clone()));
                                last_text = text;
                            }
                        }
                    }
                }
                Err(e) => {
                    if running_clone.load(Ordering::Relaxed) {
                        debug!("Clipboard read failed (may be normal): {:?}", e);
                    }
                }
            }
        }
    });

    running
}
