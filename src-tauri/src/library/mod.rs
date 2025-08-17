use serde::Serialize;
use base64::Engine as _;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Serialize)]
pub struct Manga {
    pub name: String,
    pub path: String,
    pub preview_img: String,
    pub images_count: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")] 
    pub images: Vec<String>,
}

pub struct Manager {}

impl Default for Manager { fn default() -> Self { Self {} } }

impl Manager {
    pub fn load_library(&self, root: &str) -> anyhow::Result<Vec<Manga>> {
        let mut mangas: Vec<Manga> = vec![];
        let root_path = Path::new(root);
        if !root_path.exists() { return Ok(mangas); }
        for entry in fs::read_dir(root_path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let dir = entry.path();
                let images = self.images_in_dir(&dir)?;
                if images.is_empty() { continue; }
                let mut sorted = images.clone();
                self.sort_images(&mut sorted);
                mangas.push(Manga {
                    name: dir.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    path: dir.to_string_lossy().to_string(),
                    preview_img: sorted[0].clone(),
                    images_count: sorted.len(),
                    images: vec![],
                });
            }
        }
        Ok(mangas)
    }

    pub fn images_in_dir(&self, dir: &Path) -> anyhow::Result<Vec<String>> {
        let mut images = vec![];
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                let path = entry.path();
                if is_image_file(&path) {
                    images.push(path.to_string_lossy().to_string());
                }
            }
        }
        Ok(images)
    }

    pub fn get_manga_images(&self, dir: &str) -> anyhow::Result<Vec<String>> {
        let mut images = self.images_in_dir(Path::new(dir))?;
        self.sort_images(&mut images);
        Ok(images)
    }

    pub fn sort_images(&self, images: &mut Vec<String>) {
        images.sort_by(|a, b| {
            let ai = page_offset_key(a);
            let bi = page_offset_key(b);
            ai.cmp(&bi)
        });
    }

    pub fn get_image_data_url(&self, path: &str) -> anyhow::Result<String> {
        let data = std::fs::read(path)?;
        let mime = mime_guess::from_path(path).first_or_text_plain();
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        Ok(format!("data:{};base64,{}", mime, encoded))
    }

    pub fn delete_manga(&self, path: &str) -> anyhow::Result<bool> {
        if Path::new(path).exists() {
            std::fs::remove_dir_all(path)?;
        }
        Ok(true)
    }
}

fn is_image_file(p: &Path) -> bool {
    match p.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()) {
        Some(ext) => matches!(ext.as_str(), "jpg"|"jpeg"|"png"|"gif"|"webp"|"bmp"),
        None => false,
    }
}

fn page_offset_key(p: &str) -> (i64, i64, String) {
    use std::cmp::Ordering;
    let name = std::path::Path::new(p).file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let stem = name.split('.').next().unwrap_or("");
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() == 2 {
        if let (Ok(a), Ok(b)) = (parts[0].parse::<i64>(), parts[1].parse::<i64>()) {
            return (a, b, name);
        }
    }
    // fallback: first number
    let digits: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    if let Ok(n) = digits.parse::<i64>() { return (n, 0, name); }
    (i64::MAX, i64::MAX, name)
}


