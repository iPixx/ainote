/**
 * Real-time Metrics Service
 * 
 * Coordinates real-time performance metrics collection between the frontend
 * dashboard and backend monitoring systems. Provides memory usage monitoring
 * with trend tracking, AI operation timing, and resource measurement.
 * 
 * Features:
 * - Memory usage monitoring with trend analysis
 * - AI operation timing and resource measurement  
 * - Real-time data streaming and caching
 * - Performance threshold monitoring
 * - Automatic retry on connection failures
 * - <1% overhead compliance
 */

// Use mocked invoke in test environment, actual Tauri invoke in production
const getInvoke = () => window.__TAURI__?.core?.invoke || (async () => {});

class RealTimeMetricsService {
    constructor() {
        this.isActive = false;
        this.monitoringInterval = null;
        this.subscribers = new Set();
        
        // Metrics cache
        this.metricsCache = {
            currentMetrics: {},
            resourceUtilization: null,
            memoryTrends: [],
            aiOperations: [],
            alerts: [],
            lastUpdate: null
        };
        
        // Memory monitoring configuration
        this.memoryConfig = {
            trackingInterval: 1000, // 1 second for memory tracking
            trendWindowSize: 60,    // Keep 60 seconds of memory data
            alertThresholds: {
                warning: 75,  // MB
                critical: 95, // MB
            },
            leakDetectionThreshold: 10, // MB increase over 30 seconds
        };
        
        // AI operation tracking
        this.aiOperationTracking = {
            activeOperations: new Map(),
            completedOperations: [],
            maxHistorySize: 100,
            performanceTargets: {
                embedding_generation: 500,   // ms
                similarity_search: 50,      // ms
                indexing_operation: 1000,   // ms
            }
        };
        
        // Connection retry configuration
        this.retryConfig = {
            maxRetries: 3,
            retryDelay: 1000,
            currentRetries: 0
        };
    }

    /**
     * Start real-time metrics collection
     */
    async start() {
        if (this.isActive) {
            console.warn('Real-time metrics service is already active');
            return;
        }

        try {
            const invoke = getInvoke();
            
            // Check if backend monitoring is available and start if needed
            const status = await invoke('get_monitoring_status');
            
            if (!status.is_active) {
                // Start backend monitoring with optimized configuration
                const config = {
                    enable_monitoring: true,
                    max_samples_in_memory: 1000,
                    collection_interval_ms: 100,
                    enable_resource_tracking: true,
                    resource_tracking_interval_ms: 1000,
                    max_overhead_percent: 1.0,  // <1% overhead requirement
                    enable_alerts: true,
                    alert_degradation_threshold: 20.0,
                    enable_detailed_logging: false,
                    auto_persist_interval_seconds: 60, // Auto-persist metrics every 60 seconds
                };
                
                await invoke('start_performance_monitoring', { request: { config } });
            }

            // Start enhanced metrics collection if available
            try {
                const isEnhancedActive = await invoke('is_enhanced_metrics_active');
                if (!isEnhancedActive) {
                    const enhancedConfig = {
                        enable_search_metrics: true,
                        enable_index_health_monitoring: true,
                        enable_memory_tracking: true,
                        enable_optimization_recommendations: true,
                    };
                    
                    // Get vault path from app state (mock for now)
                    const storagePath = '/tmp/ainote-vector-storage'; // TODO: Get actual path
                    
                    await invoke('start_enhanced_metrics_collection', {
                        request: {
                            config: enhancedConfig,
                            storage_dir: storagePath
                        }
                    });
                }
            } catch (error) {
                console.warn('Enhanced metrics not available:', error.message);
            }

            // Start metrics collection loop
            this.isActive = true;
            this.startMetricsLoop();
            
            
        } catch (error) {
            console.error('Failed to start real-time metrics service:', error);
            throw error;
        }
    }

    /**
     * Stop real-time metrics collection
     */
    async stop() {
        this.isActive = false;
        
        if (this.monitoringInterval) {
            clearInterval(this.monitoringInterval);
            this.monitoringInterval = null;
        }

        // Notify subscribers
        this.notifySubscribers('service_stopped', {});
        
    }

    /**
     * Start the main metrics collection loop
     */
    startMetricsLoop() {
        if (this.monitoringInterval) {
            clearInterval(this.monitoringInterval);
        }

        this.monitoringInterval = setInterval(async () => {
            try {
                await this.collectAndProcessMetrics();
                this.retryConfig.currentRetries = 0; // Reset retry count on success
            } catch (error) {
                await this.handleCollectionError(error);
            }
        }, this.memoryConfig.trackingInterval);
    }

    /**
     * Collect and process all metrics
     */
    async collectAndProcessMetrics() {
        const startTime = performance.now();

        // Collect metrics in parallel to minimize overhead
        const invoke = getInvoke();
        const [
            currentMetrics,
            resourceUtilization,
            alerts,
            searchMetrics
        ] = await Promise.allSettled([
            invoke('get_current_performance_metrics'),
            invoke('get_resource_utilization'),
            invoke('get_active_alerts'),
            this.getSearchOperationMetrics()
        ]);

        // Process results
        if (currentMetrics.status === 'fulfilled') {
            this.metricsCache.currentMetrics = currentMetrics.value;
        }

        if (resourceUtilization.status === 'fulfilled') {
            this.processResourceUtilization(resourceUtilization.value);
        }

        if (alerts.status === 'fulfilled') {
            this.metricsCache.alerts = alerts.value;
        }

        if (searchMetrics.status === 'fulfilled') {
            this.processAIOperations(searchMetrics.value);
        }

        // Update cache timestamp
        this.metricsCache.lastUpdate = Date.now();

        // Notify subscribers with updated data
        this.notifySubscribers('metrics_updated', {
            currentMetrics: this.metricsCache.currentMetrics,
            resourceUtilization: this.metricsCache.resourceUtilization,
            memoryTrends: this.metricsCache.memoryTrends,
            aiOperations: this.metricsCache.aiOperations,
            alerts: this.metricsCache.alerts
        });

        // Monitor collection overhead
        const collectionTime = performance.now() - startTime;
        if (collectionTime > 10) { // Log if collection takes >10ms
            console.warn(`Metrics collection took ${collectionTime.toFixed(2)}ms - consider optimizing`);
        }
    }

    /**
     * Get search operation metrics (with fallback)
     */
    async getSearchOperationMetrics() {
        try {
            return await getInvoke()('get_search_operation_metrics', { limit: 20 });
        } catch (error) {
            // Enhanced metrics not available, return empty array
            return [];
        }
    }

    /**
     * Process resource utilization data and track memory trends
     */
    processResourceUtilization(resourceData) {
        this.metricsCache.resourceUtilization = resourceData;
        
        // Track memory trends
        const memoryTrend = {
            timestamp: Date.now(),
            memoryUsageMB: resourceData.memory_usage_mb,
            memoryAvailableMB: resourceData.memory_available_mb,
            cpuUsagePercent: resourceData.cpu_usage_percent,
        };
        
        this.metricsCache.memoryTrends.push(memoryTrend);
        
        // Maintain trend window size
        if (this.metricsCache.memoryTrends.length > this.memoryConfig.trendWindowSize) {
            this.metricsCache.memoryTrends.shift();
        }
        
        // Detect memory leaks
        this.detectMemoryLeaks();
        
        // Check memory thresholds
        this.checkMemoryThresholds(resourceData.memory_usage_mb);
    }

    /**
     * Detect potential memory leaks
     */
    detectMemoryLeaks() {
        const trends = this.metricsCache.memoryTrends;
        if (trends.length < 30) return; // Need at least 30 seconds of data
        
        const recent = trends.slice(-30); // Last 30 seconds
        const oldest = recent[0].memoryUsageMB;
        const newest = recent[recent.length - 1].memoryUsageMB;
        const increase = newest - oldest;
        
        if (increase > this.memoryConfig.leakDetectionThreshold) {
            this.notifySubscribers('memory_leak_detected', {
                memoryIncrease: increase,
                timeWindow: '30 seconds',
                suggestion: 'Memory usage increased significantly. Check for potential memory leaks.'
            });
        }
    }

    /**
     * Check memory usage against thresholds
     */
    checkMemoryThresholds(memoryUsageMB) {
        const { warning, critical } = this.memoryConfig.alertThresholds;
        
        if (memoryUsageMB > critical) {
            this.notifySubscribers('memory_alert', {
                level: 'critical',
                currentUsage: memoryUsageMB,
                threshold: critical,
                message: `Memory usage (${memoryUsageMB.toFixed(1)}MB) exceeded critical threshold (${critical}MB)`
            });
        } else if (memoryUsageMB > warning) {
            this.notifySubscribers('memory_alert', {
                level: 'warning',
                currentUsage: memoryUsageMB,
                threshold: warning,
                message: `Memory usage (${memoryUsageMB.toFixed(1)}MB) exceeded warning threshold (${warning}MB)`
            });
        }
    }

    /**
     * Process AI operation metrics
     */
    processAIOperations(searchMetrics) {
        if (!Array.isArray(searchMetrics)) {
            this.metricsCache.aiOperations = [];
            return;
        }

        // Convert search metrics to AI operations format
        const aiOperations = searchMetrics.map(metric => ({
            operationType: metric.operation_type || 'search',
            duration: metric.duration_ms || 0,
            vectorsSearched: metric.vectors_searched || 0,
            resultsReturned: metric.results_returned || 0,
            efficiencyScore: metric.efficiency_score || 0,
            performanceTargetMet: metric.performance_target_met || false,
            timestamp: new Date(metric.timestamp || Date.now()).getTime()
        }));

        // Sort by timestamp (newest first)
        aiOperations.sort((a, b) => b.timestamp - a.timestamp);
        
        this.metricsCache.aiOperations = aiOperations.slice(0, 50); // Keep latest 50 operations
        
        // Analyze AI operation performance
        this.analyzeAIOperationPerformance(aiOperations);
    }

    /**
     * Analyze AI operation performance trends
     */
    analyzeAIOperationPerformance(operations) {
        if (operations.length === 0) return;

        // Calculate average performance
        const avgDuration = operations.reduce((sum, op) => sum + op.duration, 0) / operations.length;
        const avgEfficiency = operations.reduce((sum, op) => sum + op.efficiencyScore, 0) / operations.length;
        
        // Check against performance targets
        const slowOperations = operations.filter(op => {
            const target = this.aiOperationTracking.performanceTargets[op.operationType] || 1000;
            return op.duration > target;
        });

        if (slowOperations.length > operations.length * 0.3) { // More than 30% are slow
            this.notifySubscribers('ai_performance_degradation', {
                averageDuration: avgDuration,
                averageEfficiency: avgEfficiency,
                slowOperationCount: slowOperations.length,
                totalOperations: operations.length,
                message: 'AI operations are performing below target levels'
            });
        }
    }

    /**
     * Handle metrics collection errors with retry logic
     */
    async handleCollectionError(error) {
        console.error('Metrics collection error:', error);
        
        this.retryConfig.currentRetries++;
        
        if (this.retryConfig.currentRetries <= this.retryConfig.maxRetries) {
            console.log(`Retrying metrics collection (${this.retryConfig.currentRetries}/${this.retryConfig.maxRetries})`);
            
            // Wait before retrying
            setTimeout(() => {
                if (this.isActive) {
                    this.collectAndProcessMetrics();
                }
            }, this.retryConfig.retryDelay);
        } else {
            console.error('Max retries exceeded. Stopping metrics collection.');
            this.notifySubscribers('collection_failed', { error: error.message });
        }
    }

    /**
     * Subscribe to real-time metrics updates
     */
    subscribe(callback) {
        this.subscribers.add(callback);
        
        // Send current cached data to new subscriber
        if (this.metricsCache.lastUpdate) {
            callback('initial_data', {
                currentMetrics: this.metricsCache.currentMetrics,
                resourceUtilization: this.metricsCache.resourceUtilization,
                memoryTrends: this.metricsCache.memoryTrends,
                aiOperations: this.metricsCache.aiOperations,
                alerts: this.metricsCache.alerts
            });
        }
        
        return () => {
            this.subscribers.delete(callback);
        };
    }

    /**
     * Notify all subscribers of updates
     */
    notifySubscribers(eventType, data) {
        this.subscribers.forEach(callback => {
            try {
                callback(eventType, data);
            } catch (error) {
                console.error('Error in metrics subscriber callback:', error);
            }
        });
    }

    /**
     * Get current metrics cache
     */
    getCurrentMetrics() {
        return {
            ...this.metricsCache,
            isActive: this.isActive,
            lastUpdate: this.metricsCache.lastUpdate
        };
    }

    /**
     * Get memory usage statistics
     */
    getMemoryStatistics() {
        const trends = this.metricsCache.memoryTrends;
        if (trends.length === 0) {
            return null;
        }

        const memoryValues = trends.map(t => t.memoryUsageMB);
        const current = memoryValues[memoryValues.length - 1];
        const min = Math.min(...memoryValues);
        const max = Math.max(...memoryValues);
        const avg = memoryValues.reduce((sum, val) => sum + val, 0) / memoryValues.length;

        return {
            current,
            min,
            max,
            average: avg,
            trend: this.calculateMemoryTrend(trends),
            dataPoints: trends.length
        };
    }

    /**
     * Calculate memory usage trend
     */
    calculateMemoryTrend(trends) {
        if (trends.length < 10) {
            return 'insufficient_data';
        }

        const recent = trends.slice(-10);
        const older = trends.slice(-20, -10);
        
        if (older.length === 0) {
            return 'insufficient_data';
        }

        const recentAvg = recent.reduce((sum, t) => sum + t.memoryUsageMB, 0) / recent.length;
        const olderAvg = older.reduce((sum, t) => sum + t.memoryUsageMB, 0) / older.length;
        
        const change = ((recentAvg - olderAvg) / olderAvg) * 100;
        
        if (change > 10) {
            return 'increasing';
        } else if (change < -10) {
            return 'decreasing';
        } else {
            return 'stable';
        }
    }

    /**
     * Force metrics collection (for testing/debugging)
     */
    async forceCollection() {
        if (!this.isActive) {
            throw new Error('Metrics service is not active');
        }
        
        await this.collectAndProcessMetrics();
    }

    /**
     * Get service status
     */
    getStatus() {
        return {
            isActive: this.isActive,
            subscriberCount: this.subscribers.size,
            lastUpdate: this.metricsCache.lastUpdate,
            retryCount: this.retryConfig.currentRetries,
            memoryTrendCount: this.metricsCache.memoryTrends.length,
            aiOperationCount: this.metricsCache.aiOperations.length,
            alertCount: this.metricsCache.alerts.length
        };
    }
}

// Create and export a singleton instance
const realTimeMetricsService = new RealTimeMetricsService();

export { realTimeMetricsService, RealTimeMetricsService };