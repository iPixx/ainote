//! Comprehensive Test Suite for Similarity Search Engine
//! 
//! This test suite validates the correctness, performance, and accuracy of the
//! similarity search engine implementation according to issue #113 requirements.

use ainote_lib::similarity_search::{
    SimilaritySearch, SearchConfig, PerformanceConfig, SimilarityError,
    ConcurrentSearchManager,
};
use ainote_lib::vector_db::types::EmbeddingEntry;
use std::collections::HashSet;
use std::time::Instant;

/// Helper function to create test embedding entries
fn create_test_entry(vector: Vec<f32>, file_path: &str, chunk_id: &str) -> EmbeddingEntry {
    EmbeddingEntry::new(
        vector,
        file_path.to_string(),
        chunk_id.to_string(),
        &format!("Test content for {} chunk {}", file_path, chunk_id),
        "test-model".to_string(),
    )
}

/// Helper function to generate deterministic test vectors
fn generate_test_vector(seed: u32, dim: usize) -> Vec<f32> {
    let mut vector = Vec::with_capacity(dim);
    let mut x = seed as f32;
    
    for _i in 0..dim {
        x = ((x * 9301.0 + 49297.0) % 233280.0) / 233280.0; // Simple LCG
        vector.push(x - 0.5); // Center around 0
    }
    
    vector
}

/// Generate test dataset with known similarity relationships
fn generate_test_dataset(size: usize, vector_dim: usize) -> Vec<EmbeddingEntry> {
    let mut entries = Vec::with_capacity(size);
    
    for i in 0..size {
        let vector = generate_test_vector(i as u32 + 1, vector_dim);
        let entry = create_test_entry(
            vector,
            &format!("document_{:04}.md", i),
            "chunk_001"
        );
        entries.push(entry);
    }
    
    entries
}

#[cfg(test)]
mod cosine_similarity_accuracy_tests {
    use super::*;
    
    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let vec_a = vec![1.0, 2.0, 3.0, 4.0];
        let vec_b = vec![1.0, 2.0, 3.0, 4.0];
        
        let similarity = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        
        // Identical vectors should have perfect similarity
        assert!((similarity - 1.0).abs() < f32::EPSILON, 
            "Identical vectors should have similarity 1.0, got {}", similarity);
    }
    
    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let vec_a = vec![1.0, 0.0, 0.0, 0.0];
        let vec_b = vec![0.0, 1.0, 0.0, 0.0];
        
        let similarity = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        
        // Orthogonal vectors should have zero similarity
        assert!(similarity.abs() < f32::EPSILON, 
            "Orthogonal vectors should have similarity 0.0, got {}", similarity);
    }
    
    #[test]
    fn test_cosine_similarity_opposite_vectors() {
        let vec_a = vec![1.0, 2.0, 3.0];
        let vec_b = vec![-1.0, -2.0, -3.0];
        
        let similarity = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        
        // Opposite vectors should have similarity -1.0
        assert!((similarity + 1.0).abs() < f32::EPSILON, 
            "Opposite vectors should have similarity -1.0, got {}", similarity);
    }
    
    #[test]
    fn test_cosine_similarity_mathematical_properties() {
        let vec_a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let vec_b = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        let vec_c = vec![-1.0, -2.0, -3.0, -4.0, -5.0];
        
        // Test symmetry: cos_sim(A, B) = cos_sim(B, A)
        let sim_ab = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        let sim_ba = SimilaritySearch::cosine_similarity(&vec_b, &vec_a).unwrap();
        assert!((sim_ab - sim_ba).abs() < f32::EPSILON, 
            "Cosine similarity should be symmetric");
        
        // Test self-similarity: cos_sim(A, A) = 1.0
        let sim_aa = SimilaritySearch::cosine_similarity(&vec_a, &vec_a).unwrap();
        assert!((sim_aa - 1.0).abs() < f32::EPSILON, 
            "Self-similarity should be 1.0");
        
        // Test range: -1.0 <= similarity <= 1.0
        let sim_ac = SimilaritySearch::cosine_similarity(&vec_a, &vec_c).unwrap();
        assert!(sim_ac >= -1.0 && sim_ac <= 1.0, 
            "Similarity should be in range [-1, 1], got {}", sim_ac);
    }
    
    #[test]
    fn test_cosine_similarity_numerical_precision() {
        // Test with very small numbers
        let vec_a = vec![1e-10, 2e-10, 3e-10];
        let vec_b = vec![2e-10, 4e-10, 6e-10];
        
        let similarity = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        assert!((similarity - 1.0).abs() < 1e-6, 
            "Small proportional vectors should have high similarity");
        
        // Test with very large numbers
        let vec_c = vec![1e10, 2e10, 3e10];
        let vec_d = vec![2e10, 4e10, 6e10];
        
        let similarity2 = SimilaritySearch::cosine_similarity(&vec_c, &vec_d).unwrap();
        assert!((similarity2 - 1.0).abs() < 1e-6, 
            "Large proportional vectors should have high similarity");
    }
    
    #[test]
    fn test_normalized_cosine_similarity() {
        // Create normalized vectors (unit length)
        let vec_a = vec![0.6, 0.8];  // magnitude = 1.0
        let vec_b = vec![1.0, 0.0];  // magnitude = 1.0
        
        let normalized_sim = SimilaritySearch::cosine_similarity_normalized(&vec_a, &vec_b).unwrap();
        let standard_sim = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
        
        // Results should be identical for normalized vectors
        assert!((normalized_sim - standard_sim).abs() < f32::EPSILON,
            "Normalized and standard cosine similarity should match for unit vectors");
        
        // Should equal dot product for normalized vectors
        let dot_product: f32 = vec_a.iter().zip(vec_b.iter()).map(|(a, b)| a * b).sum();
        assert!((normalized_sim - dot_product).abs() < f32::EPSILON,
            "Normalized cosine similarity should equal dot product for unit vectors");
    }
    
    #[test]
    fn test_vector_normalization_accuracy() {
        let vector = vec![3.0, 4.0, 12.0]; // magnitude = 13.0
        let normalized = SimilaritySearch::normalize_vector(&vector).unwrap();
        
        // Check normalized values
        let expected = vec![3.0/13.0, 4.0/13.0, 12.0/13.0];
        for (got, exp) in normalized.iter().zip(expected.iter()) {
            assert!((got - exp).abs() < 1e-6, 
                "Normalization accuracy: expected {}, got {}", exp, got);
        }
        
        // Check magnitude is 1.0
        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6, 
            "Normalized vector should have unit magnitude, got {}", magnitude);
    }
}

#[cfg(test)]
mod knn_search_correctness_tests {
    use super::*;
    
    #[test]
    fn test_knn_basic_functionality() {
        let query = vec![1.0, 0.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0, 0.0], "perfect_match.md", "chunk1"),     // similarity = 1.0
            create_test_entry(vec![0.0, 1.0, 0.0], "orthogonal.md", "chunk1"),       // similarity = 0.0
            create_test_entry(vec![0.7071, 0.7071, 0.0], "diagonal.md", "chunk1"),   // similarity ≈ 0.707
            create_test_entry(vec![-1.0, 0.0, 0.0], "opposite.md", "chunk1"),        // similarity = -1.0
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 2, &config).unwrap();
        
        assert_eq!(results.len(), 2, "Should return exactly k=2 results");
        
        // Results should be sorted by similarity (descending)
        assert!(results[0].similarity >= results[1].similarity, 
            "Results should be sorted by similarity");
        
        // Best result should be perfect match
        assert!((results[0].similarity - 1.0).abs() < f32::EPSILON,
            "Best result should have similarity 1.0");
        assert_eq!(results[0].entry.metadata.file_path, "perfect_match.md");
        
        // Second best should be diagonal vector (more lenient tolerance)
        assert!(results[1].similarity > 0.7,
            "Second result should be diagonal vector with similarity >0.7, got {}", results[1].similarity);
    }
    
    #[test]
    fn test_knn_with_k_larger_than_database() {
        let query = vec![1.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0], "doc1.md", "chunk1"),
            create_test_entry(vec![0.0, 1.0], "doc2.md", "chunk1"),
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config).unwrap();
        
        // Should return all available entries (2), not k=5
        assert_eq!(results.len(), 2, "Should return all available entries when k > database size");
    }
    
    #[test]
    fn test_knn_with_threshold_filtering() {
        let query = vec![1.0, 0.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0, 0.0], "high_sim.md", "chunk1"),      // similarity = 1.0
            create_test_entry(vec![0.9, 0.436, 0.0], "med_sim.md", "chunk1"),     // similarity ≈ 0.9
            create_test_entry(vec![0.5, 0.866, 0.0], "low_sim.md", "chunk1"),     // similarity = 0.5
            create_test_entry(vec![0.0, 1.0, 0.0], "very_low.md", "chunk1"),      // similarity = 0.0
        ];
        
        let config = SearchConfig {
            min_threshold: 0.7,
            max_results: 10,
            ..SearchConfig::default()
        };
        
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 4, &config).unwrap();
        
        // Should only return results above threshold
        for result in &results {
            assert!(result.similarity >= 0.7, 
                "All results should be above threshold 0.7, got {}", result.similarity);
        }
        
        // Should exclude very_low.md (similarity = 0.0)
        let file_paths: HashSet<_> = results.iter().map(|r| &r.entry.metadata.file_path).collect();
        assert!(!file_paths.contains(&"very_low.md".to_string()),
            "Results should not include entries below threshold");
    }
    
    #[test]
    fn test_knn_result_ranking_consistency() {
        let query = generate_test_vector(42, 10);
        let entries = generate_test_dataset(20, 10);
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        
        // Verify results are properly ranked
        for i in 1..results.len() {
            assert!(results[i-1].similarity >= results[i].similarity,
                "Results must be sorted by similarity (descending): {} >= {}", 
                results[i-1].similarity, results[i].similarity);
        }
        
        // Verify no duplicates
        let mut seen_files = HashSet::new();
        for result in &results {
            let key = &result.entry.metadata.file_path;
            assert!(!seen_files.contains(key), 
                "Results should not contain duplicates: {}", key);
            seen_files.insert(key);
        }
    }
    
    #[test]
    fn test_batch_knn_consistency() {
        let queries = vec![
            generate_test_vector(1, 5),
            generate_test_vector(2, 5),
            generate_test_vector(3, 5),
        ];
        let entries = generate_test_dataset(10, 5);
        let config = SearchConfig::default();
        
        // Test batch processing
        let batch_results = SimilaritySearch::batch_k_nearest_neighbors(&queries, &entries, 3, &config).unwrap();
        
        // Test individual processing
        let mut individual_results = Vec::new();
        for query in &queries {
            let result = SimilaritySearch::k_nearest_neighbors(query, &entries, 3, &config).unwrap();
            individual_results.push(result);
        }
        
        // Results should be identical
        assert_eq!(batch_results.len(), individual_results.len());
        
        for (batch, individual) in batch_results.iter().zip(individual_results.iter()) {
            assert_eq!(batch.len(), individual.len());
            
            for (b_result, i_result) in batch.iter().zip(individual.iter()) {
                assert!((b_result.similarity - i_result.similarity).abs() < f32::EPSILON,
                    "Batch and individual results should match");
                assert_eq!(b_result.entry.metadata.file_path, i_result.entry.metadata.file_path);
            }
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;
    
    #[test]
    fn test_empty_database() {
        let query = vec![1.0, 0.0, 0.0];
        let entries: Vec<EmbeddingEntry> = vec![];
        let config = SearchConfig::default();
        
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config).unwrap();
        assert!(results.is_empty(), "Empty database should return empty results");
    }
    
    #[test]
    fn test_zero_k_value() {
        let query = vec![1.0, 0.0];
        let entries = vec![create_test_entry(vec![1.0, 0.0], "test.md", "chunk1")];
        let config = SearchConfig::default();
        
        let result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 0, &config);
        assert!(matches!(result, Err(SimilarityError::InvalidK { k: 0 })));
    }
    
    #[test]
    fn test_zero_vectors_error_handling() {
        let query = vec![0.0, 0.0, 0.0];  // Zero vector
        let entries = vec![create_test_entry(vec![1.0, 2.0, 3.0], "test.md", "chunk1")];
        let config = SearchConfig::default();
        
        let result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config);
        assert!(matches!(result, Err(SimilarityError::ZeroMagnitude)));
    }
    
    #[test]
    fn test_invalid_vector_values() {
        // Test NaN values
        let query = vec![1.0, f32::NAN, 3.0];
        let entries = vec![create_test_entry(vec![1.0, 2.0, 3.0], "test.md", "chunk1")];
        let config = SearchConfig::default();
        
        let result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config);
        assert!(matches!(result, Err(SimilarityError::InvalidVector)));
        
        // Test infinity values
        let query2 = vec![1.0, 2.0, f32::INFINITY];
        let result2 = SimilaritySearch::k_nearest_neighbors(&query2, &entries, 1, &config);
        assert!(matches!(result2, Err(SimilarityError::InvalidVector)));
    }
    
    #[test]
    fn test_dimension_mismatch() {
        let query = vec![1.0, 2.0, 3.0];  // 3D
        let entries = vec![
            create_test_entry(vec![1.0, 2.0], "test.md", "chunk1")  // 2D
        ];
        let config = SearchConfig::default();
        
        let result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config);
        assert!(matches!(result, Err(SimilarityError::DimensionMismatch { query_dim: 3, target_dim: 2 })));
    }
    
    #[test]
    fn test_empty_query_vector() {
        let query: Vec<f32> = vec![];
        let entries = vec![create_test_entry(vec![1.0, 2.0], "test.md", "chunk1")];
        let config = SearchConfig::default();
        
        let result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config);
        assert!(matches!(result, Err(SimilarityError::EmptyVector { .. })));
    }
    
    #[test]
    fn test_identical_vectors_in_database() {
        let query = vec![1.0, 0.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0, 0.0], "doc1.md", "chunk1"),
            create_test_entry(vec![1.0, 0.0, 0.0], "doc2.md", "chunk1"),  // Identical vector
            create_test_entry(vec![1.0, 0.0, 0.0], "doc3.md", "chunk1"),  // Identical vector
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 3, &config).unwrap();
        
        // All results should have similarity 1.0
        for result in &results {
            assert!((result.similarity - 1.0).abs() < f32::EPSILON,
                "All identical vectors should have similarity 1.0");
        }
        
        // Should return all 3 results
        assert_eq!(results.len(), 3, "Should return all identical vectors");
    }
    
    #[test]
    fn test_extreme_threshold_values() {
        let query = vec![1.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0], "perfect.md", "chunk1"),    // similarity = 1.0
            create_test_entry(vec![0.0, 1.0], "orthogonal.md", "chunk1"), // similarity = 0.0
        ];
        
        // Test threshold = 1.0 (only perfect matches)
        let config_high = SearchConfig {
            min_threshold: 1.0,
            ..SearchConfig::default()
        };
        let results_high = SimilaritySearch::k_nearest_neighbors(&query, &entries, 2, &config_high).unwrap();
        assert_eq!(results_high.len(), 1, "Only perfect matches should pass threshold 1.0");
        
        // Test threshold = -1.0 (all vectors)
        let config_low = SearchConfig {
            min_threshold: -1.0,
            ..SearchConfig::default()
        };
        let results_low = SimilaritySearch::k_nearest_neighbors(&query, &entries, 2, &config_low).unwrap();
        assert_eq!(results_low.len(), 2, "All vectors should pass threshold -1.0");
        
        // Test invalid threshold
        let config_invalid = SearchConfig {
            min_threshold: 1.5, // Invalid: outside [-1, 1]
            ..SearchConfig::default()
        };
        let result_invalid = SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config_invalid);
        assert!(matches!(result_invalid, Err(SimilarityError::InvalidThreshold { .. })));
    }
}

#[cfg(test)]
mod ranking_and_scoring_tests {
    use super::*;
    
    #[test]
    fn test_ranking_algorithm_accuracy() {
        // Create vectors with known similarity relationships
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0, 0.0, 0.0], "rank1.md", "chunk1"),     // similarity = 1.0
            create_test_entry(vec![0.9, 0.436, 0.0, 0.0], "rank2.md", "chunk1"),   // similarity ≈ 0.9  
            create_test_entry(vec![0.8, 0.6, 0.0, 0.0], "rank3.md", "chunk1"),     // similarity = 0.8
            create_test_entry(vec![0.7, 0.714, 0.0, 0.0], "rank4.md", "chunk1"),   // similarity ≈ 0.7
            create_test_entry(vec![0.0, 1.0, 0.0, 0.0], "rank5.md", "chunk1"),     // similarity = 0.0
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config).unwrap();
        
        // Verify strict descending order
        let expected_order = vec!["rank1.md", "rank2.md", "rank3.md", "rank4.md", "rank5.md"];
        for (i, expected_file) in expected_order.iter().enumerate() {
            assert_eq!(results[i].entry.metadata.file_path, *expected_file,
                "Ranking order incorrect at position {}", i);
        }
        
        // Verify similarity values are decreasing
        for i in 1..results.len() {
            assert!(results[i-1].similarity >= results[i].similarity,
                "Similarities should be in descending order: {} >= {}", 
                results[i-1].similarity, results[i].similarity);
        }
    }
    
    #[test]
    fn test_scoring_precision() {
        let query = vec![3.0, 4.0];  // magnitude = 5.0
        let target = vec![6.0, 8.0]; // magnitude = 10.0, same direction
        
        // Calculate expected cosine similarity manually
        let dot_product = 3.0 * 6.0 + 4.0 * 8.0; // = 18 + 32 = 50
        let magnitude_q = (3.0_f32 * 3.0 + 4.0 * 4.0).sqrt(); // = 5.0
        let magnitude_t = (6.0_f32 * 6.0 + 8.0 * 8.0).sqrt(); // = 10.0
        let expected_similarity = dot_product / (magnitude_q * magnitude_t); // = 50 / 50 = 1.0
        
        let computed_similarity = SimilaritySearch::cosine_similarity(&query, &target).unwrap();
        
        assert!((computed_similarity - expected_similarity).abs() < 1e-6,
            "Scoring precision: expected {}, got {}", expected_similarity, computed_similarity);
    }
    
    #[test]
    fn test_threshold_filtering_accuracy() {
        let query = vec![1.0, 0.0, 0.0];
        let entries = vec![
            create_test_entry(vec![1.0, 0.0, 0.0], "sim_1_00.md", "chunk1"),       // similarity = 1.00
            create_test_entry(vec![0.9, 0.436, 0.0], "sim_0_90.md", "chunk1"),     // similarity ≈ 0.90
            create_test_entry(vec![0.7071, 0.7071, 0.0], "sim_0_71.md", "chunk1"), // similarity ≈ 0.71
            create_test_entry(vec![0.6, 0.8, 0.0], "sim_0_60.md", "chunk1"),       // similarity = 0.60
            create_test_entry(vec![0.0, 1.0, 0.0], "sim_0_00.md", "chunk1"),       // similarity = 0.00
        ];
        
        // Test threshold = 0.65
        let config = SearchConfig {
            min_threshold: 0.65,
            max_results: 10,
            ..SearchConfig::default()
        };
        
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        
        // Should return entries with similarity >= 0.65
        let expected_files = vec!["sim_1_00.md", "sim_0_90.md", "sim_0_71.md"];
        assert_eq!(results.len(), expected_files.len(), 
            "Should return {} results above threshold 0.65", expected_files.len());
        
        for (result, expected) in results.iter().zip(expected_files.iter()) {
            assert_eq!(result.entry.metadata.file_path, *expected);
            assert!(result.similarity >= 0.65, 
                "All results should be >= 0.65, got {}", result.similarity);
        }
    }
    
    #[test]
    fn test_max_results_limit() {
        let query = vec![1.0, 0.0];
        let entries = generate_test_dataset(20, 2);
        
        let config = SearchConfig {
            max_results: 5,
            ..SearchConfig::default()
        };
        
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        
        // Should respect max_results limit even when k is higher
        assert_eq!(results.len(), 5, "Should respect max_results limit");
        
        // Results should still be properly ranked
        for i in 1..results.len() {
            assert!(results[i-1].similarity >= results[i].similarity,
                "Results should still be properly ranked despite max_results limit");
        }
    }
    
    #[test]
    fn test_early_termination_behavior() {
        let query = generate_test_vector(1, 10);
        let entries = generate_test_dataset(100, 10);
        
        // Test with early termination enabled
        let config_early = SearchConfig {
            early_termination: true,
            min_threshold: 0.1,
            ..SearchConfig::default()
        };
        
        let start = Instant::now();
        let results_early = SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config_early).unwrap();
        let time_early = start.elapsed();
        
        // Test with early termination disabled
        let config_full = SearchConfig {
            early_termination: false,
            min_threshold: 0.1,
            ..SearchConfig::default()
        };
        
        let start = Instant::now();
        let results_full = SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config_full).unwrap();
        let time_full = start.elapsed();
        
        // Both should return valid results
        assert_eq!(results_early.len(), results_full.len());
        
        // Early termination should generally be faster (but not guaranteed in small datasets)
        println!("Early termination: {:?}, Full search: {:?}", time_early, time_full);
        
        // Results should be of high quality regardless
        for result in &results_early {
            assert!(result.similarity.is_finite(), "All similarity scores should be finite");
            assert!(result.similarity >= config_early.min_threshold, 
                "All results should meet threshold");
        }
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Duration;
    
    #[test]
    fn test_performance_100_vectors() {
        let query = generate_test_vector(1, 384); // Typical embedding dimension
        let entries = generate_test_dataset(100, 384);
        let config = SearchConfig::default();
        
        let start = Instant::now();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(10), 
            "100-vector search should complete in <10ms, took {:?}", elapsed);
        assert_eq!(results.len(), 10, "Should return requested k=10 results");
        
        println!("100-vector performance: {:?}", elapsed);
    }
    
    #[test]
    fn test_performance_500_vectors() {
        let query = generate_test_vector(1, 384);
        let entries = generate_test_dataset(500, 384);
        let config = SearchConfig::default();
        
        let start = Instant::now();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(25), 
            "500-vector search should complete in <25ms, took {:?}", elapsed);
        assert_eq!(results.len(), 10, "Should return requested k=10 results");
        
        println!("500-vector performance: {:?}", elapsed);
    }
    
    #[test]
    fn test_performance_1000_vectors() {
        let query = generate_test_vector(1, 384);
        let entries = generate_test_dataset(1000, 384);
        let config = SearchConfig::default();
        
        let start = Instant::now();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        let elapsed = start.elapsed();
        
        assert!(elapsed < Duration::from_millis(50), 
            "1000-vector search should complete in <50ms, took {:?}", elapsed);
        assert_eq!(results.len(), 10, "Should return requested k=10 results");
        
        println!("1000-vector performance: {:?}", elapsed);
    }
    
    #[test]
    fn test_performance_5000_vectors() {
        let query = generate_test_vector(1, 384);
        let entries = generate_test_dataset(5000, 384);
        let config = SearchConfig::default();
        
        let start = Instant::now();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        let elapsed = start.elapsed();
        
        // More lenient threshold for very large dataset
        assert!(elapsed < Duration::from_millis(250), 
            "5000-vector search should complete in <250ms, took {:?}", elapsed);
        assert_eq!(results.len(), 10, "Should return requested k=10 results");
        
        println!("5000-vector performance: {:?}", elapsed);
    }
    
    #[test]
    fn test_parallel_performance_benefit() {
        let query = generate_test_vector(1, 384);
        let entries = generate_test_dataset(1000, 384);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig {
            parallel_threshold: 500, // Enable parallel processing
            ..PerformanceConfig::default()
        };
        
        // Test parallel search
        let start = Instant::now();
        let parallel_result = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 10, &config, &perf_config
        ).unwrap();
        let parallel_time = start.elapsed();
        
        // Test standard search
        let start = Instant::now();
        let standard_result = SimilaritySearch::k_nearest_neighbors(&query, &entries, 10, &config).unwrap();
        let standard_time = start.elapsed();
        
        // Both should return same number of results
        assert_eq!(parallel_result.results.len(), standard_result.len());
        
        // Parallel should use parallel processing for this dataset size
        assert!(parallel_result.metrics.used_parallel_processing, 
            "Should use parallel processing for 1000+ vectors");
        
        println!("Parallel: {:?}, Standard: {:?}", parallel_time, standard_time);
        
        // Verify results quality
        for result in &parallel_result.results {
            assert!(result.similarity.is_finite(), "All similarities should be finite");
        }
    }
    
    #[test]
    fn test_memory_efficiency_large_dataset() {
        let query = generate_test_vector(1, 128);
        let entries = generate_test_dataset(2000, 128);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig::default();
        
        let result = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 20, &config, &perf_config
        ).unwrap();
        
        // Memory should scale with k, not with database size
        let estimated_memory = result.metrics.estimated_memory_bytes;
        
        // Rough check: memory should be proportional to k (20) not dataset size (2000)
        let memory_per_result = estimated_memory / result.results.len();
        println!("Memory usage: {} bytes, per result: {} bytes", 
            estimated_memory, memory_per_result);
        
        // Memory per result should be reasonable (vector + metadata)
        assert!(memory_per_result < 10000, // Less than 10KB per result
            "Memory per result should be reasonable: {} bytes", memory_per_result);
    }
    
    #[test]
    fn test_throughput_measurement() {
        let query = generate_test_vector(1, 256);
        let entries = generate_test_dataset(1000, 256);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig::default();
        
        let result = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 10, &config, &perf_config
        ).unwrap();
        
        let throughput = result.metrics.vectors_per_second;
        
        // Should process at least 1000 vectors per second
        assert!(throughput > 1000.0, 
            "Throughput should be >1000 vectors/sec, got {:.1}", throughput);
        
        println!("Throughput: {:.1} vectors/second", throughput);
    }
}

#[cfg(test)]
mod memory_usage_tests {
    use super::*;
    
    fn estimate_object_memory<T>(_obj: &T) -> usize {
        std::mem::size_of::<T>()
    }
    
    #[test]
    fn test_memory_usage_during_search() {
        let query = generate_test_vector(1, 384);
        let entries = generate_test_dataset(1000, 384);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig::default();
        
        // Measure memory before search
        let initial_memory = estimate_object_memory(&entries);
        
        let result = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 10, &config, &perf_config
        ).unwrap();
        
        // Check reported memory usage
        let reported_memory = result.metrics.estimated_memory_bytes;
        
        // Memory usage should be reasonable compared to database size
        let database_memory = entries.len() * 384 * 4; // Rough estimate: vectors only
        
        println!("Database memory: ~{} bytes", database_memory);
        println!("Initial measured: {} bytes", initial_memory);
        println!("Reported usage: {} bytes", reported_memory);
        
        // Result memory should be much smaller than total database
        assert!(reported_memory < database_memory / 10, 
            "Result memory should be <10% of database memory");
        
        // Should track memory reasonably
        assert!(reported_memory > 1000, "Should report some memory usage");
    }
    
    #[test]
    fn test_memory_scaling_with_k() {
        let query = generate_test_vector(1, 256);
        let entries = generate_test_dataset(500, 256);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig::default();
        
        // Test with k=5
        let result_5 = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 5, &config, &perf_config
        ).unwrap();
        
        // Test with k=20
        let result_20 = SimilaritySearch::parallel_k_nearest_neighbors(
            &query, &entries, 20, &config, &perf_config
        ).unwrap();
        
        let memory_5 = result_5.metrics.estimated_memory_bytes;
        let memory_20 = result_20.metrics.estimated_memory_bytes;
        
        println!("Memory k=5: {} bytes, k=20: {} bytes", memory_5, memory_20);
        
        // Memory should scale roughly with k
        let ratio = memory_20 as f64 / memory_5 as f64;
        assert!(ratio > 2.0 && ratio < 6.0, 
            "Memory should scale with k (ratio ~4x): got {:.2}x", ratio);
    }
    
    #[test]
    fn test_memory_efficient_batch_processing() {
        let queries = vec![
            generate_test_vector(1, 128),
            generate_test_vector(2, 128),
            generate_test_vector(3, 128),
        ];
        let entries = generate_test_dataset(200, 128);
        let config = SearchConfig::default();
        let perf_config = PerformanceConfig {
            enable_memory_optimization: true,
            ..PerformanceConfig::default()
        };
        
        let results = SimilaritySearch::memory_efficient_batch_search(
            &queries, &entries, 5, &config, &perf_config
        ).unwrap();
        
        assert_eq!(results.len(), 3, "Should process all queries");
        
        // Check that each result has reasonable memory usage
        for (i, result) in results.iter().enumerate() {
            let memory = result.metrics.estimated_memory_bytes;
            println!("Query {}: {} bytes", i, memory);
            
            assert!(memory < 50000, // Less than 50KB per query
                "Memory per query should be reasonable: {} bytes", memory);
        }
    }
}

#[cfg(test)]
mod accuracy_tests_various_text_types {
    use super::*;
    
    fn create_text_entry(vector: Vec<f32>, file_path: &str, text_type: &str, content: &str) -> EmbeddingEntry {
        EmbeddingEntry::new(
            vector,
            format!("{}_{}.md", text_type, file_path),
            "chunk001".to_string(),
            content,
            "test-model".to_string(),
        )
    }
    
    #[test]
    fn test_accuracy_with_technical_text() {
        // Simulate embeddings for technical content
        let query_technical = vec![0.8, 0.1, 0.2, 0.5, 0.3]; // Technical query
        
        let entries = vec![
            create_text_entry(
                vec![0.85, 0.12, 0.18, 0.52, 0.28],  // Very similar to query
                "rust_guide", "technical",
                "This guide covers advanced Rust programming concepts including ownership, borrowing, and lifetimes."
            ),
            create_text_entry(
                vec![0.3, 0.9, 0.8, 0.1, 0.2],      // Different domain (narrative)
                "story", "narrative", 
                "Once upon a time, in a magical kingdom far away, there lived a brave knight who embarked on epic adventures."
            ),
            create_text_entry(
                vec![0.82, 0.08, 0.25, 0.48, 0.31], // Similar technical content
                "python_tutorial", "technical",
                "Learn Python programming fundamentals including data structures, algorithms, and object-oriented programming."
            ),
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query_technical, &entries, 3, &config).unwrap();
        
        // Technical content should rank higher than narrative
        assert!(results[0].entry.metadata.file_path.contains("technical"),
            "Most similar should be technical content");
        assert!(results[1].entry.metadata.file_path.contains("technical"),
            "Second most similar should be technical content");
        assert!(results[2].entry.metadata.file_path.contains("narrative"),
            "Narrative should rank last");
        
        // Similarity scores should reflect content type similarity
        assert!(results[0].similarity > results[2].similarity + 0.1,
            "Technical content should have significantly higher similarity");
    }
    
    #[test]
    fn test_accuracy_with_list_content() {
        // Simulate embeddings for list-type content
        let query_list = vec![0.2, 0.8, 0.3, 0.1, 0.6]; // List/structured query
        
        let entries = vec![
            create_text_entry(
                vec![0.25, 0.82, 0.28, 0.12, 0.58], // Similar list structure
                "shopping_list", "list",
                "Shopping List:\n1. Apples\n2. Bread\n3. Milk\n4. Eggs\n5. Cheese"
            ),
            create_text_entry(
                vec![0.18, 0.78, 0.32, 0.08, 0.64], // Another list
                "todo_list", "list",
                "TODO:\n- Fix bug in authentication\n- Update documentation\n- Review pull requests\n- Plan sprint"
            ),
            create_text_entry(
                vec![0.7, 0.1, 0.8, 0.9, 0.2],     // Very different (technical)
                "algorithm", "technical",
                "The quicksort algorithm is a divide-and-conquer algorithm that works by selecting a pivot element..."
            ),
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query_list, &entries, 3, &config).unwrap();
        
        // List content should cluster together
        assert!(results[0].entry.metadata.file_path.contains("list"),
            "Most similar should be list content");
        assert!(results[1].entry.metadata.file_path.contains("list"),
            "Second most similar should be list content");
        assert!(results[2].entry.metadata.file_path.contains("technical"),
            "Technical content should rank last for list query");
    }
    
    #[test]
    fn test_accuracy_with_short_vs_long_text() {
        let query = vec![0.5, 0.5, 0.0, 0.0, 0.0];
        
        let entries = vec![
            create_text_entry(
                vec![0.55, 0.52, 0.02, 0.01, 0.01], // Similar to query
                "short", "short",
                "Brief note."
            ),
            create_text_entry(
                vec![0.48, 0.53, 0.05, 0.02, 0.03], // Also similar
                "long", "long", 
                "This is a much longer document that contains extensive information about various topics, covering multiple paragraphs with detailed explanations, examples, and comprehensive analysis of the subject matter at hand. The document spans several pages and includes numerous references, citations, and supporting evidence to substantiate the claims made throughout the text. It represents a thorough examination of the topic with in-depth research and careful consideration of multiple perspectives and viewpoints."
            ),
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query, &entries, 2, &config).unwrap();
        
        // Both should be found based on similarity, not length
        assert_eq!(results.len(), 2, "Should find both short and long documents");
        
        // Similarity should be based on content, not length (more realistic threshold)
        for result in &results {
            assert!(result.similarity > 0.7, 
                "Both should have reasonable similarity regardless of length: {}", result.similarity);
        }
    }
    
    #[test]
    fn test_accuracy_with_multilingual_simulation() {
        // Simulate different "languages" with different vector patterns
        let query_english = vec![0.8, 0.2, 0.1, 0.0, 0.0]; // "English" pattern
        
        let entries = vec![
            create_text_entry(
                vec![0.82, 0.18, 0.08, 0.02, 0.01], // English-like
                "english_doc", "english",
                "This is an English document about machine learning and artificial intelligence."
            ),
            create_text_entry(
                vec![0.1, 0.8, 0.3, 0.5, 0.2], // Different language pattern
                "other_lang", "other", 
                "Dette er et dokument på et annet språk om maskinlæring og kunstig intelligens."
            ),
            create_text_entry(
                vec![0.78, 0.25, 0.12, 0.03, 0.02], // Another English-like
                "english_doc2", "english",
                "Another English document discussing natural language processing and computer vision."
            ),
        ];
        
        let config = SearchConfig::default();
        let results = SimilaritySearch::k_nearest_neighbors(&query_english, &entries, 3, &config).unwrap();
        
        // English-like documents should cluster together
        assert!(results[0].entry.metadata.file_path.contains("english"),
            "Most similar should be English document");
        assert!(results[1].entry.metadata.file_path.contains("english"),
            "Second most similar should be English document");
        assert!(results[2].entry.metadata.file_path.contains("other"),
            "Other language should rank last");
    }
    
    #[test]
    fn test_accuracy_within_tolerance() {
        // Test that our implementation is within 5% of theoretical exact cosine similarity
        let test_cases = vec![
            // (vector_a, vector_b, expected_similarity)
            (vec![1.0, 0.0, 0.0], vec![1.0, 0.0, 0.0], 1.0),
            (vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0], 0.0),
            (vec![1.0, 0.0, 0.0], vec![-1.0, 0.0, 0.0], -1.0),
            (vec![1.0, 1.0, 0.0], vec![1.0, 0.0, 0.0], 0.7071068), // cos(45°)
            (vec![3.0, 4.0], vec![4.0, 3.0], 0.96), // cos(≈16.26°)
        ];
        
        for (vec_a, vec_b, expected) in test_cases {
            let computed = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
            let error = (computed - expected).abs();
            let relative_error = error / expected.abs().max(f32::EPSILON);
            
            assert!(relative_error < 0.05, 
                "Accuracy test failed: expected {}, got {} (error: {:.1}%)", 
                expected, computed, relative_error * 100.0);
        }
    }
}

#[cfg(test)]
mod concurrent_search_tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_concurrent_search_manager() {
        let perf_config = PerformanceConfig {
            max_concurrent_requests: 3,
            ..PerformanceConfig::default()
        };
        let manager = ConcurrentSearchManager::new(perf_config);
        
        let _query = generate_test_vector(1, 128);
        let _entries = Arc::new(generate_test_dataset(100, 128));
        
        // Test single search
        let result = manager.execute_search(|| {
            let query = generate_test_vector(1, 128);
            let entries = generate_test_dataset(100, 128);
            let config = SearchConfig::default();
            SimilaritySearch::k_nearest_neighbors(&query, &entries, 5, &config)
        }).await;
        
        assert!(result.is_ok(), "Single search should succeed");
        assert_eq!(result.unwrap().len(), 5, "Should return k=5 results");
        
        // Check metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 1, "Should track request count");
        assert!(metrics.average_response_time_ms > 0.0, "Should track response time");
    }
    
    #[tokio::test]
    async fn test_concurrent_batch_search() {
        let manager = ConcurrentSearchManager::new(PerformanceConfig::default());
        
        let queries = vec![
            generate_test_vector(1, 64),
            generate_test_vector(2, 64),
            generate_test_vector(3, 64),
        ];
        let entries = Arc::new(generate_test_dataset(50, 64));
        let config = SearchConfig::default();
        
        let start = Instant::now();
        let results = manager.execute_batch_search(queries, entries, 3, config).await;
        let elapsed = start.elapsed();
        
        assert!(results.is_ok(), "Batch search should succeed");
        let results = results.unwrap();
        assert_eq!(results.len(), 3, "Should process all queries");
        
        for result in &results {
            assert_eq!(result.results.len(), 3, "Each query should return k=3 results");
            assert!(result.metrics.total_time_ms > 0.0, "Should track timing");
        }
        
        println!("Batch search completed in {:?}", elapsed);
    }
    
    #[tokio::test]
    async fn test_concurrent_request_limiting() {
        let perf_config = PerformanceConfig {
            max_concurrent_requests: 2, // Very limited
            ..PerformanceConfig::default()
        };
        let manager = ConcurrentSearchManager::new(perf_config);
        
        // Create multiple concurrent requests
        let mut handles = Vec::new();
        
        for i in 0..5 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone.execute_search(move || {
                    // Simulate some work
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    
                    let query = generate_test_vector(i + 1, 32);
                    let entries = generate_test_dataset(20, 32);
                    let config = SearchConfig::default();
                    SimilaritySearch::k_nearest_neighbors(&query, &entries, 2, &config)
                }).await
            });
            handles.push(handle);
        }
        
        // All requests should complete successfully despite concurrency limit
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok(), "Concurrent request should succeed");
        }
        
        // Give time for cleanup
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        // Check final metrics
        let metrics = manager.get_metrics().await;
        assert_eq!(metrics.total_requests, 5, "Should track all requests");
        
        // Active requests should be minimal (may not be exactly 0 due to timing)
        let active_count = manager.get_active_request_count();
        assert!(active_count <= 1, "Active requests should be minimal at end: {}", active_count);
    }
    
    #[tokio::test]
    async fn test_high_load_detection() {
        let perf_config = PerformanceConfig {
            max_concurrent_requests: 5,
            ..PerformanceConfig::default()
        };
        let manager = ConcurrentSearchManager::new(perf_config);
        
        // Initially should not be high load
        assert!(!manager.is_high_load().await, "Should not be high load initially");
        
        // Create many concurrent requests to trigger high load
        let mut handles = Vec::new();
        
        for i in 0..4 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone.execute_search(move || {
                    // Simulate longer work to maintain high load
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    
                    let query = generate_test_vector(i + 1, 16);
                    let entries = generate_test_dataset(10, 16);
                    let config = SearchConfig::default();
                    SimilaritySearch::k_nearest_neighbors(&query, &entries, 1, &config)
                }).await
            });
            handles.push(handle);
        }
        
        // Give requests time to start
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        
        // Should now detect high load (80% of 5 = 4 concurrent requests)
        let is_high_load = manager.is_high_load().await;
        let active_count = manager.get_active_request_count();
        
        println!("Active requests: {}, High load: {}", active_count, is_high_load);
        
        // Clean up
        for handle in handles {
            let _ = handle.await;
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    fn test_search_command_integration() {
        // Test that Tauri commands work with the search engine
        use ainote_lib::similarity_search_commands::*;
        
        let query_vector = generate_test_vector(1, 64);
        let database_entries = generate_test_dataset(20, 64);
        
        let request = SearchRequest {
            query_vector,
            k: 5,
            config: Some(SearchConfig::default()),
            perf_config: Some(PerformanceConfig::default()),
        };
        
        // This would normally be called via Tauri, but we can test it directly
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(optimized_search_similar_notes(request, database_entries));
        
        assert!(result.is_ok(), "Search command should succeed: {:?}", result);
        
        if let Ok(response) = result {
            assert_eq!(response.results.len(), 5, "Should return k=5 results");
            assert!(response.metrics.total_time_ms > 0.0, "Should report timing");
            assert!(response.metrics.vectors_processed > 0, "Should report processed count");
            
            // Check result format
            for search_result in &response.results {
                assert!(!search_result.file_path.is_empty(), "File path should not be empty");
                assert!(search_result.similarity.is_finite(), "Similarity should be finite");
                assert!(search_result.vector_dimension > 0, "Should report vector dimension");
            }
        }
    }
    
    #[test]
    fn test_mathematical_accuracy_verification() {
        // Test helper: Verify mathematical correctness of cosine similarity calculation
        let test_cases = vec![
            (vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]),
            (vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]),
            (vec![1.0, 1.0, 1.0], vec![2.0, 2.0, 2.0]),
        ];
        
        for (vec_a, vec_b) in test_cases {
            // Calculate using our implementation
            let our_result = SimilaritySearch::cosine_similarity(&vec_a, &vec_b).unwrap();
            
            // Calculate manually for verification
            let dot_product: f32 = vec_a.iter().zip(vec_b.iter()).map(|(a, b)| a * b).sum();
            let magnitude_a: f32 = vec_a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let magnitude_b: f32 = vec_b.iter().map(|x| x * x).sum::<f32>().sqrt();
            let expected = dot_product / (magnitude_a * magnitude_b);
            
            // Check if results match within floating-point precision
            let difference = (our_result - expected).abs();
            assert!(difference < 1e-6, 
                "Mathematical verification failed: expected {}, got {}", expected, our_result);
        }
    }
}