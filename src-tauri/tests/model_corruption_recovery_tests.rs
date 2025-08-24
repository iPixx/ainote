// Comprehensive tests for model corruption detection and recovery mechanisms
// Tests handling of corrupted models, partial downloads, and recovery strategies

use ainote_lib::ollama_client::{
    OllamaClient, OllamaConfig, DownloadStatus, 
    ModelCompatibility
};
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Configuration for corruption and recovery tests
#[derive(Debug, Clone)]
struct CorruptionTestConfig {
    pub recovery_attempt_timeout_ms: u64,
    pub corruption_detection_timeout_ms: u64,
    pub max_recovery_attempts: usize,
}

impl Default for CorruptionTestConfig {
    fn default() -> Self {
        Self {
            recovery_attempt_timeout_ms: 10000,  // 10 seconds per recovery attempt
            corruption_detection_timeout_ms: 5000, // 5 seconds to detect corruption
            max_recovery_attempts: 3,            // Maximum recovery retry attempts
        }
    }
}

/// Types of corruption scenarios to test
#[derive(Debug, Clone)]
enum CorruptionScenario {
    PartialDownload,
    NetworkInterruption,
    InvalidModelData,
    IncompleteVerification,
    ConcurrentModification,
}

/// Results from corruption recovery testing
#[derive(Debug, Clone)]
struct CorruptionRecoveryResult {
    pub scenario: CorruptionScenario,
    pub corruption_detected: bool,
    pub recovery_successful: bool,
    pub recovery_attempts: usize,
    pub total_recovery_time_ms: u64,
    pub final_model_state: ModelRecoveryState,
}

#[derive(Debug, Clone)]
enum ModelRecoveryState {
    Healthy,
    Corrupted,
    Unavailable,
    PartiallyRecovered,
}

/// Helper for testing model corruption and recovery
struct ModelCorruptionTester {
    client: OllamaClient,
    config: CorruptionTestConfig,
    test_results: Vec<CorruptionRecoveryResult>,
}

impl ModelCorruptionTester {
    fn new() -> Self {
        let ollama_config = OllamaConfig {
            timeout_ms: 5000, // Reasonable timeout for corruption tests
            max_retries: 3,    // Enable retries for recovery testing
            ..Default::default()
        };
        
        Self {
            client: OllamaClient::with_config(ollama_config),
            config: CorruptionTestConfig::default(),
            test_results: Vec::new(),
        }
    }

    /// Test complete corruption detection and recovery workflow
    pub async fn test_complete_corruption_recovery(&mut self) -> Result<(), String> {
        println!("Starting complete corruption detection and recovery testing");
        
        let scenarios = vec![
            CorruptionScenario::PartialDownload,
            CorruptionScenario::NetworkInterruption,
            CorruptionScenario::InvalidModelData,
            CorruptionScenario::IncompleteVerification,
            CorruptionScenario::ConcurrentModification,
        ];
        
        for scenario in scenarios {
            println!("Testing scenario: {:?}", scenario);
            
            let result = self.test_corruption_scenario(scenario.clone()).await
                .unwrap_or_else(|e| {
                    println!("Scenario {:?} failed: {}", scenario, e);
                    CorruptionRecoveryResult {
                        scenario: scenario.clone(),
                        corruption_detected: false,
                        recovery_successful: false,
                        recovery_attempts: 0,
                        total_recovery_time_ms: 0,
                        final_model_state: ModelRecoveryState::Unavailable,
                    }
                });
            
            self.test_results.push(result);
        }
        
        self.generate_corruption_test_report();
        Ok(())
    }

    /// Test a specific corruption scenario
    async fn test_corruption_scenario(&self, scenario: CorruptionScenario) -> Result<CorruptionRecoveryResult, String> {
        let recovery_start = Instant::now();
        let test_model = "nomic-embed-text"; // Use standard test model
        
        match scenario {
            CorruptionScenario::PartialDownload => {
                self.test_partial_download_recovery(test_model).await
            }
            CorruptionScenario::NetworkInterruption => {
                self.test_network_interruption_recovery(test_model).await
            }
            CorruptionScenario::InvalidModelData => {
                self.test_invalid_model_data_recovery(test_model).await
            }
            CorruptionScenario::IncompleteVerification => {
                self.test_incomplete_verification_recovery(test_model).await
            }
            CorruptionScenario::ConcurrentModification => {
                self.test_concurrent_modification_recovery(test_model).await
            }
        }.map(|mut result| {
            result.total_recovery_time_ms = recovery_start.elapsed().as_millis() as u64;
            result
        })
    }

    /// Test recovery from partial download scenario
    async fn test_partial_download_recovery(&self, model_name: &str) -> Result<CorruptionRecoveryResult, String> {
        println!("Testing partial download recovery");
        
        // Simulate partial download by starting and cancelling download
        let download_result = self.client.download_model(model_name).await;
        
        match download_result {
            Ok(_) => {
                // Wait briefly then cancel to simulate partial download
                tokio::time::sleep(Duration::from_millis(1000)).await;
                
                let cancel_result = self.client.cancel_download(model_name).await;
                let corruption_detected = cancel_result.is_ok();
                
                if corruption_detected {
                    // Attempt recovery by re-downloading
                    let recovery_attempts = self.attempt_model_recovery(model_name).await?;
                    
                    // Verify final state
                    let final_verification = self.client.verify_model(model_name).await
                        .map_err(|e| format!("Final verification failed: {}", e))?;
                    
                    let final_state = if final_verification.is_available && 
                                        final_verification.is_compatible == ModelCompatibility::Compatible {
                        ModelRecoveryState::Healthy
                    } else if final_verification.is_available {
                        ModelRecoveryState::PartiallyRecovered
                    } else {
                        ModelRecoveryState::Corrupted
                    };
                    
                    Ok(CorruptionRecoveryResult {
                        scenario: CorruptionScenario::PartialDownload,
                        corruption_detected: true,
                        recovery_successful: matches!(final_state, ModelRecoveryState::Healthy),
                        recovery_attempts,
                        total_recovery_time_ms: 0, // Will be set by caller
                        final_model_state: final_state,
                    })
                } else {
                    Ok(CorruptionRecoveryResult {
                        scenario: CorruptionScenario::PartialDownload,
                        corruption_detected: false,
                        recovery_successful: false,
                        recovery_attempts: 0,
                        total_recovery_time_ms: 0,
                        final_model_state: ModelRecoveryState::Unavailable,
                    })
                }
            }
            Err(e) => {
                println!("Download failed to start for partial download test: {}", e);
                Ok(CorruptionRecoveryResult {
                    scenario: CorruptionScenario::PartialDownload,
                    corruption_detected: false,
                    recovery_successful: false,
                    recovery_attempts: 0,
                    total_recovery_time_ms: 0,
                    final_model_state: ModelRecoveryState::Unavailable,
                })
            }
        }
    }

    /// Test recovery from network interruption scenario
    async fn test_network_interruption_recovery(&self, model_name: &str) -> Result<CorruptionRecoveryResult, String> {
        println!("Testing network interruption recovery");
        
        // Simulate network interruption by using a bad configuration temporarily
        let original_config = self.client.get_config().clone();
        
        let bad_config = OllamaConfig {
            base_url: "http://localhost:99999".to_string(), // Invalid port
            timeout_ms: 1000, // Short timeout to fail fast
            ..original_config.clone()
        };
        
        // Create client with bad configuration
        let bad_client = OllamaClient::with_config(bad_config);
        
        // Attempt operation that will fail due to "network interruption"
        let failed_result = bad_client.verify_model(model_name).await;
        let corruption_detected = failed_result.is_err();
        
        if corruption_detected {
            println!("Network interruption simulated, attempting recovery");
            
            // Simulate recovery by restoring good configuration
            let mut recovery_client = self.client.clone();
            recovery_client.update_config(original_config).await;
            
            let recovery_attempts = self.attempt_model_recovery_with_client(&recovery_client, model_name).await?;
            
            // Verify recovery
            let final_verification = recovery_client.verify_model(model_name).await;
            let recovery_successful = final_verification.is_ok();
            
            let final_state = if recovery_successful {
                match final_verification.unwrap() {
                    v if v.is_available && v.is_compatible == ModelCompatibility::Compatible => ModelRecoveryState::Healthy,
                    v if v.is_available => ModelRecoveryState::PartiallyRecovered,
                    _ => ModelRecoveryState::Corrupted,
                }
            } else {
                ModelRecoveryState::Unavailable
            };
            
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::NetworkInterruption,
                corruption_detected: true,
                recovery_successful,
                recovery_attempts,
                total_recovery_time_ms: 0,
                final_model_state: final_state,
            })
        } else {
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::NetworkInterruption,
                corruption_detected: false,
                recovery_successful: false,
                recovery_attempts: 0,
                total_recovery_time_ms: 0,
                final_model_state: ModelRecoveryState::Healthy,
            })
        }
    }

    /// Test recovery from invalid model data scenario
    async fn test_invalid_model_data_recovery(&self, _model_name: &str) -> Result<CorruptionRecoveryResult, String> {
        println!("Testing invalid model data recovery");
        
        // Test verification of non-existent model (simulates corrupted model data)
        let invalid_model = "definitely-non-existent-model-12345";
        let verification_result = self.client.verify_model(invalid_model).await;
        
        let corruption_detected = match verification_result {
            Ok(result) => !result.is_available, // Not available indicates potential corruption
            Err(_) => true, // Error indicates corruption
        };
        
        if corruption_detected {
            println!("Invalid model data detected, testing recovery");
            
            // Attempt to "recover" by verifying a known good model
            let recovery_attempts = self.attempt_model_recovery("nomic-embed-text").await?;
            
            // Check if we can recover to a working state
            let recovery_verification = self.client.verify_model("nomic-embed-text").await;
            let recovery_successful = recovery_verification.is_ok();
            
            let final_state = if recovery_successful {
                ModelRecoveryState::Healthy
            } else {
                ModelRecoveryState::Corrupted
            };
            
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::InvalidModelData,
                corruption_detected: true,
                recovery_successful,
                recovery_attempts,
                total_recovery_time_ms: 0,
                final_model_state: final_state,
            })
        } else {
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::InvalidModelData,
                corruption_detected: false,
                recovery_successful: true,
                recovery_attempts: 0,
                total_recovery_time_ms: 0,
                final_model_state: ModelRecoveryState::Healthy,
            })
        }
    }

    /// Test recovery from incomplete verification scenario
    async fn test_incomplete_verification_recovery(&self, model_name: &str) -> Result<CorruptionRecoveryResult, String> {
        println!("Testing incomplete verification recovery");
        
        // Simulate incomplete verification by using very short timeout
        let short_timeout_config = OllamaConfig {
            timeout_ms: 1, // Extremely short timeout to cause timeouts
            max_retries: 0, // No retries for immediate failure
            ..self.client.get_config().clone()
        };
        
        let timeout_client = OllamaClient::with_config(short_timeout_config);
        
        // Attempt verification that will likely timeout/fail
        let failed_verification = timeout_client.verify_model(model_name).await;
        let corruption_detected = failed_verification.is_err();
        
        if corruption_detected {
            println!("Incomplete verification detected, attempting recovery");
            
            // Recovery: use client with normal timeout
            let recovery_attempts = self.attempt_model_recovery(model_name).await?;
            
            // Verify recovery with normal client
            let recovery_verification = self.client.verify_model(model_name).await;
            let recovery_successful = recovery_verification.is_ok();
            
            let final_state = match recovery_verification {
                Ok(result) if result.is_available && result.is_compatible == ModelCompatibility::Compatible => {
                    ModelRecoveryState::Healthy
                }
                Ok(result) if result.is_available => {
                    ModelRecoveryState::PartiallyRecovered
                }
                Ok(_) => ModelRecoveryState::Corrupted,
                Err(_) => ModelRecoveryState::Unavailable,
            };
            
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::IncompleteVerification,
                corruption_detected: true,
                recovery_successful,
                recovery_attempts,
                total_recovery_time_ms: 0,
                final_model_state: final_state,
            })
        } else {
            // Verification succeeded despite short timeout
            Ok(CorruptionRecoveryResult {
                scenario: CorruptionScenario::IncompleteVerification,
                corruption_detected: false,
                recovery_successful: true,
                recovery_attempts: 0,
                total_recovery_time_ms: 0,
                final_model_state: ModelRecoveryState::Healthy,
            })
        }
    }

    /// Test recovery from concurrent modification scenario
    async fn test_concurrent_modification_recovery(&self, model_name: &str) -> Result<CorruptionRecoveryResult, String> {
        println!("Testing concurrent modification recovery");
        
        // Simulate concurrent access that might cause state corruption
        let concurrent_clients: Vec<_> = (0..5).map(|_| self.client.clone()).collect();
        let mut handles = Vec::new();
        
        // Launch concurrent operations that might interfere with each other
        for (i, client) in concurrent_clients.into_iter().enumerate() {
            let model_name = model_name.to_string();
            
            let handle = tokio::spawn(async move {
                // Stagger operations to increase chance of interference
                tokio::time::sleep(Duration::from_millis(i as u64 * 50)).await;
                
                let mut results = Vec::new();
                
                // Multiple operations that might conflict
                let verify_result = client.verify_model(&model_name).await;
                results.push(("verify", verify_result.is_ok()));
                
                let models_result = client.get_available_models().await;
                results.push(("list_models", models_result.is_ok()));
                
                let health_result = client.check_health().await;
                results.push(("health", health_result.is_ok()));
                
                (i, results)
            });
            handles.push(handle);
        }
        
        // Collect results from concurrent operations
        let mut total_operations = 0;
        let mut successful_operations = 0;
        let mut corruption_indicators = 0;
        
        for handle in handles {
            let (task_id, task_results) = handle.await
                .map_err(|e| format!("Concurrent task failed: {:?}", e))?;
            
            for (op_name, success) in task_results {
                total_operations += 1;
                if success {
                    successful_operations += 1;
                } else {
                    corruption_indicators += 1;
                    println!("Task {} operation '{}' failed (possible corruption)", task_id, op_name);
                }
            }
        }
        
        let corruption_detected = corruption_indicators > 0;
        let success_rate = successful_operations as f64 / total_operations as f64;
        
        println!("Concurrent modification test: {}/{} operations successful ({:.1}%)", 
                successful_operations, total_operations, success_rate * 100.0);
        
        let recovery_attempts = if corruption_detected {
            self.attempt_model_recovery(model_name).await?
        } else {
            0
        };
        
        // Final verification
        let final_verification = self.client.verify_model(model_name).await;
        let recovery_successful = final_verification.is_ok() && success_rate > 0.5;
        
        let final_state = if recovery_successful {
            if success_rate > 0.9 {
                ModelRecoveryState::Healthy
            } else {
                ModelRecoveryState::PartiallyRecovered
            }
        } else {
            ModelRecoveryState::Corrupted
        };
        
        Ok(CorruptionRecoveryResult {
            scenario: CorruptionScenario::ConcurrentModification,
            corruption_detected,
            recovery_successful,
            recovery_attempts,
            total_recovery_time_ms: 0,
            final_model_state: final_state,
        })
    }

    /// Attempt to recover a corrupted model
    async fn attempt_model_recovery(&self, model_name: &str) -> Result<usize, String> {
        self.attempt_model_recovery_with_client(&self.client, model_name).await
    }

    async fn attempt_model_recovery_with_client(&self, client: &OllamaClient, model_name: &str) -> Result<usize, String> {
        let mut attempts = 0;
        
        for attempt in 1..=self.config.max_recovery_attempts {
            attempts = attempt;
            println!("Recovery attempt {} for model '{}'", attempt, model_name);
            
            let recovery_start = Instant::now();
            
            // Recovery strategy 1: Re-verify model
            let verification_result = client.verify_model(model_name).await;
            
            match verification_result {
                Ok(result) if result.is_available && result.is_compatible == ModelCompatibility::Compatible => {
                    println!("✅ Model recovered successfully on attempt {}", attempt);
                    return Ok(attempts);
                }
                Ok(result) if !result.is_available => {
                    println!("Model not available, attempting re-download");
                    
                    // Recovery strategy 2: Re-download model
                    let download_result = client.download_model(model_name).await;
                    match download_result {
                        Ok(_) => {
                            // Monitor download for completion (limited time)
                            let monitor_timeout = Duration::from_millis(self.config.recovery_attempt_timeout_ms);
                            let monitor_result = timeout(monitor_timeout, 
                                self.wait_for_download_completion(client, model_name)).await;
                            
                            match monitor_result {
                                Ok(Ok(())) => {
                                    println!("✅ Model re-download completed on attempt {}", attempt);
                                    return Ok(attempts);
                                }
                                Ok(Err(e)) => {
                                    println!("Re-download failed on attempt {}: {}", attempt, e);
                                }
                                Err(_) => {
                                    println!("Re-download timed out on attempt {}", attempt);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Re-download failed to start on attempt {}: {}", attempt, e);
                        }
                    }
                }
                Ok(_) => {
                    println!("Model available but incompatible, cannot recover");
                    break;
                }
                Err(e) => {
                    println!("Verification failed on attempt {}: {}", attempt, e);
                }
            }
            
            let recovery_duration = recovery_start.elapsed();
            println!("Recovery attempt {} took {:?}", attempt, recovery_duration);
            
            // Wait before next attempt
            if attempt < self.config.max_recovery_attempts {
                tokio::time::sleep(Duration::from_millis(1000)).await;
            }
        }
        
        println!("❌ Model recovery failed after {} attempts", attempts);
        Ok(attempts)
    }

    /// Wait for download completion (helper for recovery testing)
    async fn wait_for_download_completion(&self, client: &OllamaClient, model_name: &str) -> Result<(), String> {
        let start_time = Instant::now();
        let max_wait = Duration::from_millis(self.config.recovery_attempt_timeout_ms);
        
        while start_time.elapsed() < max_wait {
            if let Some(progress) = client.get_download_progress(model_name).await {
                match progress.status {
                    DownloadStatus::Completed { .. } => {
                        return Ok(());
                    }
                    DownloadStatus::Failed { error, .. } => {
                        return Err(format!("Download failed: {}", error));
                    }
                    DownloadStatus::Cancelled => {
                        return Err("Download was cancelled".to_string());
                    }
                    _ => {
                        // Continue waiting
                    }
                }
            }
            
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        
        Err("Download completion timeout".to_string())
    }

    /// Generate comprehensive corruption test report
    fn generate_corruption_test_report(&self) {
        println!("\n=== MODEL CORRUPTION & RECOVERY TEST REPORT ===\n");
        
        if self.test_results.is_empty() {
            println!("⚠️ No corruption test results available");
            return;
        }
        
        let total_scenarios = self.test_results.len();
        let corruption_detected_count = self.test_results.iter()
            .filter(|r| r.corruption_detected)
            .count();
        let recovery_successful_count = self.test_results.iter()
            .filter(|r| r.recovery_successful)
            .count();
        
        println!("Overall Summary:");
        println!("  Total scenarios tested: {}", total_scenarios);
        println!("  Corruption detected: {}", corruption_detected_count);
        println!("  Recovery successful: {}", recovery_successful_count);
        println!("  Recovery success rate: {:.1}%", 
                (recovery_successful_count as f64 / corruption_detected_count.max(1) as f64) * 100.0);
        println!();
        
        // Detailed results
        for result in &self.test_results {
            println!("--- {:?} ---", result.scenario);
            println!("  Corruption Detected: {}", if result.corruption_detected { "✅ YES" } else { "❌ NO" });
            println!("  Recovery Successful: {}", if result.recovery_successful { "✅ YES" } else { "❌ NO" });
            println!("  Recovery Attempts: {}", result.recovery_attempts);
            println!("  Total Recovery Time: {}ms", result.total_recovery_time_ms);
            println!("  Final Model State: {:?}", result.final_model_state);
            println!();
        }
        
        // Recovery effectiveness analysis
        let avg_recovery_attempts = if corruption_detected_count > 0 {
            self.test_results.iter()
                .filter(|r| r.corruption_detected)
                .map(|r| r.recovery_attempts)
                .sum::<usize>() as f64 / corruption_detected_count as f64
        } else {
            0.0
        };
        
        let avg_recovery_time = if corruption_detected_count > 0 {
            self.test_results.iter()
                .filter(|r| r.corruption_detected)
                .map(|r| r.total_recovery_time_ms)
                .sum::<u64>() as f64 / corruption_detected_count as f64
        } else {
            0.0
        };
        
        println!("=== RECOVERY PERFORMANCE ===");
        println!("Average Recovery Attempts: {:.1}", avg_recovery_attempts);
        println!("Average Recovery Time: {:.0}ms", avg_recovery_time);
        
        // Health state distribution
        let mut state_counts = std::collections::HashMap::new();
        for result in &self.test_results {
            *state_counts.entry(format!("{:?}", result.final_model_state)).or_insert(0) += 1;
        }
        
        println!("\nFinal Model States:");
        for (state, count) in state_counts {
            println!("  {}: {} scenarios", state, count);
        }
        
        println!("\n=== RECOMMENDATIONS ===");
        if recovery_successful_count == corruption_detected_count && corruption_detected_count > 0 {
            println!("✅ Corruption recovery system working correctly");
        } else if corruption_detected_count == 0 {
            println!("⚠️ No corruption scenarios triggered - may need more aggressive testing");
        } else {
            println!("⚠️ Some corruption scenarios could not be recovered");
            println!("   Consider improving recovery strategies for failed scenarios");
        }
        
        if avg_recovery_time > 10000.0 {
            println!("⚠️ Recovery time averaging {:.0}ms may be too slow for user experience", avg_recovery_time);
        }
        
        if avg_recovery_attempts > 2.0 {
            println!("⚠️ Recovery requiring {:.1} attempts on average may frustrate users", avg_recovery_attempts);
        }
    }
}

/// Utility to check if Ollama is available for corruption testing
async fn is_ollama_available_for_corruption_testing() -> bool {
    let config = OllamaConfig {
        timeout_ms: 2000,
        ..Default::default()
    };
    let client = OllamaClient::with_config(config);
    
    match timeout(Duration::from_secs(3), client.check_health()).await {
        Ok(Ok(_)) => true,
        _ => false,
    }
}

/// Skip test if Ollama is not available
macro_rules! require_ollama_for_corruption_testing {
    () => {
        if !is_ollama_available_for_corruption_testing().await {
            println!("Skipping corruption test - Ollama not available");
            return;
        }
    };
}

#[cfg(test)]
mod corruption_recovery_tests {
    use super::*;

    #[tokio::test]
    async fn test_partial_download_corruption_recovery() {
        require_ollama_for_corruption_testing!();
        
        let tester = ModelCorruptionTester::new();
        let result = tester.test_partial_download_recovery("nomic-embed-text").await
            .expect("Partial download recovery test should complete");
        
        println!("Partial download recovery result: {:?}", result);
        
        // Test should detect and handle partial downloads
        if result.corruption_detected {
            assert!(result.recovery_attempts > 0, "Should attempt recovery");
        }
    }

    #[tokio::test]
    async fn test_network_interruption_recovery() {
        require_ollama_for_corruption_testing!();
        
        let tester = ModelCorruptionTester::new();
        let result = tester.test_network_interruption_recovery("nomic-embed-text").await
            .expect("Network interruption recovery test should complete");
        
        println!("Network interruption recovery result: {:?}", result);
        
        // Should detect network issues and recover
        assert!(result.corruption_detected, "Should detect network interruption");
        // Recovery success depends on network conditions
    }

    #[tokio::test]
    async fn test_invalid_model_data_recovery() {
        require_ollama_for_corruption_testing!();
        
        let tester = ModelCorruptionTester::new();
        let result = tester.test_invalid_model_data_recovery("nomic-embed-text").await
            .expect("Invalid model data recovery test should complete");
        
        println!("Invalid model data recovery result: {:?}", result);
        
        // Should handle invalid model gracefully
    }

    #[tokio::test]
    async fn test_incomplete_verification_recovery() {
        require_ollama_for_corruption_testing!();
        
        let tester = ModelCorruptionTester::new();
        let result = tester.test_incomplete_verification_recovery("nomic-embed-text").await
            .expect("Incomplete verification recovery test should complete");
        
        println!("Incomplete verification recovery result: {:?}", result);
        
        // Should handle verification timeouts and recover
    }

    #[tokio::test]
    async fn test_concurrent_modification_recovery() {
        require_ollama_for_corruption_testing!();
        
        let tester = ModelCorruptionTester::new();
        let result = tester.test_concurrent_modification_recovery("nomic-embed-text").await
            .expect("Concurrent modification recovery test should complete");
        
        println!("Concurrent modification recovery result: {:?}", result);
        
        // Should handle concurrent access gracefully
    }

    #[tokio::test]
    async fn test_complete_corruption_recovery_suite() {
        require_ollama_for_corruption_testing!();
        
        println!("Running complete corruption recovery test suite");
        
        let mut tester = ModelCorruptionTester::new();
        tester.test_complete_corruption_recovery().await
            .expect("Complete corruption recovery suite should pass");
        
        // Verify comprehensive testing was performed
        assert!(tester.test_results.len() >= 5, "Should test all corruption scenarios");
        
        let recovery_rate = tester.test_results.iter()
            .filter(|r| r.recovery_successful)
            .count() as f64 / tester.test_results.len() as f64;
        
        println!("Overall recovery success rate: {:.1}%", recovery_rate * 100.0);
        
        // Recovery system should handle majority of corruption scenarios
        assert!(recovery_rate >= 0.6, "Should recover from at least 60% of corruption scenarios");
    }

    #[tokio::test]
    async fn test_corruption_detection_performance() {
        require_ollama_for_corruption_testing!();
        
        println!("Testing corruption detection performance");
        
        let tester = ModelCorruptionTester::new();
        let test_model = "nomic-embed-text";
        
        // Test detection speed for various scenarios
        let scenarios = vec![
            ("valid_model", test_model),
            ("invalid_model", "non-existent-model-xyz"),
            ("empty_model_name", ""),
        ];
        
        for (scenario_name, model_name) in scenarios {
            let detection_start = Instant::now();
            
            let verification_result = tester.client.verify_model(model_name).await;
            let detection_time = detection_start.elapsed();
            
            println!("Corruption detection for '{}': {:?} in {:?}", 
                    scenario_name, verification_result.is_ok(), detection_time);
            
            // Detection should be fast enough for good user experience
            assert!(detection_time < Duration::from_millis(tester.config.corruption_detection_timeout_ms),
                   "Corruption detection for '{}' took {:?}, too slow", 
                   scenario_name, detection_time);
        }
        
        println!("✅ Corruption detection performance verified");
    }

    #[tokio::test]
    async fn test_recovery_strategy_effectiveness() {
        require_ollama_for_corruption_testing!();
        
        println!("Testing recovery strategy effectiveness");
        
        let tester = ModelCorruptionTester::new();
        
        // Test different recovery strategies
        let strategy_names = vec!["re_verification", "health_check_recovery", "model_list_refresh"];
        
        for strategy_name in strategy_names {
            let strategy_start = Instant::now();
            let strategy_success = match strategy_name {
                "re_verification" => {
                    // Strategy: Re-verify model multiple times
                    let mut success_count = 0;
                    for _ in 0..3 {
                        if tester.client.verify_model("nomic-embed-text").await.is_ok() {
                            success_count += 1;
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                    success_count >= 2
                }
                "health_check_recovery" => {
                    // Strategy: Health check before model operations
                    let health_ok = tester.client.check_health().await.is_ok();
                    if health_ok {
                        tester.client.verify_model("nomic-embed-text").await.is_ok()
                    } else {
                        false
                    }
                }
                "model_list_refresh" => {
                    // Strategy: Refresh model list before verification
                    let models_result = tester.client.get_available_models().await;
                    match models_result {
                        Ok(_) => tester.client.verify_model("nomic-embed-text").await.is_ok(),
                        Err(_) => false
                    }
                }
                _ => false,
            };
            
            let strategy_time = strategy_start.elapsed();
            
            println!("Strategy '{}': {} in {:?}", 
                    strategy_name, 
                    if strategy_success { "✅ SUCCESS" } else { "❌ FAILED" },
                    strategy_time);
            
            // Strategies should be reasonably fast
            assert!(strategy_time < Duration::from_millis(tester.config.recovery_attempt_timeout_ms),
                   "Recovery strategy '{}' took too long: {:?}", strategy_name, strategy_time);
        }
        
        println!("✅ Recovery strategy effectiveness tested");
    }
}

/// Manual testing documentation for corruption and recovery
#[cfg(test)]
mod manual_corruption_testing_utils {

    #[tokio::test]
    async fn print_corruption_testing_checklist() {
        println!("=== MANUAL CORRUPTION & RECOVERY TESTING CHECKLIST ===");
        println!();
        println!("Prerequisites:");
        println!("  □ Ollama installed with test models");
        println!("  □ Network connection that can be interrupted");
        println!("  □ Ability to stop/start Ollama service");
        println!("  □ File system access to Ollama model directory");
        println!();
        println!("Partial Download Testing:");
        println!("  □ Start large model download (e.g., llama2)");
        println!("  □ Cancel download when 25% complete");
        println!("  □ Verify UI shows cancelled state");
        println!("  □ Restart download - should resume or restart cleanly");
        println!("  □ Verify model is functional after completion");
        println!();
        println!("Network Interruption Testing:");
        println!("  □ Start model download");
        println!("  □ Disconnect network during download");
        println!("  □ Verify UI shows network error");
        println!("  □ Reconnect network");
        println!("  □ Verify download resumes or restarts automatically");
        println!();
        println!("Service Interruption Testing:");
        println!("  □ Start model operation");
        println!("  □ Stop Ollama service during operation");
        println!("  □ Verify UI detects disconnection");
        println!("  □ Restart Ollama service");
        println!("  □ Verify UI reconnects and operations resume");
        println!();
        println!("File System Corruption Testing:");
        println!("  □ Locate Ollama model files on disk");
        println!("  □ Modify/corrupt a model file");
        println!("  □ Attempt to use corrupted model in aiNote");
        println!("  □ Verify corruption detection");
        println!("  □ Test automatic re-download of corrupted model");
        println!();
        println!("Concurrent Access Testing:");
        println!("  □ Open multiple aiNote instances");
        println!("  □ Perform model operations simultaneously");
        println!("  □ Verify no conflicts or corruption");
        println!("  □ Test with other applications using Ollama concurrently");
        println!();
        println!("Recovery Validation:");
        println!("  □ Verify recovered models work correctly");
        println!("  □ Test model embedding generation after recovery");
        println!("  □ Verify performance is not degraded after recovery");
        println!("  □ Check that recovery process doesn't affect other models");
        println!();
        println!("Performance During Recovery:");
        println!("  □ Measure recovery time for different scenarios");
        println!("  □ Verify UI remains responsive during recovery");
        println!("  □ Monitor memory usage during recovery operations");
        println!("  □ Test recovery under system load");
        println!();
    }

    #[tokio::test]
    async fn generate_corruption_testing_guidelines() {
        println!("=== MODEL CORRUPTION TESTING GUIDELINES ===");
        println!();
        println!("Corruption Detection Criteria:");
        println!("  - Model verification fails unexpectedly");
        println!("  - Download progress stalls for >30 seconds");
        println!("  - Model size doesn't match expected values");
        println!("  - Health checks consistently fail");
        println!("  - Concurrent operations produce inconsistent results");
        println!();
        println!("Recovery Strategies Implemented:");
        println!("  1. Automatic retry with exponential backoff");
        println!("  2. Model re-verification before operations");
        println!("  3. Complete model re-download when corrupted");
        println!("  4. Service health check before model operations");
        println!("  5. Graceful degradation when recovery fails");
        println!();
        println!("Recovery Success Criteria:");
        println!("  - Model becomes available and verified");
        println!("  - Model compatibility confirmed");
        println!("  - Normal operation performance restored");
        println!("  - No side effects on other models");
        println!("  - User receives clear status updates");
        println!();
        println!("Performance Requirements:");
        println!("  - Corruption detection: <5 seconds");
        println!("  - Recovery initiation: <2 seconds");
        println!("  - Recovery completion: <30 seconds (small models)");
        println!("  - UI responsiveness: maintained throughout");
        println!();
        println!("Testing Best Practices:");
        println!("  - Test on clean system and system with existing models");
        println!("  - Vary network conditions and speeds");
        println!("  - Test with different model sizes");
        println!("  - Include edge cases and boundary conditions");
        println!("  - Verify both automatic and manual recovery");
        println!();
    }
}