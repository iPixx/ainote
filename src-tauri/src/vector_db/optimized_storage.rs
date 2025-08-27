//! Optimized Storage Module
//!
//! This module provides optimized JSON storage formats and serialization techniques
//! to reduce file sizes and improve I/O performance for vector database operations.
//!
//! ## Features
//!
//! - **Compact JSON Serialization**: Minimized field names and optimized structure
//! - **Delta Encoding**: Store only changes between similar vectors
//! - **Batch Optimization**: Efficient storage of multiple embeddings
//! - **Binary Encoding**: Optional binary formats for maximum efficiency
//! - **Progressive Loading**: Support for streaming and lazy loading

use std::collections::HashMap;
use std::io::{Read, Write};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

use crate::vector_db::types::{EmbeddingEntry, EmbeddingMetadata, VectorDbError, VectorDbResult};
use crate::vector_db::compression::{VectorCompressor, CompressedVector};

/// Errors that can occur during optimized storage operations
#[derive(Error, Debug)]
pub enum OptimizedStorageError {
    #[error("Serialization failed: {message}")]
    SerializationFailed { message: String },
    
    #[error("Deserialization failed: {message}")]
    DeserializationFailed { message: String },
    
    #[error("Compression failed: {message}")]
    CompressionFailed { message: String },
    
    #[error("Invalid storage format: {message}")]
    InvalidFormat { message: String },
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type OptimizedStorageResult<T> = Result<T, OptimizedStorageError>;

/// Configuration for optimized storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedStorageConfig {
    /// Use compact field names to reduce JSON size
    pub use_compact_field_names: bool,
    /// Enable delta encoding for similar vectors
    pub enable_delta_encoding: bool,
    /// Similarity threshold for delta encoding (0.8-0.99)
    pub delta_similarity_threshold: f32,
    /// Use binary encoding instead of JSON for maximum efficiency
    pub use_binary_encoding: bool,
    /// Enable batch compression for multiple entries
    pub enable_batch_compression: bool,
    /// Minimum batch size for compression
    pub min_batch_size: usize,
    /// Enable progressive loading support
    pub enable_progressive_loading: bool,
    /// Chunk size for progressive loading
    pub progressive_chunk_size: usize,
}

impl Default for OptimizedStorageConfig {
    fn default() -> Self {
        Self {
            use_compact_field_names: true,
            enable_delta_encoding: true,
            delta_similarity_threshold: 0.85,
            use_binary_encoding: false, // Keep JSON for readability by default
            enable_batch_compression: true,
            min_batch_size: 10,
            enable_progressive_loading: true,
            progressive_chunk_size: 100,
        }
    }
}

/// Compact representation of embedding metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactEmbeddingMetadata {
    /// File path (shortened field name)
    #[serde(rename = "fp")]
    pub file_path: String,
    
    /// Chunk ID (shortened field name)
    #[serde(rename = "cid")]
    pub chunk_id: String,
    
    /// Created timestamp (shortened field name)
    #[serde(rename = "ct")]
    pub created_at: u64,
    
    /// Updated timestamp (shortened field name)
    #[serde(rename = "ut")]
    pub updated_at: u64,
    
    /// Content preview (shortened and optional)
    #[serde(rename = "cp", skip_serializing_if = "Option::is_none")]
    pub content_preview: Option<String>,
    
    /// Text length (shortened field name)
    #[serde(rename = "tl")]
    pub text_length: usize,
    
    /// Model name (shortened field name)
    #[serde(rename = "mn")]
    pub model_name: String,
    
    /// Text hash (shortened field name)
    #[serde(rename = "th")]
    pub text_hash: String,
    
    /// Custom metadata (optional and shortened)
    #[serde(rename = "cm", skip_serializing_if = "HashMap::is_empty")]
    pub custom_metadata: HashMap<String, String>,
}

impl From<EmbeddingMetadata> for CompactEmbeddingMetadata {
    fn from(metadata: EmbeddingMetadata) -> Self {
        Self {
            file_path: metadata.file_path,
            chunk_id: metadata.chunk_id,
            created_at: metadata.created_at,
            updated_at: metadata.updated_at,
            content_preview: if metadata.content_preview.is_empty() {
                None
            } else {
                Some(metadata.content_preview)
            },
            text_length: metadata.text_length,
            model_name: metadata.model_name,
            text_hash: metadata.text_hash,
            custom_metadata: metadata.custom_metadata,
        }
    }
}

impl Into<EmbeddingMetadata> for CompactEmbeddingMetadata {
    fn into(self) -> EmbeddingMetadata {
        EmbeddingMetadata {
            file_path: self.file_path,
            chunk_id: self.chunk_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            content_preview: self.content_preview.unwrap_or_default(),
            text_length: self.text_length,
            model_name: self.model_name,
            text_hash: self.text_hash,
            custom_metadata: self.custom_metadata,
        }
    }
}

/// Compact representation of embedding entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactEmbeddingEntry {
    /// Entry ID (shortened field name)
    #[serde(rename = "id")]
    pub id: String,
    
    /// Compressed vector or reference to shared vector
    #[serde(rename = "v")]
    pub vector: CompactVector,
    
    /// Compact metadata
    #[serde(rename = "m")]
    pub metadata: CompactEmbeddingMetadata,
    
    /// Created timestamp (shortened field name)
    #[serde(rename = "ct")]
    pub created_at: u64,
    
    /// Updated timestamp (shortened field name)
    #[serde(rename = "ut")]
    pub updated_at: u64,
}

/// Optimized vector storage format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompactVector {
    /// Raw f32 vector (uncompressed)
    #[serde(rename = "r")]
    Raw(Vec<f32>),
    
    /// Compressed vector data
    #[serde(rename = "c")]
    Compressed {
        /// Compressed data
        #[serde(rename = "d")]
        data: Vec<u8>,
        /// Original dimension
        #[serde(rename = "dim")]
        dimension: usize,
        /// Compression parameters
        #[serde(rename = "p")]
        params: CompactCompressionParams,
    },
    
    /// Reference to another vector (for delta encoding)
    #[serde(rename = "ref")]
    Reference {
        /// Reference vector ID
        #[serde(rename = "rid")]
        reference_id: String,
        /// Delta from reference
        #[serde(rename = "d")]
        delta: Vec<f32>,
    },
    
    /// Shared vector reference (for identical vectors)
    #[serde(rename = "s")]
    Shared {
        /// Shared vector pool ID
        #[serde(rename = "sid")]
        shared_id: String,
    },
}

/// Compact compression parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactCompressionParams {
    /// Minimum value
    #[serde(rename = "min")]
    pub min_value: f32,
    /// Maximum value
    #[serde(rename = "max")]
    pub max_value: f32,
    /// Scale factor
    #[serde(rename = "s")]
    pub scale: f32,
    /// Quantization bits
    #[serde(rename = "b")]
    pub bits: u8,
}

/// Optimized storage batch with shared vector pool
#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizedStorageBatch {
    /// Format version
    #[serde(rename = "v")]
    pub version: u32,
    
    /// Batch metadata
    #[serde(rename = "m")]
    pub metadata: BatchMetadata,
    
    /// Shared vector pool (for identical vectors)
    #[serde(rename = "pool", skip_serializing_if = "HashMap::is_empty")]
    pub shared_vectors: HashMap<String, Vec<f32>>,
    
    /// Reference vectors (for delta encoding)
    #[serde(rename = "refs", skip_serializing_if = "HashMap::is_empty")]
    pub reference_vectors: HashMap<String, Vec<f32>>,
    
    /// Compact embedding entries
    #[serde(rename = "e")]
    pub entries: Vec<CompactEmbeddingEntry>,
}

/// Metadata for optimized storage batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Creation timestamp
    #[serde(rename = "ct")]
    pub created_at: u64,
    
    /// Number of entries
    #[serde(rename = "n")]
    pub entry_count: usize,
    
    /// Compression algorithm used
    #[serde(rename = "ca")]
    pub compression_algorithm: String,
    
    /// Original uncompressed size (bytes)
    #[serde(rename = "os")]
    pub original_size: usize,
    
    /// Compression ratio achieved
    #[serde(rename = "cr")]
    pub compression_ratio: f32,
}

/// Optimized storage engine
pub struct OptimizedStorageEngine {
    config: OptimizedStorageConfig,
    vector_compressor: Option<VectorCompressor>,
    shared_vector_pool: HashMap<String, Vec<f32>>,
    reference_vectors: HashMap<String, Vec<f32>>,
}

impl OptimizedStorageEngine {
    /// Create new optimized storage engine
    pub fn new(config: OptimizedStorageConfig) -> OptimizedStorageResult<Self> {
        let vector_compressor = if config.enable_delta_encoding || config.enable_batch_compression {
            Some(VectorCompressor::new(Default::default())
                .map_err(|e| OptimizedStorageError::SerializationFailed {
                    message: e.to_string(),
                })?)
        } else {
            None
        };
        
        Ok(Self {
            config,
            vector_compressor,
            shared_vector_pool: HashMap::new(),
            reference_vectors: HashMap::new(),
        })
    }
    
    /// Serialize a batch of embeddings with optimizations
    pub fn serialize_batch(
        &mut self,
        entries: &[EmbeddingEntry],
    ) -> OptimizedStorageResult<Vec<u8>> {
        let start_time = std::time::Instant::now();
        
        // Convert to compact format
        let compact_entries = self.convert_to_compact_entries(entries)?;
        
        // Create optimized batch
        let batch = OptimizedStorageBatch {
            version: 1,
            metadata: BatchMetadata {
                created_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                entry_count: entries.len(),
                compression_algorithm: "OptimizedV1".to_string(),
                original_size: self.estimate_uncompressed_size(entries),
                compression_ratio: 0.0, // Will be calculated after compression
            },
            shared_vectors: self.shared_vector_pool.clone(),
            reference_vectors: self.reference_vectors.clone(),
            entries: compact_entries,
        };
        
        let serialized = if self.config.use_binary_encoding {
            // Use bincode for maximum efficiency
            bincode::serialize(&batch).map_err(|e| {
                OptimizedStorageError::SerializationFailed {
                    message: e.to_string(),
                }
            })?
        } else {
            // Use compact JSON
            serde_json::to_vec(&batch)?
        };
        
        // Apply compression if enabled
        let final_data = if self.config.enable_batch_compression && 
                            entries.len() >= self.config.min_batch_size {
            self.compress_data(&serialized)?
        } else {
            serialized
        };
        
        eprintln!("📦 Optimized serialization: {} entries -> {} bytes ({:.2}% of original) in {:.2}ms",
                  entries.len(),
                  final_data.len(),
                  (final_data.len() as f32 / self.estimate_uncompressed_size(entries) as f32) * 100.0,
                  start_time.elapsed().as_secs_f64() * 1000.0);
        
        Ok(final_data)
    }
    
    /// Deserialize a batch of embeddings
    pub fn deserialize_batch(&mut self, data: &[u8]) -> OptimizedStorageResult<Vec<EmbeddingEntry>> {
        let start_time = std::time::Instant::now();
        
        // Decompress if needed
        let decompressed_data = if self.is_compressed_data(data) {
            self.decompress_data(data)?
        } else {
            data.to_vec()
        };
        
        // Deserialize batch
        let batch: OptimizedStorageBatch = if self.config.use_binary_encoding {
            bincode::deserialize(&decompressed_data).map_err(|e| {
                OptimizedStorageError::DeserializationFailed {
                    message: e.to_string(),
                }
            })?
        } else {
            serde_json::from_slice(&decompressed_data)?
        };
        
        // Update shared pools
        self.shared_vector_pool.extend(batch.shared_vectors);
        self.reference_vectors.extend(batch.reference_vectors);
        
        // Convert back to full entries
        let entries = self.convert_from_compact_entries(&batch.entries)?;
        
        eprintln!("📦 Optimized deserialization: {} bytes -> {} entries in {:.2}ms",
                  data.len(),
                  entries.len(),
                  start_time.elapsed().as_secs_f64() * 1000.0);
        
        Ok(entries)
    }
    
    /// Progressive loading support for large batches
    pub fn serialize_progressive(
        &mut self,
        entries: &[EmbeddingEntry],
    ) -> OptimizedStorageResult<Vec<Vec<u8>>> {
        if !self.config.enable_progressive_loading {
            return Ok(vec![self.serialize_batch(entries)?]);
        }
        
        let mut chunks = Vec::new();
        let chunk_size = self.config.progressive_chunk_size;
        
        for chunk in entries.chunks(chunk_size) {
            let chunk_data = self.serialize_batch(chunk)?;
            chunks.push(chunk_data);
        }
        
        eprintln!("📦 Progressive serialization: {} entries -> {} chunks",
                  entries.len(), chunks.len());
        
        Ok(chunks)
    }
    
    /// Get storage statistics
    pub fn get_storage_stats(&self) -> StorageStats {
        StorageStats {
            shared_vectors_count: self.shared_vector_pool.len(),
            reference_vectors_count: self.reference_vectors.len(),
            config: self.config.clone(),
        }
    }
    
    // Private helper methods
    
    fn convert_to_compact_entries(
        &mut self,
        entries: &[EmbeddingEntry],
    ) -> OptimizedStorageResult<Vec<CompactEmbeddingEntry>> {
        let mut compact_entries = Vec::with_capacity(entries.len());
        
        for entry in entries {
            let compact_vector = if self.config.enable_delta_encoding {
                self.optimize_vector(&entry.vector, &entry.id)?
            } else {
                CompactVector::Raw(entry.vector.clone())
            };
            
            let compact_entry = CompactEmbeddingEntry {
                id: entry.id.clone(),
                vector: compact_vector,
                metadata: entry.metadata.clone().into(),
                created_at: entry.created_at,
                updated_at: entry.updated_at,
            };
            
            compact_entries.push(compact_entry);
        }
        
        Ok(compact_entries)
    }
    
    fn convert_from_compact_entries(
        &self,
        compact_entries: &[CompactEmbeddingEntry],
    ) -> OptimizedStorageResult<Vec<EmbeddingEntry>> {
        let mut entries = Vec::with_capacity(compact_entries.len());
        
        for compact_entry in compact_entries {
            let vector = self.reconstruct_vector(&compact_entry.vector)?;
            
            let entry = EmbeddingEntry {
                id: compact_entry.id.clone(),
                vector,
                metadata: compact_entry.metadata.clone().into(),
                created_at: compact_entry.created_at,
                updated_at: compact_entry.updated_at,
            };
            
            entries.push(entry);
        }
        
        Ok(entries)
    }
    
    fn optimize_vector(&mut self, vector: &[f32], entry_id: &str) -> OptimizedStorageResult<CompactVector> {
        // Check for identical vectors in pool
        let vector_hash = self.compute_vector_hash(vector);
        if let Some(pool_vector) = self.shared_vector_pool.get(&vector_hash) {
            if Self::vectors_identical(vector, pool_vector) {
                return Ok(CompactVector::Shared {
                    shared_id: vector_hash,
                });
            }
        }
        
        // Check for similar vectors for delta encoding
        if let Some(_compressor) = &mut self.vector_compressor {
            if let Ok(best_reference) = self.find_best_reference_vector(vector) {
                if let Some((ref_id, ref_vector)) = best_reference {
                    let delta: Vec<f32> = vector
                        .iter()
                        .zip(ref_vector.iter())
                        .map(|(&a, &b)| a - b)
                        .collect();
                    
                    return Ok(CompactVector::Reference {
                        reference_id: ref_id,
                        delta,
                    });
                }
            }
        }
        
        // Add to pools for future reference
        self.shared_vector_pool.insert(vector_hash.clone(), vector.to_vec());
        self.reference_vectors.insert(entry_id.to_string(), vector.to_vec());
        
        // Return raw vector if no optimizations apply
        Ok(CompactVector::Raw(vector.to_vec()))
    }
    
    fn reconstruct_vector(&self, compact_vector: &CompactVector) -> OptimizedStorageResult<Vec<f32>> {
        match compact_vector {
            CompactVector::Raw(vector) => Ok(vector.clone()),
            
            CompactVector::Compressed { data, dimension, params } => {
                self.decompress_vector(data, *dimension, params)
            }
            
            CompactVector::Reference { reference_id, delta } => {
                let ref_vector = self.reference_vectors.get(reference_id)
                    .ok_or_else(|| OptimizedStorageError::InvalidFormat {
                        message: format!("Reference vector {} not found", reference_id),
                    })?;
                
                let reconstructed: Vec<f32> = ref_vector
                    .iter()
                    .zip(delta.iter())
                    .map(|(&r, &d)| r + d)
                    .collect();
                
                Ok(reconstructed)
            }
            
            CompactVector::Shared { shared_id } => {
                let shared_vector = self.shared_vector_pool.get(shared_id)
                    .ok_or_else(|| OptimizedStorageError::InvalidFormat {
                        message: format!("Shared vector {} not found", shared_id),
                    })?;
                
                Ok(shared_vector.clone())
            }
        }
    }
    
    fn compress_data(&self, data: &[u8]) -> OptimizedStorageResult<Vec<u8>> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        Ok(compressed)
    }
    
    fn decompress_data(&self, data: &[u8]) -> OptimizedStorageResult<Vec<u8>> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        Ok(decompressed)
    }
    
    fn is_compressed_data(&self, data: &[u8]) -> bool {
        // Check for gzip magic number
        data.len() >= 2 && data[0] == 0x1f && data[1] == 0x8b
    }
    
    fn compute_vector_hash(&self, vector: &[f32]) -> String {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        for &value in vector {
            hasher.update(value.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
    
    fn vectors_identical(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && 
        a.iter().zip(b.iter()).all(|(&x, &y)| (x - y).abs() < f32::EPSILON)
    }
    
    fn find_best_reference_vector(&self, vector: &[f32]) -> OptimizedStorageResult<Option<(String, Vec<f32>)>> {
        let mut best_similarity = 0.0;
        let mut best_reference = None;
        
        for (id, ref_vector) in &self.reference_vectors {
            if ref_vector.len() != vector.len() {
                continue;
            }
            
            let similarity = self.cosine_similarity(vector, ref_vector);
            if similarity > best_similarity && similarity >= self.config.delta_similarity_threshold {
                best_similarity = similarity;
                best_reference = Some((id.clone(), ref_vector.clone()));
            }
        }
        
        Ok(best_reference)
    }
    
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        let dot_product: f32 = a.iter().zip(b.iter()).map(|(&x, &y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|&x| x * x).sum::<f32>().sqrt();
        
        if norm_a > 0.0 && norm_b > 0.0 {
            dot_product / (norm_a * norm_b)
        } else {
            0.0
        }
    }
    
    fn estimate_uncompressed_size(&self, entries: &[EmbeddingEntry]) -> usize {
        entries.iter().map(|e| e.memory_footprint()).sum()
    }
    
    fn decompress_vector(
        &self,
        _data: &[u8],
        _dimension: usize,
        _params: &CompactCompressionParams,
    ) -> OptimizedStorageResult<Vec<f32>> {
        // Placeholder - would implement actual decompression
        Err(OptimizedStorageError::InvalidFormat {
            message: "Vector decompression not implemented".to_string(),
        })
    }
}

/// Statistics about optimized storage operations
#[derive(Debug, Clone)]
pub struct StorageStats {
    /// Number of shared vectors in pool
    pub shared_vectors_count: usize,
    /// Number of reference vectors for delta encoding
    pub reference_vectors_count: usize,
    /// Current storage configuration
    pub config: OptimizedStorageConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_db::types::EmbeddingEntry;
    
    fn create_test_entry(id: &str, vector: Vec<f32>, file_path: &str) -> EmbeddingEntry {
        EmbeddingEntry::new(
            vector,
            file_path.to_string(),
            format!("chunk_{}", id),
            "test content",
            "test-model".to_string(),
        )
    }
    
    #[test]
    fn test_optimized_storage_engine_creation() {
        let config = OptimizedStorageConfig::default();
        let engine = OptimizedStorageEngine::new(config);
        assert!(engine.is_ok());
    }
    
    #[test]
    fn test_compact_metadata_conversion() {
        let original = crate::vector_db::types::EmbeddingMetadata::new(
            "/test/file.md".to_string(),
            "chunk_1".to_string(),
            "Test content preview".to_string(),
            100,
            "test-model".to_string(),
            "test content",
        );
        
        let compact: CompactEmbeddingMetadata = original.clone().into();
        let restored: crate::vector_db::types::EmbeddingMetadata = compact.into();
        
        assert_eq!(original.file_path, restored.file_path);
        assert_eq!(original.chunk_id, restored.chunk_id);
        assert_eq!(original.text_length, restored.text_length);
        assert_eq!(original.model_name, restored.model_name);
    }
    
    #[test]
    fn test_vector_hash_consistency() {
        let config = OptimizedStorageConfig::default();
        let engine = OptimizedStorageEngine::new(config).unwrap();
        
        let vector = vec![0.1, 0.2, 0.3, 0.4];
        let hash1 = engine.compute_vector_hash(&vector);
        let hash2 = engine.compute_vector_hash(&vector);
        
        assert_eq!(hash1, hash2);
        
        let different_vector = vec![0.1, 0.2, 0.3, 0.5];
        let hash3 = engine.compute_vector_hash(&different_vector);
        assert_ne!(hash1, hash3);
    }
    
    #[test]
    fn test_vectors_identical() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let vec3 = vec![1.0, 2.0, 3.1];
        
        assert!(OptimizedStorageEngine::vectors_identical(&vec1, &vec2));
        assert!(!OptimizedStorageEngine::vectors_identical(&vec1, &vec3));
    }
    
    #[test]
    fn test_cosine_similarity() {
        let config = OptimizedStorageConfig::default();
        let engine = OptimizedStorageEngine::new(config).unwrap();
        
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0]; // Identical
        let vec3 = vec![0.0, 1.0, 0.0]; // Orthogonal
        let vec4 = vec![0.5, 0.0, 0.0]; // Same direction, different magnitude
        
        assert!((engine.cosine_similarity(&vec1, &vec2) - 1.0).abs() < 1e-6);
        assert!((engine.cosine_similarity(&vec1, &vec3) - 0.0).abs() < 1e-6);
        assert!((engine.cosine_similarity(&vec1, &vec4) - 1.0).abs() < 1e-6);
    }
    
    #[test]
    fn test_serialization_deserialization_roundtrip() {
        let config = OptimizedStorageConfig::default();
        let mut engine = OptimizedStorageEngine::new(config).unwrap();
        
        let entries = vec![
            create_test_entry("1", vec![0.1, 0.2, 0.3], "/test/file1.md"),
            create_test_entry("2", vec![0.4, 0.5, 0.6], "/test/file2.md"),
        ];
        
        let serialized = engine.serialize_batch(&entries).unwrap();
        let deserialized = engine.deserialize_batch(&serialized).unwrap();
        
        assert_eq!(entries.len(), deserialized.len());
        assert_eq!(entries[0].id, deserialized[0].id);
        assert_eq!(entries[1].id, deserialized[1].id);
    }
}