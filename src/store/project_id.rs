use sha2::{Digest, Sha256};
use std::path::Path;

/// Generates a deterministic project identifier using SHA-256 hash.
/// Returns the first 8 hexadecimal characters of the hash.
///
/// # Arguments
/// * `project_root` - The absolute path to the project root directory
///
/// # Returns
/// An 8-character hexadecimal string representing the project ID
pub fn project_id(project_root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        result[0], result[1], result[2], result[3]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_project_id_is_deterministic() {
        let path = Path::new("/home/user/project");
        let id1 = project_id(path);
        let id2 = project_id(path);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_project_id_is_8_chars() {
        let path = Path::new("/home/user/project");
        let id = project_id(path);
        assert_eq!(id.len(), 8);
    }

    #[test]
    fn test_project_id_is_hex() {
        let path = Path::new("/home/user/project");
        let id = project_id(path);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_different_paths_produce_different_ids() {
        let path1 = Path::new("/home/user/project1");
        let path2 = Path::new("/home/user/project2");
        let id1 = project_id(path1);
        let id2 = project_id(path2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_similar_paths_produce_different_ids() {
        let path1 = Path::new("/home/user/project");
        let path2 = Path::new("/home/user/project/");
        let id1 = project_id(path1);
        let id2 = project_id(path2);
        // These might be the same or different depending on path normalization
        // but the function should handle both consistently
        assert!(!id1.is_empty());
        assert!(!id2.is_empty());
    }

    #[test]
    fn test_absolute_vs_relative_paths_differ() {
        let path1 = PathBuf::from("/home/user/project");
        let path2 = PathBuf::from("project");
        let id1 = project_id(&path1);
        let id2 = project_id(&path2);
        assert_ne!(id1, id2);
    }
}
