use serde::Deserialize;

#[derive(Deserialize)]
pub struct GalleryInfo {
    pub title: String,
    pub files: Vec<HitomiFile>,
}

#[derive(Deserialize)]
pub struct HitomiFile {
    pub hash: String,
    #[serde(rename = "haswebp")]
    #[allow(unused)]
    pub haswebp: Option<i32>,
    pub name: String,
    #[serde(rename = "hasavif")]
    #[allow(unused)]
    pub hasavif: Option<i32>,
    #[allow(unused)]
    pub width: Option<i32>,
    #[allow(unused)]
    pub height: Option<i32>,
}
