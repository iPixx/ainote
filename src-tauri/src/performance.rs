use std::time::{Duration, Instant};

/// Performance instrumentation module
/// Macro to time operations and log performance
#[cfg(debug_assertions)]
macro_rules! time_operation {
    ($operation:expr, $name:expr) => {{
        let start = std::time::Instant::now();
        let result = $operation;
        let duration = start.elapsed();
        if duration.as_millis() > 10 {
            eprintln!("PERF: {} took {:.3}ms", $name, duration.as_secs_f64() * 1000.0);
        }
        result
    }};
}

#[cfg(not(debug_assertions))]
macro_rules! time_operation {
    ($operation:expr, $name:expr) => {{
        $operation
    }};
}

pub(crate) use time_operation;

/// Performance tracker for detailed metrics
pub struct PerformanceTracker {
    #[allow(dead_code)] // Used in debug prints
    operation: String,
    start: Instant,
}

impl PerformanceTracker {
    pub fn start(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            start: Instant::now(),
        }
    }
    
    pub fn finish(self) -> Duration {
        let duration = self.start.elapsed();
        #[cfg(debug_assertions)]
        if duration.as_millis() > 5 {
            eprintln!("PERF: {} completed in {:.3}ms", self.operation, duration.as_secs_f64() * 1000.0);
        }
        duration
    }
    
    pub fn checkpoint(&self, _checkpoint_name: &str) {
        let _duration = self.start.elapsed();
        #[cfg(debug_assertions)]
        eprintln!("PERF: {} - {} at {:.3}ms", self.operation, _checkpoint_name, _duration.as_secs_f64() * 1000.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_performance_tracker_creation() {
        let tracker = PerformanceTracker::start("test_operation");
        assert_eq!(tracker.operation, "test_operation");
        assert!(tracker.start <= Instant::now());
    }

    #[test]
    fn test_performance_tracker_finish() {
        let tracker = PerformanceTracker::start("test_operation");
        
        // Sleep for a small amount to ensure measurable duration
        thread::sleep(Duration::from_millis(1));
        
        let duration = tracker.finish();
        assert!(duration.as_millis() >= 1);
    }

    #[test]
    fn test_performance_tracker_checkpoint() {
        let tracker = PerformanceTracker::start("test_operation");
        
        // Sleep for a small amount to ensure measurable duration
        thread::sleep(Duration::from_millis(1));
        
        // This should not panic and should print in debug mode
        tracker.checkpoint("test_checkpoint");
        
        let duration = tracker.finish();
        assert!(duration.as_millis() >= 1);
    }

    #[test]
    fn test_time_operation_macro() {
        let result = time_operation!({
            thread::sleep(Duration::from_millis(1));
            42
        }, "test_macro");
        
        assert_eq!(result, 42);
    }

    #[test]
    fn test_time_operation_macro_with_error() {
        let result: Result<i32, &str> = time_operation!({
            Err("test error")
        }, "test_macro_error");
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "test error");
    }

    #[test]
    fn test_performance_tracker_multiple_checkpoints() {
        let tracker = PerformanceTracker::start("multi_checkpoint_test");
        
        thread::sleep(Duration::from_millis(1));
        tracker.checkpoint("checkpoint_1");
        
        thread::sleep(Duration::from_millis(1));
        tracker.checkpoint("checkpoint_2");
        
        thread::sleep(Duration::from_millis(1));
        tracker.checkpoint("checkpoint_3");
        
        let duration = tracker.finish();
        assert!(duration.as_millis() >= 3);
    }

    #[test]
    fn test_performance_tracker_long_operation() {
        let tracker = PerformanceTracker::start("long_operation");
        
        // Simulate a longer operation
        thread::sleep(Duration::from_millis(10));
        
        let duration = tracker.finish();
        assert!(duration.as_millis() >= 10);
    }

    #[test] 
    fn test_performance_tracker_concurrent_usage() {
        let handles: Vec<_> = (0..5).map(|i| {
            thread::spawn(move || {
                let tracker = PerformanceTracker::start(&format!("concurrent_op_{}", i));
                thread::sleep(Duration::from_millis(1));
                tracker.checkpoint("mid_point");
                thread::sleep(Duration::from_millis(1));
                let duration = tracker.finish();
                duration.as_millis() >= 2
            })
        }).collect();

        // All operations should complete successfully
        for handle in handles {
            assert!(handle.join().unwrap());
        }
    }

    #[test]
    fn test_time_operation_with_complex_operation() {
        let mut counter = 0;
        let result = time_operation!({
            for i in 0..100 {
                counter += i;
            }
            counter
        }, "complex_operation");
        
        assert_eq!(result, (0..100).sum::<i32>());
    }
}