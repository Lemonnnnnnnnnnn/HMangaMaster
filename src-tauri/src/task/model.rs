use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    #[serde(rename = "pending")] Pending,
    #[serde(rename = "parsing")] Parsing,
    #[serde(rename = "downloading")] Running,
    #[serde(rename = "completed")] Completed,
    #[serde(rename = "failed")] Failed,
    #[serde(rename = "cancelled")] Cancelled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress { pub current: i32, pub total: i32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub url: String,
    pub status: TaskStatus,
    pub save_path: String,
    pub name: String,
    pub error: String,
    pub progress: Progress,
    pub start_time: String,
    pub complete_time: String,
    pub updated_at: String,
}

impl Default for Task {
    fn default() -> Self {
        Self { id: String::new(), url: String::new(), status: TaskStatus::Pending, save_path: String::new(), name: String::new(), error: String::new(), progress: Progress::default(), start_time: String::new(), complete_time: String::new(), updated_at: String::new() }
    }
}


