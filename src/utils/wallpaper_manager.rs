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

    /// Main entry point:
    /// - creates cache dirs
    /// - generates thumbnails for ALL wallpapers (images + videos)
    /// - writes index.json
    pub fn ensure_cache(&self) {
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

        eprintln!("[Wallpaper] Scanning wallpapers...");

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
                _ => continue, // skip unknown formats
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
            eprintln!("[Wallpaper] Index saved to {:?}", index_path);
        }
    }

    /// Load index.json from cache, or None if cache is invalid/outdated
    pub fn load_index(&self) -> Option<WallpaperIndex> {
        let index_path = self.cache_dir.join("index.json");
        let content = fs::read_to_string(&index_path).ok()?;
        let index: WallpaperIndex = serde_json::from_str(&content).ok()?;

        // Validate cache: check if wallpapers still exist and no new ones added
        if !self.is_cache_valid(&index) {
            eprintln!("[Wallpaper] Cache outdated - wallpapers changed");
            return None;
        }

        Some(index)
    }

    /// Check if cached index matches current wallpaper directory
    fn is_cache_valid(&self, index: &WallpaperIndex) -> bool {
        // Get current wallpaper files
        let Ok(entries) = fs::read_dir(&self.wallpaper_dir) else {
            return false;
        };

        let mut current_files: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| {
                let path = e.path();
                let ext = path.extension()?.to_str()?.to_lowercase();
                match ext.as_str() {
                    "mp4" | "mkv" | "webm" | "avi" | "jpg" | "jpeg" | "png" | "webp" | "bmp" => {
                        Some(path.file_name()?.to_string_lossy().to_string())
                    }
                    _ => None,
                }
            })
            .collect();

        let mut cached_files: Vec<String> = index
            .wallpapers
            .iter()
            .map(|w| w.name.clone())
            .collect();

        current_files.sort();
        cached_files.sort();

        // If file lists don't match, cache is invalid
        current_files == cached_files
    }

    /// Set wallpaper using gSlapper and update pywal colors
    pub fn set_wallpaper(&self, entry: &WallpaperEntry) {
        // KILL ALL EXISTING gSlapper INSTANCES FIRST
        let _ = Command::new("pkill")
            .arg("gslapper")
            .output();
        
        // Wait a moment for processes to die
        std::thread::sleep(std::time::Duration::from_millis(100));

        let wallpaper_path = entry.path.to_string_lossy();
        let thumbnail_path = entry.thumbnail.to_string_lossy();

        // Set wallpaper with gSlapper
        let gslapper_args = match entry.kind {
            WallpaperKind::Video => vec!["-o", "loop no-audio", "*", &wallpaper_path],
            WallpaperKind::Image => vec!["-o", "fill", "*", &wallpaper_path],
        };

        let _ = Command::new("gslapper")
            .args(&gslapper_args)
            .spawn();

        // Update pywal colors from thumbnail (faster than full image)
        let _ = Command::new("wal")
            .args(&["-i", &thumbnail_path, "-n"])
            .spawn();

        eprintln!("[Wallpaper] Set to: {:?}", entry.name);
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
        let thumb = img.resize(480, 270, image::imageops::FilterType::Lanczos3);
        if let Err(e) = thumb.save_with_format(thumbnail, image::ImageFormat::Jpeg) {
            eprintln!("[Wallpaper] Failed to save thumbnail {:?}: {}", thumbnail, e);
        } else {
            eprintln!("[Wallpaper] ✓ Generated thumbnail: {:?}", thumbnail);
        }
    }

    /// Extract first frame of video using ffmpeg
    fn generate_video_thumbnail(video: &PathBuf, thumbnail: &PathBuf) {
        eprintln!("[Wallpaper] Generating video thumbnail: {:?}", video);
        
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-loglevel",
                "error",
                "-i",
                video.to_str().unwrap(),
                "-vf",
                "scale=480:270", 
                "-frames:v",
                "1",
                "-q:v",
                "5", // JPEG quality
                thumbnail.to_str().unwrap(),
            ])
            .status();

        match status {
            Ok(s) if s.success() => eprintln!("[Wallpaper] ✓ Generated video thumbnail: {:?}", thumbnail),
            _ => eprintln!("[Wallpaper] Failed to generate video thumbnail for {:?}", video),
        }
    }

    /// ~/.cache/sierra/wallpapers
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

    /// Read last wallpaper from cache
    pub fn get_last_wallpaper(&self) -> Option<PathBuf> {
        let cache_path = self.cache_dir.join("last_wallpaper.json");
        if let Ok(content) = fs::read_to_string(&cache_path) {
            if let Ok(cache) = serde_json::from_str::<WallpaperCache>(&content) {
                return Some(cache.last_wallpaper);
            }
        }
        None
    }

    /// Save last wallpaper to cache
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