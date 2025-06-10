use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Custom error types for file operations
#[derive(Debug)]
pub enum FileSystemError {
    Io(io::Error),
    Timeout,
    PermissionDenied,
    ConcurrentAccess,
}

impl From<io::Error> for FileSystemError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::PermissionDenied => FileSystemError::PermissionDenied,
            io::ErrorKind::WouldBlock => FileSystemError::ConcurrentAccess,
            _ => FileSystemError::Io(error),
        }
    }
}

/// Handles file system operations for the ZZZ plugin
pub struct FileSystem;

impl FileSystem {
    /// Maximum number of retry attempts for file operations
    const MAX_RETRIES: u32 = 3;
    
    /// Delay between retry attempts
    const RETRY_DELAY: Duration = Duration::from_millis(50);
    
    /// Timeout for file operations
    const OPERATION_TIMEOUT: Duration = Duration::from_secs(5);

    /// Atomically writes content to a file using temporary file + rename pattern
    pub fn write_file_atomic<P: AsRef<Path>>(path: P, content: &str) -> Result<(), FileSystemError> {
        let path = path.as_ref();
        let temp_path = path.with_extension("tmp");
        
        Self::with_retry(|| {
            // Write to temporary file first
            let mut temp_file = fs::File::create(&temp_path)?;
            temp_file.write_all(content.as_bytes())?;
            temp_file.sync_all()?;
            drop(temp_file);
            
            // Atomically rename to final location
            fs::rename(&temp_path, path)?;
            Ok(())
        })
    }

    /// Safely reads file content with retry logic for concurrent access
    pub fn read_file_safe<P: AsRef<Path>>(path: P) -> Result<String, FileSystemError> {
        let path = path.as_ref();
        
        Self::with_retry(|| {
            fs::read_to_string(path).map_err(FileSystemError::from)
        })
    }

    /// Appends content to a file (useful for log files)
    pub fn append_to_file<P: AsRef<Path>>(path: P, content: &str) -> Result<(), FileSystemError> {
        let path = path.as_ref();
        
        Self::with_retry(|| {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
            Ok(())
        })
    }

    /// Checks if a file exists and is readable
    pub fn file_exists<P: AsRef<Path>>(path: P) -> bool {
        let path = path.as_ref();
        path.exists() && path.is_file()
    }

    /// Checks if a file is readable by attempting to read metadata
    pub fn file_is_readable<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().metadata().is_ok()
    }

    /// Creates a file if it doesn't exist
    pub fn ensure_file_exists<P: AsRef<Path>>(path: P) -> Result<(), FileSystemError> {
        let path = path.as_ref();
        if !Self::file_exists(path) {
            Self::write_file_atomic(path, "")?;
        }
        Ok(())
    }

    /// Retry wrapper for file operations with exponential backoff
    fn with_retry<F, T>(mut operation: F) -> Result<T, FileSystemError>
    where
        F: FnMut() -> Result<T, FileSystemError>,
    {
        let start_time = Instant::now();
        let mut attempt = 0;
        
        loop {
            if start_time.elapsed() > Self::OPERATION_TIMEOUT {
                return Err(FileSystemError::Timeout);
            }
            
            match operation() {
                Ok(result) => return Ok(result),
                Err(FileSystemError::ConcurrentAccess) if attempt < Self::MAX_RETRIES => {
                    attempt += 1;
                    std::thread::sleep(Self::RETRY_DELAY * attempt);
                    continue;
                }
                Err(FileSystemError::Io(ref io_err)) 
                    if io_err.kind() == io::ErrorKind::Interrupted && attempt < Self::MAX_RETRIES => {
                    attempt += 1;
                    std::thread::sleep(Self::RETRY_DELAY * attempt);
                    continue;
                }
                Err(err) => return Err(err),
            }
        }
    }
    /// Creates the directory structure for a given task ID
    /// Creates .zzz/task-{task_id}/ directory structure
    pub fn create_task_directory(task_id: u32) -> Result<PathBuf, std::io::Error> {
        let task_dir = Self::get_task_directory_path(task_id);
        fs::create_dir_all(&task_dir)?;
        Ok(task_dir)
    }

    /// Gets the path to the task directory for the given task_id
    pub fn get_task_directory_path(task_id: u32) -> PathBuf {
        PathBuf::from("/host/.zzz").join(format!("task-{}", task_id))
    }

    /// Creates the main .zzz directory if it doesn't exist
    pub fn create_zzz_directory() -> Result<(), std::io::Error> {
        fs::create_dir_all("/host/.zzz")
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

    /// Writes a timestamped log entry to the overseer log
    pub fn log_overseer(task_id: u32, message: &str) -> Result<(), FileSystemError> {
        let log_path = Self::get_overseer_log_path(task_id);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = format!("[{}] {}\n", timestamp, message);
        Self::append_to_file(log_path, &entry)
    }

    /// Writes a timestamped log entry to the commander log
    pub fn log_commander(task_id: u32, message: &str) -> Result<(), FileSystemError> {
        let log_path = Self::get_commander_log_path(task_id);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = format!("[{}] {}\n", timestamp, message);
        Self::append_to_file(log_path, &entry)
    }

    /// Writes a timestamped log entry to the coordinator log
    pub fn log_coordinator(task_id: u32, message: &str) -> Result<(), FileSystemError> {
        let log_path = Self::get_coordinator_log_path(task_id);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = format!("[{}] {}\n", timestamp, message);
        Self::append_to_file(log_path, &entry)
    }

    /// Generic logging function that can write to any log file
    pub fn log_to_file<P: AsRef<Path>>(path: P, message: &str) -> Result<(), FileSystemError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let entry = format!("[{}] {}\n", timestamp, message);
        Self::append_to_file(path, &entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_get_task_directory_path() {
        let path = FileSystem::get_task_directory_path(123);
        assert_eq!(path, PathBuf::from("/host/.zzz/task-123"));
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
        assert_eq!(task_dir_path, PathBuf::from("/host/.zzz/task-456"));

        // Verify parent .zzz directory also exists
        assert!(PathBuf::from("/host/.zzz").exists());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_create_zzz_directory() {
        let temp_dir = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = FileSystem::create_zzz_directory();
        assert!(result.is_ok());
        assert!(PathBuf::from("/host/.zzz").exists());
        assert!(PathBuf::from("/host/.zzz").is_dir());

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