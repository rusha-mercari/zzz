use std::fs;
use std::path::PathBuf;

/// Handles file system operations for the ZZZ plugin
pub struct FileSystem;

impl FileSystem {
    /// Creates the directory structure for a given task ID
    /// Creates .zzz/task-{task_id}/ directory structure
    pub fn create_task_directory(task_id: u32) -> Result<PathBuf, std::io::Error> {
        let task_dir = Self::get_task_directory_path(task_id);
        fs::create_dir_all(&task_dir)?;
        Ok(task_dir)
    }

    /// Gets the path to the task directory for the given task_id
    pub fn get_task_directory_path(task_id: u32) -> PathBuf {
        PathBuf::from(".zzz").join(format!("task-{}", task_id))
    }

    /// Creates the main .zzz directory if it doesn't exist
    pub fn create_zzz_directory() -> Result<(), std::io::Error> {
        fs::create_dir_all(".zzz")
    }

    /// Sets up the complete directory structure for the given task
    pub fn setup_task_directories(task_id: u32) -> Result<PathBuf, std::io::Error> {
        // First ensure .zzz directory exists
        Self::create_zzz_directory()?;
        
        // Then create the specific task directory
        let task_dir = Self::create_task_directory(task_id)?;
        
        // Create logs subdirectory
        let logs_dir = Self::get_logs_dir_path(task_id);
        fs::create_dir_all(&logs_dir)?;
        
        Ok(task_dir)
    }

    /// Gets the path to the todo-list.md file for the given task_id
    pub fn get_todo_list_path(task_id: u32) -> PathBuf {
        Self::get_task_directory_path(task_id).join("todo-list.md")
    }

    /// Gets the path to the review.md file for the given task_id
    pub fn get_review_path(task_id: u32) -> PathBuf {
        Self::get_task_directory_path(task_id).join("review.md")
    }

    /// Gets the path to the plan.md file for the given task_id
    pub fn get_plan_path(task_id: u32) -> PathBuf {
        Self::get_task_directory_path(task_id).join("plan.md")
    }

    /// Gets the path to the logs directory for the given task_id
    pub fn get_logs_dir_path(task_id: u32) -> PathBuf {
        Self::get_task_directory_path(task_id).join("logs")
    }

    /// Gets the path to the overseer.log file for the given task_id
    pub fn get_overseer_log_path(task_id: u32) -> PathBuf {
        Self::get_logs_dir_path(task_id).join("overseer.log")
    }

    /// Gets the path to the commander.log file for the given task_id
    pub fn get_commander_log_path(task_id: u32) -> PathBuf {
        Self::get_logs_dir_path(task_id).join("commander.log")
    }

    /// Gets the path to the coordinator.log file for the given task_id
    pub fn get_coordinator_log_path(task_id: u32) -> PathBuf {
        Self::get_logs_dir_path(task_id).join("coordinator.log")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_task_directory_path() {
        let path = FileSystem::get_task_directory_path(123);
        assert_eq!(path, PathBuf::from(".zzz/task-123"));
    }

    #[test]
    fn test_create_task_directory() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = FileSystem::setup_task_directories(456);
        assert!(result.is_ok());

        let task_dir_path = result.unwrap();
        assert!(task_dir_path.exists());
        assert!(task_dir_path.is_dir());
        assert_eq!(task_dir_path, PathBuf::from(".zzz/task-456"));

        // Verify parent .zzz directory also exists
        assert!(PathBuf::from(".zzz").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_zzz_directory() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = FileSystem::create_zzz_directory();
        assert!(result.is_ok());
        assert!(PathBuf::from(".zzz").exists());
        assert!(PathBuf::from(".zzz").is_dir());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_directory_creation_idempotent() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create directory first time
        let result1 = FileSystem::setup_task_directories(789);
        assert!(result1.is_ok());

        // Create directory second time - should not fail
        let result2 = FileSystem::setup_task_directories(789);
        assert!(result2.is_ok());

        let task_dir_path = result2.unwrap();
        assert!(task_dir_path.exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

}