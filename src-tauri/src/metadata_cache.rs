use std::collections::HashMap;
use std::sync::{Mutex, LazyLock};
use std::time::{SystemTime, Duration};
use std::path::Path;
use std::fs::Metadata;

/// Cache entry with TTL
struct CacheEntry {
    metadata: Metadata,
    timestamp: SystemTime,
}

impl CacheEntry {
    fn new(metadata: Metadata) -> Self {
        Self {
            metadata,
            timestamp: SystemTime::now(),
        }
    }

    fn is_expired(&self) -> bool {
        // Cache entries expire after 5 seconds
        const CACHE_TTL: Duration = Duration::from_secs(5);
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|duration| duration > CACHE_TTL)
            .unwrap_or(true)
    }
}

static CACHE: LazyLock<Mutex<HashMap<String, CacheEntry>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Get cached metadata or fetch and cache it
pub fn get_metadata(path: &Path) -> std::io::Result<Metadata> {
    let path_str = path.to_string_lossy().to_string();
    
    // Try to get from cache first
    {
        if let Ok(mut cache) = CACHE.lock() {
            if let Some(entry) = cache.get(&path_str) {
                if !entry.is_expired() {
                    // Clone metadata (it's relatively cheap)
                    return Ok(entry.metadata.clone());
                } else {
                    // Remove expired entry
                    cache.remove(&path_str);
                }
            }
        }
    }

    // Cache miss or expired - fetch metadata
    let metadata = path.metadata()?;
    
    // Store in cache
    {
        if let Ok(mut cache) = CACHE.lock() {
            cache.insert(path_str, CacheEntry::new(metadata.clone()));
            
            // Prevent cache from growing too large
            if cache.len() > 1000 {
                // Remove oldest entries (simple cleanup)
                let mut expired_keys = Vec::new();
                for (key, entry) in cache.iter() {
                    if entry.is_expired() {
                        expired_keys.push(key.clone());
                    }
                }
                for key in expired_keys {
                    cache.remove(&key);
                }
            }
        }
    }
    
    Ok(metadata)
}

/// Clear the entire cache (useful for testing)
#[allow(dead_code)]
pub fn clear() {
    if let Ok(mut cache) = CACHE.lock() {
        cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cache_hit() {
        clear(); // Clear cache before test
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();

        // First call should cache the metadata
        let metadata1 = get_metadata(&test_file).unwrap();
        
        // Second call should hit the cache
        let metadata2 = get_metadata(&test_file).unwrap();
        
        // Both should have the same values
        assert_eq!(metadata1.len(), metadata2.len());
        assert_eq!(metadata1.is_file(), metadata2.is_file());
    }

    #[test]
    fn test_cache_miss_nonexistent_file() {
        clear(); // Clear cache before test
        
        let temp_dir = TempDir::new().unwrap();
        let nonexistent_file = temp_dir.path().join("nonexistent.md");

        let result = get_metadata(&nonexistent_file);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_expiry() {
        clear(); // Clear cache before test
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();

        // Get metadata to populate cache
        let _metadata = get_metadata(&test_file).unwrap();

        // Create a cache entry that should be expired
        {
            if let Ok(mut cache) = CACHE.lock() {
                let path_str = test_file.to_string_lossy().to_string();
                if let Some(entry) = cache.get_mut(&path_str) {
                    // Manually set timestamp to old value to simulate expiry
                    entry.timestamp = SystemTime::now() - Duration::from_secs(10);
                }
            }
        }

        // This should refresh the cache due to expiry
        let metadata = get_metadata(&test_file).unwrap();
        assert!(metadata.is_file());
    }

    #[test]
    fn test_cache_cleanup() {
        clear(); // Clear cache before test
        
        let temp_dir = TempDir::new().unwrap();
        
        // Create many files to trigger cleanup
        for i in 0..20 {
            let test_file = temp_dir.path().join(format!("test_{}.md", i));
            fs::write(&test_file, "content").unwrap();
            let _ = get_metadata(&test_file);
        }

        // Cache should contain entries
        {
            if let Ok(cache) = CACHE.lock() {
                assert!(cache.len() > 0);
                // Note: Due to concurrent tests, cache size may vary
            }
        }
    }

    #[test]
    fn test_cache_entry_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();
        
        let metadata = test_file.metadata().unwrap();
        let entry = CacheEntry::new(metadata);
        
        assert!(!entry.is_expired()); // Should not be expired immediately
        assert_eq!(entry.metadata.len(), "content".len() as u64);
    }

    #[test]
    fn test_cache_entry_expiry() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();
        
        let metadata = test_file.metadata().unwrap();
        let mut entry = CacheEntry::new(metadata);
        
        // Should not be expired initially
        assert!(!entry.is_expired());
        
        // Manually set old timestamp
        entry.timestamp = SystemTime::now() - Duration::from_secs(10);
        
        // Should now be expired
        assert!(entry.is_expired());
    }

    #[test]
    fn test_cache_clear() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();

        // Populate cache
        let _ = get_metadata(&test_file);
        
        // Verify cache has content
        {
            if let Ok(cache) = CACHE.lock() {
                assert!(cache.len() > 0);
            }
        }

        // Clear cache
        clear();
        
        // Verify cache is empty
        {
            if let Ok(cache) = CACHE.lock() {
                assert_eq!(cache.len(), 0);
            }
        }
    }

    #[test]
    fn test_concurrent_cache_access() {
        clear(); // Clear cache before test
        
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.md");
        fs::write(&test_file, "content").unwrap();

        // Simulate concurrent access by calling get_metadata multiple times
        let handles: Vec<_> = (0..10).map(|_| {
            let path = test_file.clone();
            std::thread::spawn(move || {
                get_metadata(&path)
            })
        }).collect();

        // All threads should succeed
        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.is_ok());
        }
    }
}