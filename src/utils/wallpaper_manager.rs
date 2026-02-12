use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperCache {
    pub last_wallpaper: PathBuf,
}

#[derive(Debug)]
pub struct WallpaperManager {
    wallpaper_dir: PathBuf,
    cache_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperIndex {
    pub wallpaper_dir: PathBuf,
    pub generated_at: u64,
    pub wallpapers: Vec<WallpaperEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WallpaperEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: WallpaperKind,
    pub thumbnail: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WallpaperKind {
    Image,
    Video,
}

impl WallpaperManager {
    pub fn new(wallpaper_dir: PathBuf) -> Self {
        let cache_dir = Self::default_cache_dir();
        Self {
            wallpaper_dir,
            cache_dir,
        }
    }

    /// ✅ FIXED: Restore last wallpaper WITHOUT killing existing gslapper (prevents flicker)
    pub fn restore_last_wallpaper(&self) {
        if let Some(last_wallpaper_path) = self.get_last_wallpaper() {
            if !last_wallpaper_path.exists() {
                eprintln!("[Wallpaper] Last wallpaper no longer exists: {:?}", last_wallpaper_path);
                return;
            }

            // Check if gslapper is already running with this wallpaper
            if let Ok(output) = Command::new("pgrep").arg("-a").arg("gslapper").output() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    let wallpaper_str = last_wallpaper_path.to_string_lossy();
                    if stdout.contains(wallpaper_str.as_ref()) {
                        eprintln!("[Wallpaper] ✓ Already running: {:?}", last_wallpaper_path);
                        return; // Already running with correct wallpaper, don't restart
                    }
                }
            }

            // Determine wallpaper type
            let ext = last_wallpaper_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let kind = match ext.as_str() {
                "mp4" | "mkv" | "webm" | "avi" => WallpaperKind::Video,
                "jpg" | "jpeg" | "png" | "webp" | "bmp" => WallpaperKind::Image,
                _ => {
                    eprintln!("[Wallpaper] Unknown file type: {:?}", ext);
                    return;
                }
            };

            // Create entry manually
            let entry = WallpaperEntry {
                name: last_wallpaper_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                path: last_wallpaper_path.clone(),
                kind,
                thumbnail: PathBuf::new(),
            };

            eprintln!("[Wallpaper] ✓ Restoring last wallpaper: {:?}", entry.name);
            
            // FIX: Use set_wallpaper_gentle for restoration (doesn't kill existing)
            self.set_wallpaper_gentle(&entry);
        } else {
            eprintln!("[Wallpaper] No last wallpaper found in cache");
        }
    }

    /// ✅ NEW: Gentle wallpaper setting (checks if already running first)
    fn set_wallpaper_gentle(&self, entry: &WallpaperEntry) {
        let wallpaper_path = entry.path.to_string_lossy();

        // Check if already running
        if let Ok(output) = Command::new("pgrep").arg("-a").arg("gslapper").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if stdout.contains(wallpaper_path.as_ref()) {
                    eprintln!("[Wallpaper] Already running, skipping");
                    return;
                }
            }
        }

        // Kill existing gslapper only if different wallpaper
        let _ = Command::new("pkill").arg("-9").arg("gslapper").output();
        std::thread::sleep(std::time::Duration::from_millis(50));

        let gslapper_args = match entry.kind {
            WallpaperKind::Video => vec!["-o", "loop no-audio", "*", &wallpaper_path],
            WallpaperKind::Image => vec!["-o", "fill", "*", &wallpaper_path],
        };

        let _ = Command::new("gslapper").args(&gslapper_args).spawn();
        eprintln!("[Wallpaper] Set to: {:?}", entry.name);
        let wal_path = match entry.kind {
            WallpaperKind::Image => entry.path.clone(),
            WallpaperKind::Video => entry.thumbnail.clone(),
        };

        if wal_path.exists() {
            let wp = wal_path.clone();
            eprintln!("[Pywal] Queued color update from: {:?}", wp);
            // Run pywal in background to avoid blocking UI
            std::thread::spawn(move || {
                let _ = Command::new("wal")
                    .arg("-i")
                    .arg(&wp)
                    .arg("-n")
                    .output();
            });
        } else {
            eprintln!("[Pywal] Could not find path for wal: {:?}", wal_path);
        }
    }

    /// OPTIMIZED: Only generate cache if it doesn't exist or is invalid
    pub fn ensure_cache(&self) {
        if self.load_index().is_some() {
            eprintln!("[Wallpaper] Valid cache exists - skipping generation");
            return;
        }

        eprintln!("[Wallpaper] Generating fresh cache...");
        
        if fs::create_dir_all(&self.cache_dir).is_err() {
            eprintln!("[Wallpaper] Failed to create cache dir");
            return;
        }

        let thumbs_dir = self.cache_dir.join("thumbs");
        let _ = fs::create_dir_all(&thumbs_dir);

        let mut wallpapers = Vec::new();

        let entries = match fs::read_dir(&self.wallpaper_dir) {
            Ok(e) => e,
            Err(_) => {
                eprintln!("[Wallpaper] Cannot read wallpaper dir");
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let name = match path.file_name() {
                Some(n) => n.to_string_lossy().to_string(),
                None => continue,
            };

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let (kind, needs_ffmpeg) = match ext.as_str() {
                "mp4" | "mkv" | "webm" | "avi" => (WallpaperKind::Video, true),
                "jpg" | "jpeg" | "png" | "webp" | "bmp" => (WallpaperKind::Image, false),
                _ => continue,
            };

            let thumb_path = thumbs_dir.join(format!("{}.jpg", name));

            if !thumb_path.exists() {
                if needs_ffmpeg {
                    Self::generate_video_thumbnail(&path, &thumb_path);
                } else {
                    Self::generate_image_thumbnail(&path, &thumb_path);
                }
            }

            wallpapers.push(WallpaperEntry {
                name,
                path,
                kind,
                thumbnail: thumb_path,
            });
        }

        eprintln!("[Wallpaper] Processed {} wallpapers", wallpapers.len());

        let generated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let index = WallpaperIndex {
            wallpaper_dir: self.wallpaper_dir.clone(),
            generated_at,
            wallpapers,
        };

        let index_path = self.cache_dir.join("index.json");

        if let Ok(json) = serde_json::to_string_pretty(&index) {
            let _ = fs::write(&index_path, json);
            eprintln!("[Wallpaper] Cache saved to {:?}", index_path);
        }
    }

    pub fn load_index(&self) -> Option<WallpaperIndex> {
        let index_path = self.cache_dir.join("index.json");
        let content = fs::read_to_string(&index_path).ok()?;
        let index: WallpaperIndex = serde_json::from_str(&content).ok()?;

        if !self.is_cache_valid(&index) {
            eprintln!("[Wallpaper] Cache outdated - wallpapers changed");
            return None;
        }

        Some(index)
    }

    fn is_cache_valid(&self, index: &WallpaperIndex) -> bool {
        let Ok(entries) = fs::read_dir(&self.wallpaper_dir) else {
            return false;
        };

        let current_count = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter(|e| {
                if let Some(ext) = e.path().extension().and_then(|e| e.to_str()) {
                    matches!(
                        ext.to_lowercase().as_str(),
                        "mp4" | "mkv" | "webm" | "avi" | "jpg" | "jpeg" | "png" | "webp" | "bmp"
                    )
                } else {
                    false
                }
            })
            .count();

        current_count == index.wallpapers.len()
    }

    pub fn set_wallpaper(&self, entry: &WallpaperEntry) {
        // ALWAYS kill when explicitly setting a new wallpaper
        let _ = Command::new("pkill").arg("-9").arg("gslapper").output();
        std::thread::sleep(std::time::Duration::from_millis(100));

        let wallpaper_path = entry.path.to_string_lossy();

        let gslapper_args = match entry.kind {
            WallpaperKind::Video => vec!["-o", "loop no-audio", "*", &wallpaper_path],
            WallpaperKind::Image => vec!["-o", "fill", "*", &wallpaper_path],
        };

        let _ = Command::new("gslapper").args(&gslapper_args).spawn();
        eprintln!("[Wallpaper] Set to: {:?}", entry.name);
        let wal_path = match entry.kind {
            WallpaperKind::Image => entry.path.clone(),
            WallpaperKind::Video => entry.thumbnail.clone(),
        };

        if wal_path.exists() {
            let wp = wal_path.clone();
            eprintln!("[Pywal] Queued color update from: {:?}", wp);
            // Run pywal in background to avoid blocking UI
            std::thread::spawn(move || {
                let _ = Command::new("wal")
                    .arg("-i")
                    .arg(&wp)
                    .arg("-n")
                    .output();
            });
        } else {
            eprintln!("[Pywal] Could not find path for wal: {:?}", wal_path);
        }
    }

    fn generate_image_thumbnail(source: &PathBuf, thumbnail: &PathBuf) {
        use image::ImageReader;
        
        let img = match ImageReader::open(source) {
            Ok(reader) => match reader.decode() {
                Ok(img) => img,
                Err(e) => {
                    eprintln!("[Wallpaper] Failed to decode {:?}: {}", source, e);
                    return;
                }
            },
            Err(e) => {
                eprintln!("[Wallpaper] Failed to open {:?}: {}", source, e);
                return;
            }
        };
        
        let thumb = img.resize(480, 270, image::imageops::FilterType::Triangle);
        if let Err(e) = thumb.save_with_format(thumbnail, image::ImageFormat::Jpeg) {
            eprintln!("[Wallpaper] Failed to save thumbnail {:?}: {}", thumbnail, e);
        }
    }

    fn generate_video_thumbnail(video: &PathBuf, thumbnail: &PathBuf) {
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "quiet",
                "-i",
                video.to_str().unwrap(),
                "-vf",
                "scale=480:270",
                "-frames:v",
                "1",
                "-q:v",
                "5",
                thumbnail.to_str().unwrap(),
            ])
            .status();

        match status {
            Ok(s) if s.success() => {},
            _ => eprintln!("[Wallpaper] Failed to generate video thumbnail for {:?}", video),
        }
    }

    fn default_cache_dir() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".cache")
                .join("sierra")
                .join("wallpapers")
        } else {
            PathBuf::from(".cache/sierra/wallpapers")
        }
    }

    pub fn get_last_wallpaper(&self) -> Option<PathBuf> {
        let cache_path = self.cache_dir.join("last_wallpaper.json");
        if let Ok(content) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str::<WallpaperCache>(&content) {
                return Some(cache.last_wallpaper);
            }
        }
        None
    }

    pub fn set_last_wallpaper(&self, path: &PathBuf) {
        let cache_path = self.cache_dir.join("last_wallpaper.json");
        let cache = WallpaperCache {
            last_wallpaper: path.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&cache) {
            let _ = fs::write(&cache_path, json);
        }
    }
}