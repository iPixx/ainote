use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use thiserror::Error;

/// Errors that can occur during vector database operations
#[derive(Error, Debug)]
pub enum VectorDbError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Compression error: {message}")]
    Compression { message: String },
    
    #[error("Data integrity error: checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Version compatibility error: expected {expected}, found {found}")]
    VersionIncompatible { expected: String, found: String },
    
    #[error("Invalid embedding entry: {reason}")]
    InvalidEntry { reason: String },
    
    #[error("Storage error: {message}")]
    Storage { message: String },
}

pub type VectorDbResult<T> = Result<T, VectorDbError>;

/// Version information for data format compatibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataVersion {
    /// Major version (breaking changes)
    pub major: u32,
    /// Minor version (backward compatible features)
    pub minor: u32,
    /// Patch version (bug fixes)
    pub patch: u32,
}

impl DataVersion {
    /// Current data format version
    pub const CURRENT: DataVersion = DataVersion {
        major: 1,
        minor: 0,
        patch: 0,
    };
    
    /// Check if this version is compatible with another version
    /// Returns true if 'other' version can be read by this version
    pub fn is_compatible(&self, other: &DataVersion) -> bool {
        // Same major version is compatible, this version can read older or same minor versions
        self.major == other.major && self.minor >= other.minor
    }
    
    /// Convert to string representation
    pub fn version_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for DataVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// Metadata associated with an embedding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingMetadata {
    /// Full path to the source file
    pub file_path: String,
    /// Unique identifier for the text chunk within the file
    pub chunk_id: String,
    /// Timestamp when the embedding was created
    pub created_at: u64,
    /// Timestamp when the entry was last updated
    pub updated_at: u64,
    /// Content preview (first 100 characters of the original text)
    pub content_preview: String,
    /// Original text length in characters
    pub text_length: usize,
    /// Model name used to generate the embedding
    pub model_name: String,
    /// Hash of the original text (for deduplication and validation)
    pub text_hash: String,
    /// Additional custom metadata
    pub custom_metadata: HashMap<String, String>,
}

impl EmbeddingMetadata {
    /// Create new metadata with required fields
    pub fn new(
        file_path: String,
        chunk_id: String,
        content_preview: String,
        text_length: usize,
        model_name: String,
        original_text: &str,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let text_hash = Self::compute_text_hash(original_text);
        
        Self {
            file_path,
            chunk_id,
            created_at: now,
            updated_at: now,
            content_preview,
            text_length,
            model_name,
            text_hash,
            custom_metadata: HashMap::new(),
        }
    }
    
    /// Compute SHA-256 hash of text for validation and deduplication
    pub fn compute_text_hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Update the last modified timestamp
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
    
    /// Add custom metadata field
    pub fn add_custom_metadata(&mut self, key: String, value: String) {
        self.custom_metadata.insert(key, value);
        self.touch();
    }
    
    /// Get custom metadata field
    pub fn get_custom_metadata(&self, key: &str) -> Option<&String> {
        self.custom_metadata.get(key)
    }
    
    /// Create content preview from original text (first 100 chars)
    pub fn create_preview(text: &str) -> String {
        if text.len() <= 100 {
            text.to_string()
        } else {
            format!("{}...", &text[..97])
        }
    }
    
    /// Validate metadata consistency
    pub fn validate(&self) -> VectorDbResult<()> {
        if self.file_path.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "file_path cannot be empty".to_string(),
            });
        }
        
        if self.chunk_id.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "chunk_id cannot be empty".to_string(),
            });
        }
        
        if self.model_name.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "model_name cannot be empty".to_string(),
            });
        }
        
        if self.text_hash.len() != 64 {
            return Err(VectorDbError::InvalidEntry {
                reason: "text_hash must be 64 character SHA-256 hex string".to_string(),
            });
        }
        
        if self.text_length == 0 {
            return Err(VectorDbError::InvalidEntry {
                reason: "text_length must be greater than 0".to_string(),
            });
        }
        
        Ok(())
    }
}

/// Core embedding entry containing vector data and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingEntry {
    /// Unique identifier for this embedding entry
    pub id: String,
    /// The embedding vector
    pub vector: Vec<f32>,
    /// Associated metadata
    pub metadata: EmbeddingMetadata,
    /// Entry creation timestamp
    pub created_at: u64,
    /// Last modification timestamp
    pub updated_at: u64,
}

impl EmbeddingEntry {
    /// Create a new embedding entry
    pub fn new(
        vector: Vec<f32>,
        file_path: String,
        chunk_id: String,
        original_text: &str,
        model_name: String,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let content_preview = EmbeddingMetadata::create_preview(original_text);
        let metadata = EmbeddingMetadata::new(
            file_path.clone(),
            chunk_id.clone(),
            content_preview,
            original_text.len(),
            model_name,
            original_text,
        );
        
        let id = Self::generate_id(&file_path, &chunk_id, &metadata.text_hash);
        
        Self {
            id,
            vector,
            metadata,
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Generate a unique ID for the embedding entry
    pub fn generate_id(file_path: &str, chunk_id: &str, text_hash: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_path.as_bytes());
        hasher.update(b":");
        hasher.update(chunk_id.as_bytes());
        hasher.update(b":");
        hasher.update(text_hash.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// Update the embedding vector and timestamp
    pub fn update_vector(&mut self, new_vector: Vec<f32>) {
        self.vector = new_vector;
        self.touch();
    }
    
    /// Update the last modified timestamp
    pub fn touch(&mut self) {
        self.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.metadata.touch();
    }
    
    /// Get vector dimension
    pub fn dimension(&self) -> usize {
        self.vector.len()
    }
    
    /// Calculate memory footprint in bytes (approximate)
    pub fn memory_footprint(&self) -> usize {
        // Vector (f32 = 4 bytes per dimension)
        let vector_size = self.vector.len() * 4;
        
        // String fields (approximate)
        let strings_size = self.id.len()
            + self.metadata.file_path.len()
            + self.metadata.chunk_id.len()
            + self.metadata.content_preview.len()
            + self.metadata.model_name.len()
            + self.metadata.text_hash.len()
            + self.metadata.custom_metadata.iter()
                .map(|(k, v)| k.len() + v.len())
                .sum::<usize>();
        
        // Fixed size fields
        let fixed_size = std::mem::size_of::<u64>() * 4 // timestamps
            + std::mem::size_of::<usize>(); // text_length
        
        vector_size + strings_size + fixed_size
    }
    
    /// Validate the embedding entry
    pub fn validate(&self) -> VectorDbResult<()> {
        if self.id.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "id cannot be empty".to_string(),
            });
        }
        
        if self.vector.is_empty() {
            return Err(VectorDbError::InvalidEntry {
                reason: "vector cannot be empty".to_string(),
            });
        }
        
        // Validate vector contains valid float values
        for (i, &value) in self.vector.iter().enumerate() {
            if !value.is_finite() {
                return Err(VectorDbError::InvalidEntry {
                    reason: format!("vector contains invalid value at index {}: {}", i, value),
                });
            }
        }
        
        self.metadata.validate()?;
        
        Ok(())
    }
    
    /// Compute checksum for data integrity verification
    pub fn compute_checksum(&self) -> VectorDbResult<String> {
        let serialized = serde_json::to_string(self)?;
        let mut hasher = Sha256::new();
        hasher.update(serialized.as_bytes());
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Configuration for vector database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorStorageConfig {
    /// Base directory for vector storage files
    pub storage_dir: String,
    /// Enable compression for storage files
    pub enable_compression: bool,
    /// Compression algorithm to use
    pub compression_algorithm: CompressionAlgorithm,
    /// Maximum entries per storage file
    pub max_entries_per_file: usize,
    /// Enable data integrity checksums
    pub enable_checksums: bool,
    /// Automatic backup creation
    pub auto_backup: bool,
    /// Maximum number of backup files to keep
    pub max_backups: usize,
    /// Enable detailed storage metrics
    pub enable_metrics: bool,
}

impl Default for VectorStorageConfig {
    fn default() -> Self {
        Self {
            storage_dir: "vector_storage".to_string(),
            enable_compression: true,
            compression_algorithm: CompressionAlgorithm::Gzip,
            max_entries_per_file: 1000,
            enable_checksums: true,
            auto_backup: true,
            max_backups: 5,
            enable_metrics: true,
        }
    }
}

/// Supported compression algorithms
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,
    /// Gzip compression (good balance of speed and compression ratio)
    Gzip,
    /// LZ4 compression (faster, lower compression ratio)
    Lz4,
}

impl CompressionAlgorithm {
    /// Get file extension for the compression algorithm
    pub fn file_extension(&self) -> &'static str {
        match self {
            CompressionAlgorithm::None => "",
            CompressionAlgorithm::Gzip => ".gz",
            CompressionAlgorithm::Lz4 => ".lz4",
        }
    }
}

/// Header information for vector storage files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageFileHeader {
    /// Data format version
    pub version: DataVersion,
    /// Compression algorithm used
    pub compression: CompressionAlgorithm,
    /// File creation timestamp
    pub created_at: u64,
    /// Number of entries in the file
    pub entry_count: usize,
    /// Total uncompressed data size in bytes
    pub uncompressed_size: usize,
    /// File checksum (if enabled)
    pub checksum: Option<String>,
    /// Custom header metadata
    pub metadata: HashMap<String, String>,
}

impl StorageFileHeader {
    /// Create a new storage file header
    pub fn new(compression: CompressionAlgorithm, entry_count: usize) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            version: DataVersion::CURRENT,
            compression,
            created_at: now,
            entry_count,
            uncompressed_size: 0,
            checksum: None,
            metadata: HashMap::new(),
        }
    }
    
    /// Validate header compatibility
    pub fn validate_compatibility(&self) -> VectorDbResult<()> {
        if !DataVersion::CURRENT.is_compatible(&self.version) {
            return Err(VectorDbError::VersionIncompatible {
                expected: DataVersion::CURRENT.version_string(),
                found: self.version.version_string(),
            });
        }
        Ok(())
    }
}

/// Storage statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageMetrics {
    /// Total number of entries stored
    pub total_entries: usize,
    /// Number of storage files
    pub file_count: usize,
    /// Total storage size in bytes (compressed)
    pub total_size_bytes: usize,
    /// Total uncompressed size in bytes
    pub uncompressed_size_bytes: usize,
    /// Compression ratio (compressed/uncompressed)
    pub compression_ratio: f64,
    /// Average entries per file
    pub avg_entries_per_file: f64,
    /// Timestamp of last metrics update
    pub last_updated: u64,
}

impl StorageMetrics {
    /// Update metrics with current data
    pub fn update(&mut self, total_entries: usize, file_count: usize, total_size: usize, uncompressed_size: usize) {
        self.total_entries = total_entries;
        self.file_count = file_count;
        self.total_size_bytes = total_size;
        self.uncompressed_size_bytes = uncompressed_size;
        
        self.compression_ratio = if uncompressed_size > 0 {
            total_size as f64 / uncompressed_size as f64
        } else {
            1.0
        };
        
        self.avg_entries_per_file = if file_count > 0 {
            total_entries as f64 / file_count as f64
        } else {
            0.0
        };
        
        self.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_version_compatibility() {
        let current = DataVersion::CURRENT; // 1.0.0
        let newer_minor = DataVersion { major: 1, minor: 1, patch: 0 };
        let incompatible_major = DataVersion { major: 2, minor: 0, patch: 0 };
        let older_minor = DataVersion { major: 1, minor: 0, patch: 0 };
        
        // Current version (1.0.0) cannot read newer minor version (1.1.0)
        assert!(!current.is_compatible(&newer_minor));
        // Current version (1.0.0) cannot read different major version (2.0.0)
        assert!(!current.is_compatible(&incompatible_major));
        // Current version (1.0.0) can read same version (1.0.0)
        assert!(current.is_compatible(&current));
        // Newer minor version (1.1.0) can read older minor version (1.0.0)
        assert!(newer_minor.is_compatible(&older_minor));
        // Newer minor version (1.1.0) can read same version (1.1.0)
        assert!(newer_minor.is_compatible(&newer_minor));
    }

    #[test]
    fn test_embedding_metadata_creation() {
        let text = "This is a test document with some content that will be used to create an embedding.";
        let metadata = EmbeddingMetadata::new(
            "/path/to/file.md".to_string(),
            "chunk_1".to_string(),
            EmbeddingMetadata::create_preview(text),
            text.len(),
            "test-model".to_string(),
            text,
        );
        
        assert_eq!(metadata.file_path, "/path/to/file.md");
        assert_eq!(metadata.chunk_id, "chunk_1");
        assert_eq!(metadata.text_length, text.len());
        assert_eq!(metadata.model_name, "test-model");
        assert!(!metadata.text_hash.is_empty());
        assert_eq!(metadata.text_hash.len(), 64); // SHA-256 hex string
    }

    #[test]
    fn test_embedding_entry_creation() {
        let vector = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let text = "Test text for embedding";
        
        let entry = EmbeddingEntry::new(
            vector.clone(),
            "/test/file.md".to_string(),
            "chunk_1".to_string(),
            text,
            "test-model".to_string(),
        );
        
        assert_eq!(entry.vector, vector);
        assert_eq!(entry.dimension(), 5);
        assert!(!entry.id.is_empty());
        assert_eq!(entry.metadata.text_length, text.len());
    }

    #[test]
    fn test_embedding_entry_validation() {
        let mut entry = EmbeddingEntry::new(
            vec![0.1, 0.2, 0.3],
            "/test/file.md".to_string(),
            "chunk_1".to_string(),
            "test text",
            "test-model".to_string(),
        );
        
        // Valid entry
        assert!(entry.validate().is_ok());
        
        // Invalid vector with NaN
        entry.vector = vec![0.1, f32::NAN, 0.3];
        assert!(entry.validate().is_err());
        
        // Invalid vector with infinity
        entry.vector = vec![0.1, f32::INFINITY, 0.3];
        assert!(entry.validate().is_err());
        
        // Empty vector
        entry.vector = vec![];
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_content_preview() {
        let short_text = "Short text";
        let long_text = "This is a very long text that should be truncated when creating a preview because it exceeds the maximum preview length of 100 characters and should be cut off at some point.";
        
        let short_preview = EmbeddingMetadata::create_preview(short_text);
        let long_preview = EmbeddingMetadata::create_preview(long_text);
        
        assert_eq!(short_preview, short_text);
        assert!(long_preview.len() <= 100);
        assert!(long_preview.ends_with("..."));
    }

    #[test]
    fn test_text_hash_consistency() {
        let text = "Test text for hashing";
        let hash1 = EmbeddingMetadata::compute_text_hash(text);
        let hash2 = EmbeddingMetadata::compute_text_hash(text);
        let hash3 = EmbeddingMetadata::compute_text_hash("Different text");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA-256 hex string length
    }

    #[test]
    fn test_entry_id_generation() {
        let file_path = "/test/file.md";
        let chunk_id = "chunk_1";
        let text_hash = "abcdef1234567890";
        
        let id1 = EmbeddingEntry::generate_id(file_path, chunk_id, text_hash);
        let id2 = EmbeddingEntry::generate_id(file_path, chunk_id, text_hash);
        let id3 = EmbeddingEntry::generate_id(file_path, "chunk_2", text_hash);
        
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert_eq!(id1.len(), 64); // SHA-256 hex string length
    }

    #[test]
    fn test_compression_algorithm_extensions() {
        assert_eq!(CompressionAlgorithm::None.file_extension(), "");
        assert_eq!(CompressionAlgorithm::Gzip.file_extension(), ".gz");
        assert_eq!(CompressionAlgorithm::Lz4.file_extension(), ".lz4");
    }

    #[test]
    fn test_storage_file_header() {
        let header = StorageFileHeader::new(CompressionAlgorithm::Gzip, 100);
        
        assert_eq!(header.version, DataVersion::CURRENT);
        assert_eq!(header.entry_count, 100);
        assert!(matches!(header.compression, CompressionAlgorithm::Gzip));
        assert!(header.validate_compatibility().is_ok());
    }

    #[test]
    fn test_storage_metrics_update() {
        let mut metrics = StorageMetrics::default();
        metrics.update(1000, 5, 50000, 100000);
        
        assert_eq!(metrics.total_entries, 1000);
        assert_eq!(metrics.file_count, 5);
        assert_eq!(metrics.total_size_bytes, 50000);
        assert_eq!(metrics.uncompressed_size_bytes, 100000);
        assert_eq!(metrics.compression_ratio, 0.5);
        assert_eq!(metrics.avg_entries_per_file, 200.0);
    }
}