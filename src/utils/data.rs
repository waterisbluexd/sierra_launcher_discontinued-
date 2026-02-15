use super::item::{ClipboardContent, ClipboardItem};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use std::collections::VecDeque;
use std::sync::RwLock;
use std::fs;
use std::path::PathBuf;

const CACHE_FILE: &str = ".cache/sierra/clipboard.cache";
const MAX_HISTORY: usize = 50;

static CLIPBOARD_HISTORY: RwLock<Option<VecDeque<ClipboardItem>>> = RwLock::new(None);

fn get_cache_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(CACHE_FILE)
}

fn load_from_cache() -> VecDeque<ClipboardItem> {
    let path = get_cache_path();
    if let Ok(content) = fs::read(&path) {
        if let Ok(items) = bincode::deserialize::<VecDeque<ClipboardItem>>(&content) {
            eprintln!("Loaded {} items from clipboard cache", items.len());
            return items;
        }
    }
    VecDeque::new()
}

fn save_to_cache(history: &VecDeque<ClipboardItem>) {
    let path = get_cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(encoded) = bincode::serialize(history) {
        if let Err(e) = fs::write(&path, encoded) {
            eprintln!("Failed to save clipboard cache: {}", e);
        } else {
            eprintln!("Saved {} items to clipboard cache", history.len());
        }
    }
}

pub fn init() {
    let mut history = CLIPBOARD_HISTORY.write().unwrap();
    if history.is_none() {
        *history = Some(load_from_cache());
    }
}

pub fn add_item(content: ClipboardContent) {
    let mut history = CLIPBOARD_HISTORY.write().unwrap();
    let history = history.as_mut().expect("Clipboard history not initialized");

    if let Some(last) = history.front() {
        if is_same_content(&last.content, &content) {
            return;
        }
    }

    let item = ClipboardItem::new(content);
    history.push_front(item);
    
    if history.len() > MAX_HISTORY {
        history.truncate(MAX_HISTORY);
    }
    
    static SAVE_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let count = SAVE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    
    if count % 5 == 0 {
        save_to_cache(history);
    }
}

fn is_same_content(a: &ClipboardContent, b: &ClipboardContent) -> bool {
    match (a, b) {
        (ClipboardContent::Text(a), ClipboardContent::Text(b)) => a == b,
        (ClipboardContent::FilePaths(a), ClipboardContent::FilePaths(b)) => a == b,
        (
            ClipboardContent::RichText {
                plain: p1,
                html: h1,
            },
            ClipboardContent::RichText {
                plain: p2,
                html: h2,
            },
        ) => p1 == p2 && h1 == h2,
        _ => false,
    }
}

pub fn search_items(query: &str) -> Vec<ClipboardItem> {
    let history = CLIPBOARD_HISTORY.read().unwrap();
    let history = history.as_ref().expect("Clipboard history not initialized");

    if query.is_empty() {
        return history.iter().cloned().collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(ClipboardItem, i64)> = history
        .iter()
        .filter_map(|item| {
            let search_text = match &item.content {
                ClipboardContent::Text(text) => text.clone(),
                ClipboardContent::FilePaths(paths) => paths
                    .iter()
                    .filter_map(|p| p.to_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                ClipboardContent::RichText { plain, .. } => plain.clone(),
            };

            matcher
                .fuzzy_match(&search_text, query)
                .map(|score| (item.clone(), score))
        })
        .collect();

    scored.sort_by(|a, b| b.1.cmp(&a.1));
    scored.into_iter().map(|(item, _)| item).collect()
}

pub fn item_count() -> usize {
    let history = CLIPBOARD_HISTORY.read().unwrap();
    history.as_ref().map(|h| h.len()).unwrap_or(0)
}

pub fn delete_item(index: usize) -> bool {
    let mut history = CLIPBOARD_HISTORY.write().unwrap();
    let history = history.as_mut().expect("Clipboard history not initialized");
    
    if index < history.len() {
        history.remove(index);
        save_to_cache(history);
        eprintln!("Deleted clipboard item at index {}, {} items remaining", index, history.len());
        true
    } else {
        false
    }
}

pub fn save_on_shutdown() {
    let history = CLIPBOARD_HISTORY.read().unwrap();
    if let Some(h) = history.as_ref() {
        save_to_cache(h);
    }
}
