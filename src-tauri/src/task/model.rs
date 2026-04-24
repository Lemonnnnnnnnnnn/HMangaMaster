use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    #[serde(rename = "pending")] Pending,
    #[serde(rename = "parsing")] Parsing,
    #[serde(rename = "queued")] Queued,
    #[serde(rename = "downloading")] Running,
    #[serde(rename = "completed")] Completed,
    #[serde(rename = "partial_failed")] PartialFailed,
    #[serde(rename = "failed")] Failed,
    #[serde(rename = "cancelled")] Cancelled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress { pub current: i32, pub total: i32 }

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FailedFile {
    pub index: usize,
    pub url: String,
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusInfo {
    pub running_tasks: usize,
    pub queued_tasks: usize,
    pub max_concurrent_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub url: String,
    pub status: TaskStatus,
    pub save_path: String,
    pub name: String,
    pub error: String,
    pub failed_count: i32,
    #[serde(default)]
    pub failed_files: Vec<FailedFile>,
    pub progress: Progress,
    pub start_time: String,
    pub complete_time: String,
    pub updated_at: String,
    pub last_retry_time: String,
    pub retryable: bool,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            id: String::new(),
            url: String::new(),
            status: TaskStatus::Pending,
            save_path: String::new(),
            name: String::new(),
            error: String::new(),
            failed_count: 0,
            failed_files: Vec::new(),
            progress: Progress::default(),
            start_time: String::new(),
            complete_time: String::new(),
            updated_at: String::new(),
            last_retry_time: String::new(),
            retryable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_file_serializes_with_camel_case_fields() {
        let failed_file = FailedFile {
            index: 3,
            url: "https://example.test/3.jpg".to_string(),
            path: "D:/manga/0003.jpg".to_string(),
            error: "bad status: 500".to_string(),
        };

        let json = serde_json::to_value(failed_file).unwrap();

        assert_eq!(json["index"], 3);
        assert_eq!(json["url"], "https://example.test/3.jpg");
        assert_eq!(json["path"], "D:/manga/0003.jpg");
        assert_eq!(json["error"], "bad status: 500");
    }

    #[test]
    fn default_task_has_no_failed_files() {
        let task = Task::default();

        assert!(task.failed_files.is_empty());
    }
}


