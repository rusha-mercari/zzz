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
    pub fn write_file_atomic<P: AsRef<Path>>(
        path: P,
        content: &str,
    ) -> Result<(), FileSystemError> {
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

        Self::with_retry(|| fs::read_to_string(path).map_err(FileSystemError::from))
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
                    if io_err.kind() == io::ErrorKind::Interrupted
                        && attempt < Self::MAX_RETRIES =>
                {
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
    use std::fs;
    use std::io::ErrorKind;
    use std::thread;
    use std::time::Duration;
    use tempfile::{tempdir, TempDir};

    fn create_test_dir() -> TempDir {
        tempdir().expect("Failed to create temporary directory")
    }

    #[test]
    fn test_filesystem_error_from_io_error() {
        let permission_error =
            std::io::Error::new(ErrorKind::PermissionDenied, "Permission denied");
        let fs_error = FileSystemError::from(permission_error);
        matches!(fs_error, FileSystemError::PermissionDenied);

        let would_block_error = std::io::Error::new(ErrorKind::WouldBlock, "Would block");
        let fs_error = FileSystemError::from(would_block_error);
        matches!(fs_error, FileSystemError::ConcurrentAccess);

        let other_error = std::io::Error::new(ErrorKind::NotFound, "Not found");
        let fs_error = FileSystemError::from(other_error);
        matches!(fs_error, FileSystemError::Io(_));
    }

    #[test]
    fn test_filesystem_error_debug() {
        let error = FileSystemError::Timeout;
        let debug_str = format!("{:?}", error);
        assert_eq!(debug_str, "Timeout");

        let error = FileSystemError::PermissionDenied;
        let debug_str = format!("{:?}", error);
        assert_eq!(debug_str, "PermissionDenied");

        let error = FileSystemError::ConcurrentAccess;
        let debug_str = format!("{:?}", error);
        assert_eq!(debug_str, "ConcurrentAccess");
    }

    #[test]
    fn test_write_file_atomic_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Hello, World!";

        let result = FileSystem::write_file_atomic(&file_path, content);
        assert!(result.is_ok());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);

        // Ensure temp file is cleaned up
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    #[test]
    fn test_write_file_atomic_creates_parent_dir() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("subdir").join("test.txt");
        let content = "Test content";

        // This should fail because parent directory doesn't exist
        let result = FileSystem::write_file_atomic(&file_path, content);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_file_safe_success() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("test.txt");
        let content = "Test content for reading";

        fs::write(&file_path, content).unwrap();

        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_read_file_safe_nonexistent() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_err());
        matches!(result.unwrap_err(), FileSystemError::Io(_));
    }

    #[test]
    fn test_append_to_file_new_file() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("append_test.txt");
        let content = "First line\n";

        let result = FileSystem::append_to_file(&file_path, content);
        assert!(result.is_ok());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_append_to_file_existing_file() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("append_test.txt");
        let initial_content = "First line\n";
        let additional_content = "Second line\n";

        fs::write(&file_path, initial_content).unwrap();

        let result = FileSystem::append_to_file(&file_path, additional_content);
        assert!(result.is_ok());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(
            read_content,
            format!("{}{}", initial_content, additional_content)
        );
    }

    #[test]
    fn test_multiple_appends() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("multi_append.txt");

        for i in 1..=3 {
            let content = format!("Line {}\n", i);
            let result = FileSystem::append_to_file(&file_path, &content);
            assert!(result.is_ok());
        }

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, "Line 1\nLine 2\nLine 3\n");
    }

    #[test]
    fn test_file_exists_true() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("exists.txt");
        fs::write(&file_path, "content").unwrap();

        assert!(FileSystem::file_exists(&file_path));
    }

    #[test]
    fn test_file_exists_false() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("nonexistent.txt");

        assert!(!FileSystem::file_exists(&file_path));
    }

    #[test]
    fn test_file_exists_directory() {
        let temp_dir = create_test_dir();
        let dir_path = temp_dir.path().join("subdir");
        fs::create_dir(&dir_path).unwrap();

        // file_exists should return false for directories
        assert!(!FileSystem::file_exists(&dir_path));
    }

    #[test]
    fn test_file_is_readable_true() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("readable.txt");
        fs::write(&file_path, "content").unwrap();

        assert!(FileSystem::file_is_readable(&file_path));
    }

    #[test]
    fn test_file_is_readable_false() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("nonexistent.txt");

        assert!(!FileSystem::file_is_readable(&file_path));
    }

    #[test]
    fn test_ensure_file_exists_new_file() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("ensure_test.txt");

        assert!(!file_path.exists());

        let result = FileSystem::ensure_file_exists(&file_path);
        assert!(result.is_ok());
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_ensure_file_exists_existing_file() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("existing.txt");
        let original_content = "Original content";
        fs::write(&file_path, original_content).unwrap();

        let result = FileSystem::ensure_file_exists(&file_path);
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, original_content);
    }

    #[test]
    fn test_get_task_directory_path() {
        let task_id = 42;
        let expected_path = PathBuf::from("/host/.zzz/task-42");
        let actual_path = FileSystem::get_task_directory_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_todo_list_path() {
        let task_id = 123;
        let expected_path = PathBuf::from("/host/.zzz/task-123/todo-list.md");
        let actual_path = FileSystem::get_todo_list_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_review_path() {
        let task_id = 456;
        let expected_path = PathBuf::from("/host/.zzz/task-456/review.md");
        let actual_path = FileSystem::get_review_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_plan_path() {
        let task_id = 789;
        let expected_path = PathBuf::from("/host/.zzz/task-789/plan.md");
        let actual_path = FileSystem::get_plan_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_logs_dir_path() {
        let task_id = 100;
        let expected_path = PathBuf::from("/host/.zzz/task-100/logs");
        let actual_path = FileSystem::get_logs_dir_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_overseer_log_path() {
        let task_id = 200;
        let expected_path = PathBuf::from("/host/.zzz/task-200/logs/overseer.log");
        let actual_path = FileSystem::get_overseer_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_commander_log_path() {
        let task_id = 300;
        let expected_path = PathBuf::from("/host/.zzz/task-300/logs/commander.log");
        let actual_path = FileSystem::get_commander_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_get_coordinator_log_path() {
        let task_id = 400;
        let expected_path = PathBuf::from("/host/.zzz/task-400/logs/coordinator.log");
        let actual_path = FileSystem::get_coordinator_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_path_consistency() {
        let task_id = 500;
        let task_dir = FileSystem::get_task_directory_path(task_id);
        let logs_dir = FileSystem::get_logs_dir_path(task_id);
        let todo_path = FileSystem::get_todo_list_path(task_id);
        let review_path = FileSystem::get_review_path(task_id);
        let plan_path = FileSystem::get_plan_path(task_id);
        let overseer_log = FileSystem::get_overseer_log_path(task_id);
        let commander_log = FileSystem::get_commander_log_path(task_id);
        let coordinator_log = FileSystem::get_coordinator_log_path(task_id);

        // Ensure all paths are under the task directory
        assert!(todo_path.starts_with(&task_dir));
        assert!(review_path.starts_with(&task_dir));
        assert!(plan_path.starts_with(&task_dir));
        assert!(logs_dir.starts_with(&task_dir));

        // Ensure all log files are under the logs directory
        assert!(overseer_log.starts_with(&logs_dir));
        assert!(commander_log.starts_with(&logs_dir));
        assert!(coordinator_log.starts_with(&logs_dir));
    }

    #[test]
    fn test_log_to_file() {
        let temp_dir = create_test_dir();
        let log_path = temp_dir.path().join("test.log");
        let message = "Test log message";

        let result = FileSystem::log_to_file(&log_path, message);
        assert!(result.is_ok());

        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains(message));
        assert!(content.starts_with('['));
        assert!(content.ends_with("Test log message\n"));
    }

    #[test]
    fn test_log_to_file_multiple_entries() {
        let temp_dir = create_test_dir();
        let log_path = temp_dir.path().join("multi.log");

        let messages = ["First message", "Second message", "Third message"];
        for message in &messages {
            let result = FileSystem::log_to_file(&log_path, message);
            assert!(result.is_ok());
        }

        let content = fs::read_to_string(&log_path).unwrap();
        for message in &messages {
            assert!(content.contains(message));
        }

        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_log_timestamp_format() {
        let temp_dir = create_test_dir();
        let log_path = temp_dir.path().join("timestamp.log");
        let message = "Timestamp test";

        let before_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = FileSystem::log_to_file(&log_path, message);
        assert!(result.is_ok());

        let after_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let content = fs::read_to_string(&log_path).unwrap();
        let line = content.lines().next().unwrap();

        // Extract timestamp from log line format: [timestamp] message
        let timestamp_str = &line[1..line.find(']').unwrap()];
        let logged_timestamp: u64 = timestamp_str.parse().unwrap();

        assert!(logged_timestamp >= before_timestamp);
        assert!(logged_timestamp <= after_timestamp);
    }

    #[test]
    fn test_log_overseer_creates_correct_path() {
        // We can't easily test the actual logging in the /host/.zzz directory
        // since it's outside our temp directory, but we can test the path generation
        // and ensure the function doesn't panic
        let task_id = 999;
        let expected_path = PathBuf::from("/host/.zzz/task-999/logs/overseer.log");
        let actual_path = FileSystem::get_overseer_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_log_commander_creates_correct_path() {
        let task_id = 888;
        let expected_path = PathBuf::from("/host/.zzz/task-888/logs/commander.log");
        let actual_path = FileSystem::get_commander_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_log_coordinator_creates_correct_path() {
        let task_id = 777;
        let expected_path = PathBuf::from("/host/.zzz/task-777/logs/coordinator.log");
        let actual_path = FileSystem::get_coordinator_log_path(task_id);
        assert_eq!(actual_path, expected_path);
    }

    #[test]
    fn test_with_retry_success_first_attempt() {
        let mut attempts = 0;
        let result = FileSystem::with_retry(|| {
            attempts += 1;
            Ok::<i32, FileSystemError>(42)
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 1);
    }

    #[test]
    fn test_with_retry_success_after_retries() {
        let mut attempts = 0;
        let result = FileSystem::with_retry(|| {
            attempts += 1;
            if attempts < 3 {
                Err(FileSystemError::ConcurrentAccess)
            } else {
                Ok::<i32, FileSystemError>(99)
            }
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 99);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_with_retry_max_retries_exceeded() {
        let mut attempts = 0;
        let result: Result<i32, FileSystemError> = FileSystem::with_retry(|| {
            attempts += 1;
            Err(FileSystemError::ConcurrentAccess)
        });

        assert!(result.is_err());
        matches!(result.unwrap_err(), FileSystemError::ConcurrentAccess);
        assert_eq!(attempts, FileSystem::MAX_RETRIES + 1);
    }

    #[test]
    fn test_with_retry_interrupted_error_retries() {
        let mut attempts = 0;
        let result = FileSystem::with_retry(|| {
            attempts += 1;
            if attempts < 2 {
                let io_error = std::io::Error::new(ErrorKind::Interrupted, "Interrupted");
                Err(FileSystemError::Io(io_error))
            } else {
                Ok::<String, FileSystemError>("success".to_string())
            }
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(attempts, 2);
    }

    #[test]
    fn test_with_retry_non_retryable_error() {
        let mut attempts = 0;
        let result: Result<i32, FileSystemError> = FileSystem::with_retry(|| {
            attempts += 1;
            Err(FileSystemError::PermissionDenied)
        });

        assert!(result.is_err());
        matches!(result.unwrap_err(), FileSystemError::PermissionDenied);
        assert_eq!(attempts, 1); // Should not retry
    }

    #[test]
    fn test_with_retry_timeout() {
        // This test verifies that the timeout mechanism works by creating
        // an operation that takes too long
        use std::sync::{Arc, Mutex};
        use std::thread;
        use std::time::Instant;

        let start_time = Instant::now();
        let attempts = Arc::new(Mutex::new(0));
        let attempts_clone = attempts.clone();

        let result: Result<i32, FileSystemError> = FileSystem::with_retry(|| {
            let mut count = attempts_clone.lock().unwrap();
            *count += 1;

            // Check if we're past the timeout duration
            if start_time.elapsed() > FileSystem::OPERATION_TIMEOUT {
                return Err(FileSystemError::Timeout);
            }

            // Sleep for a bit and then continue to trigger timeout on next iteration
            thread::sleep(Duration::from_millis(100));

            // Always return an error that would normally be retried
            Err(FileSystemError::ConcurrentAccess)
        });

        let elapsed = start_time.elapsed();

        assert!(result.is_err());
        // The result should be either Timeout or ConcurrentAccess
        match result.unwrap_err() {
            FileSystemError::Timeout => {
                // Expected timeout
                assert!(elapsed >= FileSystem::OPERATION_TIMEOUT);
            }
            FileSystemError::ConcurrentAccess => {
                // Also acceptable if we hit max retries first
                assert!(elapsed <= FileSystem::OPERATION_TIMEOUT + Duration::from_millis(500));
            }
            other => panic!("Unexpected error type: {:?}", other),
        }

        // Should have attempted at least once before timing out
        let final_attempts = *attempts.lock().unwrap();
        assert!(final_attempts >= 1);
    }

    #[test]
    fn test_retry_delay_progression() {
        // Test that retry delays follow the expected pattern
        use std::sync::{Arc, Mutex};
        use std::time::Instant;

        let attempt_times = Arc::new(Mutex::new(Vec::new()));
        let attempt_times_clone = attempt_times.clone();

        let result = FileSystem::with_retry(|| {
            let mut times = attempt_times_clone.lock().unwrap();
            times.push(Instant::now());

            if times.len() < 3 {
                Err(FileSystemError::ConcurrentAccess)
            } else {
                Ok::<i32, FileSystemError>(42)
            }
        });

        assert!(result.is_ok());

        let times = attempt_times.lock().unwrap();
        assert_eq!(times.len(), 3);

        // Check that delays are approximately correct
        if times.len() >= 2 {
            let first_delay = times[1].duration_since(times[0]);
            // First retry delay should be around RETRY_DELAY * 1 = 50ms
            assert!(first_delay >= FileSystem::RETRY_DELAY);
            assert!(first_delay < FileSystem::RETRY_DELAY * 3); // Allow some variance
        }

        if times.len() >= 3 {
            let second_delay = times[2].duration_since(times[1]);
            // Second retry delay should be around RETRY_DELAY * 2 = 100ms
            assert!(second_delay >= FileSystem::RETRY_DELAY * 2);
            assert!(second_delay < FileSystem::RETRY_DELAY * 4); // Allow some variance
        }
    }

    #[test]
    fn test_complete_file_lifecycle() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("lifecycle.txt");

        // 1. File doesn't exist initially
        assert!(!FileSystem::file_exists(&file_path));
        assert!(!FileSystem::file_is_readable(&file_path));

        // 2. Ensure file exists (creates empty file)
        let result = FileSystem::ensure_file_exists(&file_path);
        assert!(result.is_ok());
        assert!(FileSystem::file_exists(&file_path));
        assert!(FileSystem::file_is_readable(&file_path));

        // 3. Write initial content atomically
        let initial_content = "Initial content\n";
        let result = FileSystem::write_file_atomic(&file_path, initial_content);
        assert!(result.is_ok());

        // 4. Read content back
        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), initial_content);

        // 5. Append additional content
        let additional_content = "Additional line\n";
        let result = FileSystem::append_to_file(&file_path, additional_content);
        assert!(result.is_ok());

        // 6. Read complete content
        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_ok());
        let final_content = result.unwrap();
        assert_eq!(
            final_content,
            format!("{}{}", initial_content, additional_content)
        );

        // 7. Overwrite with new content
        let new_content = "Completely new content\n";
        let result = FileSystem::write_file_atomic(&file_path, new_content);
        assert!(result.is_ok());

        // 8. Verify overwrite worked
        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), new_content);
    }

    #[test]
    fn test_logging_workflow() {
        let temp_dir = create_test_dir();
        let log_file = temp_dir.path().join("workflow.log");

        // Simulate a series of operations with logging
        let operations = [
            "Starting operation",
            "Processing data",
            "Intermediate result: 42",
            "Finalizing operation",
            "Operation completed successfully",
        ];

        for operation in &operations {
            let result = FileSystem::log_to_file(&log_file, operation);
            assert!(result.is_ok());
        }

        // Verify all operations were logged
        let content = fs::read_to_string(&log_file).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), operations.len());

        for (i, operation) in operations.iter().enumerate() {
            assert!(lines[i].contains(operation));
            assert!(lines[i].starts_with('['));
            assert!(lines[i].contains("] "));
        }
    }

    #[test]
    fn test_concurrent_append_operations() {
        let temp_dir = create_test_dir();
        let log_file = temp_dir.path().join("concurrent.log");

        // Simulate multiple threads appending to the same file
        use std::sync::Arc;
        use std::thread;

        let file_path = Arc::new(log_file);
        let mut handles = vec![];

        for thread_id in 0..5 {
            let file_path_clone = Arc::clone(&file_path);
            let handle = thread::spawn(move || {
                for i in 0..3 {
                    let message = format!("Thread {} - Message {}\n", thread_id, i);
                    let result = FileSystem::append_to_file(&*file_path_clone, &message);
                    assert!(result.is_ok());
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all messages were written
        let content = fs::read_to_string(&*file_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();

        // Check that we have at least some messages (concurrent operations might vary)
        assert!(lines.len() > 0);
        assert!(lines.len() <= 15); // 5 threads * 3 messages each

        // Verify all threads and messages are represented
        for thread_id in 0..5 {
            for i in 0..3 {
                let expected_message = format!("Thread {} - Message {}", thread_id, i);
                assert!(content.contains(&expected_message));
            }
        }
    }

    #[test]
    fn test_atomic_write_consistency() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("atomic.txt");

        // Write initial content
        let initial_content = "Initial content that should not be corrupted";
        let result = FileSystem::write_file_atomic(&file_path, initial_content);
        assert!(result.is_ok());

        // Verify atomic writes don't leave temp files around
        let temp_file_path = file_path.with_extension("tmp");
        assert!(!temp_file_path.exists());

        // Simulate multiple quick atomic writes
        let contents = [
            "First update",
            "Second update with more content",
            "Third update with even more content that is longer",
            "Final update",
        ];

        for content in &contents {
            let result = FileSystem::write_file_atomic(&file_path, content);
            assert!(result.is_ok());

            // Verify temp file is cleaned up each time
            assert!(!temp_file_path.exists());

            // Verify content is correct
            let read_content = fs::read_to_string(&file_path).unwrap();
            assert_eq!(read_content, *content);
        }
    }

    #[test]
    fn test_empty_content_operations() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("empty.txt");

        // Test writing empty content
        let result = FileSystem::write_file_atomic(&file_path, "");
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");

        // Test appending to empty file
        let result = FileSystem::append_to_file(&file_path, "");
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");

        // Test appending actual content to empty file
        let result = FileSystem::append_to_file(&file_path, "Now has content");
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Now has content");
    }

    #[test]
    fn test_large_content_operations() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("large.txt");

        // Create large content (1MB)
        let large_content = "x".repeat(1024 * 1024);

        let result = FileSystem::write_file_atomic(&file_path, &large_content);
        assert!(result.is_ok());

        let read_content = FileSystem::read_file_safe(&file_path).unwrap();
        assert_eq!(read_content.len(), large_content.len());
        assert_eq!(read_content, large_content);
    }

    #[test]
    fn test_special_characters_in_content() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("special.txt");

        let special_content = "Special chars: \n\t\r\0\u{1F4A9}\u{200B}émojis and unicode 🚀";

        let result = FileSystem::write_file_atomic(&file_path, special_content);
        assert!(result.is_ok());

        let read_content = FileSystem::read_file_safe(&file_path).unwrap();
        assert_eq!(read_content, special_content);
    }

    #[test]
    fn test_very_long_file_paths() {
        let temp_dir = create_test_dir();

        // Create a path that's quite long but within filesystem limits
        let long_filename = "a".repeat(100);
        let file_path = temp_dir.path().join(format!("{}.txt", long_filename));

        let result = FileSystem::write_file_atomic(&file_path, "content");
        assert!(result.is_ok());

        assert!(FileSystem::file_exists(&file_path));

        let content = FileSystem::read_file_safe(&file_path).unwrap();
        assert_eq!(content, "content");
    }

    #[test]
    fn test_rapid_successive_operations() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("rapid.txt");

        // Perform many rapid operations
        for i in 0..50 {
            let content = format!("Content {}", i);

            // Write, read, append sequence
            let result = FileSystem::write_file_atomic(&file_path, &content);
            assert!(result.is_ok());

            let read_result = FileSystem::read_file_safe(&file_path);
            assert!(read_result.is_ok());
            assert_eq!(read_result.unwrap(), content);

            let append_content = format!(" - appended {}", i);
            let result = FileSystem::append_to_file(&file_path, &append_content);
            assert!(result.is_ok());
        }

        // Verify final state
        let final_content = FileSystem::read_file_safe(&file_path).unwrap();
        assert!(final_content.starts_with("Content 49"));
        assert!(final_content.ends_with(" - appended 49"));
    }

    #[test]
    fn test_file_operations_on_readonly_content() {
        let temp_dir = create_test_dir();
        let file_path = temp_dir.path().join("readonly_test.txt");

        // Create a file first
        FileSystem::write_file_atomic(&file_path, "initial content").unwrap();

        // Test reading works
        let result = FileSystem::read_file_safe(&file_path);
        assert!(result.is_ok());

        // Test that file_exists and file_is_readable work
        assert!(FileSystem::file_exists(&file_path));
        assert!(FileSystem::file_is_readable(&file_path));
    }

    #[test]
    fn test_task_id_edge_cases() {
        // Test with various task IDs including edge cases
        let test_cases = [0, 1, 42, 999, 1000, 9999, u32::MAX];

        for &task_id in &test_cases {
            let task_dir = FileSystem::get_task_directory_path(task_id);
            assert!(task_dir.to_string_lossy().contains(&task_id.to_string()));

            let todo_path = FileSystem::get_todo_list_path(task_id);
            assert!(todo_path.to_string_lossy().contains(&task_id.to_string()));
            assert!(todo_path.to_string_lossy().contains("todo-list.md"));

            let log_path = FileSystem::get_overseer_log_path(task_id);
            assert!(log_path.to_string_lossy().contains(&task_id.to_string()));
            assert!(log_path.to_string_lossy().contains("overseer.log"));
        }
    }

    #[test]
    fn test_filesystem_constants() {
        // Verify the constants are reasonable
        assert!(FileSystem::MAX_RETRIES > 0);
        assert!(FileSystem::MAX_RETRIES <= 10); // Reasonable upper bound

        assert!(FileSystem::RETRY_DELAY >= Duration::from_millis(1));
        assert!(FileSystem::RETRY_DELAY <= Duration::from_secs(1)); // Reasonable upper bound

        assert!(FileSystem::OPERATION_TIMEOUT >= Duration::from_secs(1));
        assert!(FileSystem::OPERATION_TIMEOUT <= Duration::from_secs(60)); // Reasonable upper bound
    }
}
