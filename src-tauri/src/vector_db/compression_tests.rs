//! Comprehensive tests for vector compression and storage optimization
//! 
//! This module contains extensive tests for the storage compression and optimization
//! features implemented in issue #145.

#[cfg(test)]
mod tests {
    use super::super::compression::*;
    use super::super::optimized_storage::*;
    use super::super::types::EmbeddingEntry;
    
    /// Test vector for compression operations
    fn create_test_vector(size: usize, pattern: f32) -> Vec<f32> {
        (0..size).map(|i| pattern + (i as f32) * 0.1).collect()
    }
    
    /// Create test embedding entry
    fn create_test_entry(id: &str, vector: Vec<f32>) -> EmbeddingEntry {
        EmbeddingEntry::new(
            vector,
            "/test/file.md".to_string(),
            format!("chunk_{}", id),
            "test content for compression",
            "test-model".to_string(),
        )
    }
    
    #[test]
    fn test_vector_compression_config_validation() {
        // Test valid configurations
        let valid_configs = vec![
            VectorCompressionConfig {
                quantization_bits: 8,
                ..Default::default()
            },
            VectorCompressionConfig {
                quantization_bits: 16,
                ..Default::default()
            },
            VectorCompressionConfig {
                quantization_bits: 32,
                ..Default::default()
            },
        ];
        
        for config in valid_configs {
            let result = VectorCompressor::new(config);
            assert!(result.is_ok(), "Valid config should create compressor");
        }
        
        // Test invalid configurations
        let invalid_configs = vec![
            VectorCompressionConfig {
                quantization_bits: 7, // Invalid
                ..Default::default()
            },
            VectorCompressionConfig {
                quantization_bits: 12, // Invalid
                ..Default::default()
            },
            VectorCompressionConfig {
                delta_similarity_threshold: 1.5, // Invalid (> 1.0)
                ..Default::default()
            },
        ];
        
        for config in invalid_configs {
            let result = VectorCompressor::new(config);
            assert!(result.is_err(), "Invalid config should fail");
        }
    }
    
    #[test]
    fn test_8bit_quantization_accuracy() {
        let config = VectorCompressionConfig {
            algorithm: VectorCompressionAlgorithm::Quantized8Bit,
            quantization_bits: 8,
            ..Default::default()
        };
        
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        // Test with various vector patterns
        let test_vectors = vec![
            vec![0.0, 0.5, 1.0, -0.5, -1.0], // Range test
            vec![0.1; 10], // Constant value
            create_test_vector(50, 0.5), // Larger vector
            vec![-10.0, -5.0, 0.0, 5.0, 10.0], // Wide range
        ];
        
        for (i, original_vector) in test_vectors.iter().enumerate() {
            let compressed = compressor.compress_vector(original_vector, &format!("test_{}", i)).unwrap();
            let decompressed = compressor.decompress_vector(&compressed).unwrap();
            
            assert_eq!(original_vector.len(), decompressed.len(), 
                      "Vector dimensions should match");
            
            // Check that compression ratio is reasonable (should be < 1.0 for 8-bit)
            assert!(compressed.compression_ratio < 1.0, 
                   "8-bit quantization should achieve compression");
            
            // Check quantization accuracy (should be close but not exact)
            let max_error = original_vector.iter()
                .zip(decompressed.iter())
                .map(|(orig, decomp)| (orig - decomp).abs())
                .fold(0.0, f32::max);
            
            // 8-bit quantization should have some error but not too much
            assert!(max_error < 0.5, 
                   "8-bit quantization error should be reasonable: {} for vector {}", 
                   max_error, i);
        }
    }
    
    #[test]
    fn test_16bit_quantization_accuracy() {
        let config = VectorCompressionConfig {
            algorithm: VectorCompressionAlgorithm::Quantized16Bit,
            quantization_bits: 16,
            ..Default::default()
        };
        
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        let test_vector = vec![0.123, -0.456, 0.789, -0.987, 0.543];
        let compressed = compressor.compress_vector(&test_vector, "test").unwrap();
        let decompressed = compressor.decompress_vector(&compressed).unwrap();
        
        // 16-bit should be more accurate than 8-bit
        let max_error = test_vector.iter()
            .zip(decompressed.iter())
            .map(|(orig, decomp)| (orig - decomp).abs())
            .fold(0.0, f32::max);
        
        assert!(max_error < 0.01, 
               "16-bit quantization should be very accurate: error = {}", max_error);
        
        // Should still achieve some compression
        assert!(compressed.compression_ratio < 1.0, 
               "16-bit should still compress");
    }
    
    #[test]
    fn test_delta_compression() {
        let config = VectorCompressionConfig {
            algorithm: VectorCompressionAlgorithm::DeltaQuantized,
            enable_delta_compression: true,
            delta_similarity_threshold: 0.7,
            ..Default::default()
        };
        
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        // Create reference vector
        let reference_vector = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        compressor.add_reference_vector("ref1".to_string(), reference_vector.clone());
        
        // Create similar vector for delta compression
        let similar_vector = vec![1.1, 2.1, 3.1, 4.1, 5.1];
        
        let compressed = compressor.compress_vector(&similar_vector, "test_delta").unwrap();
        let decompressed = compressor.decompress_vector(&compressed).unwrap();
        
        // Check reconstruction accuracy
        for (orig, decomp) in similar_vector.iter().zip(decompressed.iter()) {
            assert!((orig - decomp).abs() < 0.2, 
                   "Delta compression should reconstruct accurately: {} vs {}", orig, decomp);
        }
        
        // Delta compression should achieve good compression ratios for similar vectors
        assert!(compressed.compression_ratio < 0.8, 
               "Delta compression should be very effective for similar vectors");
    }
    
    #[test]
    fn test_compression_performance() {
        let config = VectorCompressionConfig::default();
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        // Test with various vector sizes
        let sizes = vec![128, 384, 768, 1536]; // Common embedding sizes
        
        for size in sizes {
            let vector = create_test_vector(size, 0.5);
            
            let start_time = std::time::Instant::now();
            let compressed = compressor.compress_vector(&vector, &format!("perf_test_{}", size)).unwrap();
            let compression_time = start_time.elapsed();
            
            let start_time = std::time::Instant::now();
            let _decompressed = compressor.decompress_vector(&compressed).unwrap();
            let decompression_time = start_time.elapsed();
            
            // Performance requirements: should be fast for real-time use
            assert!(compression_time.as_millis() < 100, 
                   "Compression should be fast (size {}): {}ms", size, compression_time.as_millis());
            assert!(decompression_time.as_millis() < 50, 
                   "Decompression should be very fast (size {}): {}ms", size, decompression_time.as_millis());
            
            eprintln!("Size {}: compress={}ms, decompress={}ms, ratio={:.3}", 
                     size, compression_time.as_millis(), decompression_time.as_millis(), 
                     compressed.compression_ratio);
        }
    }
    
    #[test]
    fn test_batch_compression() {
        let config = VectorCompressionConfig {
            enable_batch_compression: true,
            min_batch_size: 3,
            ..Default::default()
        };
        
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        let vectors = vec![
            ("vec1".to_string(), create_test_vector(10, 1.0)),
            ("vec2".to_string(), create_test_vector(10, 2.0)),
            ("vec3".to_string(), create_test_vector(10, 3.0)),
            ("vec4".to_string(), create_test_vector(10, 4.0)),
        ];
        
        let compressed_batch = compressor.compress_batch(&vectors).unwrap();
        
        assert_eq!(compressed_batch.len(), vectors.len(), 
                  "Batch compression should return same number of vectors");
        
        // Verify each vector can be decompressed correctly
        for (i, compressed) in compressed_batch.iter().enumerate() {
            let decompressed = compressor.decompress_vector(compressed).unwrap();
            assert_eq!(decompressed.len(), vectors[i].1.len(), 
                      "Decompressed vector size should match original");
        }
    }
    
    #[test]
    fn test_optimized_storage_serialization() {
        let config = OptimizedStorageConfig {
            use_compact_field_names: true,
            enable_batch_compression: true,
            min_batch_size: 2,
            ..Default::default()
        };
        
        let mut engine = OptimizedStorageEngine::new(config).unwrap();
        
        let entries = vec![
            create_test_entry("1", create_test_vector(10, 1.0)),
            create_test_entry("2", create_test_vector(10, 2.0)),
            create_test_entry("3", create_test_vector(10, 3.0)),
        ];
        
        // Test serialization
        let serialized = engine.serialize_batch(&entries).unwrap();
        assert!(!serialized.is_empty(), "Serialized data should not be empty");
        
        // Test deserialization
        let deserialized = engine.deserialize_batch(&serialized).unwrap();
        assert_eq!(entries.len(), deserialized.len(), 
                  "Deserialized should have same number of entries");
        
        // Verify entry content
        for (orig, deser) in entries.iter().zip(deserialized.iter()) {
            assert_eq!(orig.id, deser.id, "Entry IDs should match");
            assert_eq!(orig.metadata.file_path, deser.metadata.file_path, "File paths should match");
            // Vector comparison with tolerance (due to compression)
            assert_eq!(orig.vector.len(), deser.vector.len(), "Vector lengths should match");
        }
    }
    
    #[test]
    fn test_storage_compression_ratios() {
        let configs = vec![
            ("uncompressed", OptimizedStorageConfig {
                use_compact_field_names: false,
                enable_batch_compression: false,
                enable_delta_encoding: false,
                ..Default::default()
            }),
            ("compact_fields", OptimizedStorageConfig {
                use_compact_field_names: true,
                enable_batch_compression: false,
                enable_delta_encoding: false,
                ..Default::default()
            }),
            ("batch_compression", OptimizedStorageConfig {
                use_compact_field_names: true,
                enable_batch_compression: true,
                enable_delta_encoding: false,
                min_batch_size: 2,
                ..Default::default()
            }),
            ("full_optimization", OptimizedStorageConfig {
                use_compact_field_names: true,
                enable_batch_compression: true,
                enable_delta_encoding: true,
                min_batch_size: 2,
                delta_similarity_threshold: 0.8,
                ..Default::default()
            }),
        ];
        
        let entries = (0..10)
            .map(|i| create_test_entry(&format!("entry_{}", i), create_test_vector(50, i as f32)))
            .collect::<Vec<_>>();
        
        let original_size = entries.iter().map(|e| e.memory_footprint()).sum::<usize>();
        
        for (name, config) in configs {
            let mut engine = OptimizedStorageEngine::new(config).unwrap();
            let serialized = engine.serialize_batch(&entries).unwrap();
            let compression_ratio = serialized.len() as f32 / original_size as f32;
            
            eprintln!("{}: {} bytes -> {} bytes (ratio: {:.3})", 
                     name, original_size, serialized.len(), compression_ratio);
            
            // Verify we can deserialize correctly
            let deserialized = engine.deserialize_batch(&serialized).unwrap();
            assert_eq!(entries.len(), deserialized.len(), 
                      "Deserialized count should match for config: {}", name);
        }
    }
    
    #[test]
    fn test_progressive_loading() {
        let config = OptimizedStorageConfig {
            enable_progressive_loading: true,
            progressive_chunk_size: 3,
            ..Default::default()
        };
        
        let mut engine = OptimizedStorageEngine::new(config).unwrap();
        
        let entries = (0..10)
            .map(|i| create_test_entry(&format!("prog_{}", i), create_test_vector(20, i as f32)))
            .collect::<Vec<_>>();
        
        let chunks = engine.serialize_progressive(&entries).unwrap();
        
        // Should create multiple chunks
        assert!(chunks.len() > 1, "Should create multiple chunks for progressive loading");
        assert!(chunks.len() <= 4, "Should not create too many chunks"); // 10 entries / 3 per chunk = ~4 chunks
        
        // Verify each chunk can be deserialized
        let mut total_deserialized = 0;
        for (i, chunk_data) in chunks.iter().enumerate() {
            let chunk_entries = engine.deserialize_batch(chunk_data).unwrap();
            total_deserialized += chunk_entries.len();
            
            eprintln!("Chunk {}: {} entries, {} bytes", i, chunk_entries.len(), chunk_data.len());
        }
        
        assert_eq!(total_deserialized, entries.len(), 
                  "Total deserialized entries should match original count");
    }
    
    #[test]
    fn test_memory_efficiency() {
        // Test that compression actually saves memory compared to storing raw data
        let entries = (0..100)
            .map(|i| create_test_entry(&format!("mem_test_{}", i), create_test_vector(100, i as f32 * 0.1)))
            .collect::<Vec<_>>();
        
        // Calculate raw memory usage
        let raw_memory = entries.iter().map(|e| e.memory_footprint()).sum::<usize>();
        
        // Test compressed storage
        let config = OptimizedStorageConfig {
            use_compact_field_names: true,
            enable_batch_compression: true,
            enable_delta_encoding: true,
            min_batch_size: 10,
            ..Default::default()
        };
        
        let mut engine = OptimizedStorageEngine::new(config).unwrap();
        let compressed_data = engine.serialize_batch(&entries).unwrap();
        
        let compression_ratio = compressed_data.len() as f32 / raw_memory as f32;
        
        eprintln!("Memory efficiency test: {} bytes -> {} bytes (ratio: {:.3})", 
                 raw_memory, compressed_data.len(), compression_ratio);
        
        // Should achieve significant compression
        assert!(compression_ratio < 0.8, 
               "Should achieve significant compression: actual ratio = {:.3}", compression_ratio);
        
        // Verify correctness
        let deserialized = engine.deserialize_batch(&compressed_data).unwrap();
        assert_eq!(entries.len(), deserialized.len(), 
                  "Decompressed entries count should match");
    }
    
    #[test]
    fn test_error_handling() {
        // Test various error conditions
        
        // Invalid quantization bits
        let invalid_config = VectorCompressionConfig {
            quantization_bits: 7, // Invalid
            ..Default::default()
        };
        assert!(VectorCompressor::new(invalid_config).is_err());
        
        // Empty vector compression
        let config = VectorCompressionConfig::default();
        let mut compressor = VectorCompressor::new(config).unwrap();
        let empty_vector: Vec<f32> = vec![];
        let result = compressor.compress_vector(&empty_vector, "empty_test");
        // Should handle empty vectors gracefully (either succeed or fail cleanly)
        match result {
            Ok(compressed) => {
                let decompressed = compressor.decompress_vector(&compressed).unwrap();
                assert_eq!(decompressed.len(), 0);
            }
            Err(_) => {
                // Also acceptable to fail on empty vectors
            }
        }
        
        // Test corrupted compressed data
        let config = VectorCompressionConfig::default();
        let mut compressor = VectorCompressor::new(config).unwrap();
        let test_vector = vec![1.0, 2.0, 3.0];
        let mut compressed = compressor.compress_vector(&test_vector, "corruption_test").unwrap();
        
        // Corrupt the data
        compressed.data[0] = compressed.data[0].wrapping_add(1);
        
        let result = compressor.decompress_vector(&compressed);
        // Should handle corruption gracefully by returning an error
        assert!(result.is_err(), "Should detect corrupted data");
    }
    
    /// Integration test combining multiple compression features
    #[test]
    fn test_compression_integration() {
        let config = VectorCompressionConfig {
            algorithm: VectorCompressionAlgorithm::DeltaQuantized,
            enable_delta_compression: true,
            quantization_bits: 8,
            enable_batch_compression: true,
            min_batch_size: 3,
            delta_similarity_threshold: 0.85,
        };
        
        let mut compressor = VectorCompressor::new(config).unwrap();
        
        // Create a mix of similar and dissimilar vectors
        let base_vector = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let similar_vectors = (0..5)
            .map(|i| {
                base_vector.iter().map(|&x| x + (i as f32) * 0.1).collect::<Vec<f32>>()
            })
            .collect::<Vec<_>>();
        
        let different_vector = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        
        // Process vectors
        let mut all_compressed = Vec::new();
        
        // Add base vector as reference
        let compressed = compressor.compress_vector(&base_vector, "base").unwrap();
        all_compressed.push((base_vector.clone(), compressed));
        
        // Add similar vectors (should use delta compression)
        for (i, vector) in similar_vectors.iter().enumerate() {
            let compressed = compressor.compress_vector(vector, &format!("similar_{}", i)).unwrap();
            all_compressed.push((vector.clone(), compressed));
        }
        
        // Add different vector
        let compressed = compressor.compress_vector(&different_vector, "different").unwrap();
        all_compressed.push((different_vector.clone(), compressed));
        
        // Verify all vectors can be decompressed correctly
        for (original, compressed) in &all_compressed {
            let decompressed = compressor.decompress_vector(compressed).unwrap();
            
            for (orig, decomp) in original.iter().zip(decompressed.iter()) {
                assert!((orig - decomp).abs() < 0.3, 
                       "Integration test: vector should decompress accurately: {} vs {}", orig, decomp);
            }
        }
        
        // Check compression ratios
        let total_original_size: usize = all_compressed.iter()
            .map(|(orig, _)| orig.len() * 4)
            .sum();
        let total_compressed_size: usize = all_compressed.iter()
            .map(|(_, comp)| comp.data.len())
            .sum();
        
        let overall_ratio = total_compressed_size as f32 / total_original_size as f32;
        
        eprintln!("Integration test compression ratio: {:.3}", overall_ratio);
        assert!(overall_ratio < 1.0, "Overall compression should be achieved");
    }
}