/**
 * Ollama Connection Monitor Service
 * 
 * Provides automatic background monitoring of Ollama service connection
 * with health checks, reconnection attempts, and status updates.
 * 
 * Features:
 * - Periodic health checks (every 30 seconds)
 * - Automatic reconnection with exponential backoff
 * - Connection status tracking and notifications
 * - Model availability monitoring
 * - Performance metrics collection
 * - UI integration with status indicators
 * 
 * @author aiNote Development Team
 * @version 1.0.0
 */

const { invoke } = window.__TAURI__.core;

/**
 * Ollama Connection Monitor Service Class
 */
class OllamaConnectionMonitor {
    
    /**
     * Monitor events
     */
    static EVENTS = {
        STATUS_CHANGED: 'ollama_status_changed',
        HEALTH_CHECK_COMPLETED: 'ollama_health_check_completed',
        RECONNECTION_ATTEMPT: 'ollama_reconnection_attempt',
        MODEL_STATUS_UPDATED: 'ollama_model_status_updated',
        PERFORMANCE_UPDATE: 'ollama_performance_update',
        ERROR_OCCURRED: 'ollama_error_occurred'
    };

    /**
     * Connection status types
     */
    static STATUS = {
        CONNECTED: 'Connected',
        CONNECTING: 'Connecting', 
        DISCONNECTED: 'Disconnected',
        FAILED: 'Failed',
        RETRYING: 'Retrying'
    };

    /**
     * Configuration defaults
     */
    static DEFAULTS = {
        HEALTH_CHECK_INTERVAL: 30000,    // 30 seconds
        RECONNECT_INTERVAL: 5000,        // 5 seconds initial retry
        MAX_RECONNECT_ATTEMPTS: 10,      // Maximum retry attempts
        RECONNECT_BACKOFF: 1.5,          // Exponential backoff multiplier
        MODEL_CHECK_INTERVAL: 60000,     // 1 minute for model checks
        REQUIRED_MODEL: 'nomic-embed-text',
        PERFORMANCE_SAMPLE_SIZE: 10      // Number of samples for performance averaging
    };

    /**
     * Initialize the Ollama Connection Monitor
     */
    constructor() {
        
        // Service state
        this.isRunning = false;
        this.currentStatus = null;
        this.lastHealthCheck = null;
        this.consecutiveFailures = 0;
        this.reconnectAttempts = 0;
        this.nextReconnectTime = null;
        
        // Monitoring intervals
        this.healthCheckInterval = null;
        this.reconnectTimeout = null;
        this.modelCheckInterval = null;
        
        // Configuration
        this.config = { ...OllamaConnectionMonitor.DEFAULTS };
        
        // Performance tracking
        this.performanceMetrics = {
            responseTimeHistory: [],
            averageResponseTime: 0,
            healthCheckCount: 0,
            successfulChecks: 0,
            failedChecks: 0,
            uptime: 0,
            startTime: null
        };
        
        // Model status tracking
        this.modelStatus = {
            isAvailable: false,
            isCompatible: false,
            lastChecked: null,
            downloadInProgress: false
        };
        
        // Event listeners
        this.eventListeners = new Map();
        
    }

    /**
     * Start automatic connection monitoring
     */
    async start() {
        if (this.isRunning) {
            console.log('⚠️ Ollama Connection Monitor already running');
            return;
        }

        
        this.isRunning = true;
        this.performanceMetrics.startTime = Date.now();
        this.reconnectAttempts = 0;

        try {
            // Initialize backend monitoring
            await this.initializeBackendMonitoring();
            
            // Start periodic health checks
            this.startHealthChecks();
            
            // Start model status monitoring
            this.startModelMonitoring();
            
            // Perform initial status check
            await this.performHealthCheck();
            
            
        } catch (error) {
            console.error('❌ Failed to start Ollama Connection Monitor:', error);
            this.handleError('Failed to start monitoring', error);
        }
    }

    /**
     * Stop connection monitoring
     */
    stop() {
        if (!this.isRunning) {
            return;
        }

        console.log('🛑 Stopping Ollama Connection Monitor...');
        
        this.isRunning = false;
        
        // Clear intervals
        if (this.healthCheckInterval) {
            clearInterval(this.healthCheckInterval);
            this.healthCheckInterval = null;
        }
        
        if (this.modelCheckInterval) {
            clearInterval(this.modelCheckInterval);
            this.modelCheckInterval = null;
        }
        
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
        
    }

    /**
     * Initialize backend monitoring service
     * @private
     */
    async initializeBackendMonitoring() {
        try {
            await invoke('start_ollama_monitoring');
        } catch (error) {
            console.warn('⚠️ Backend monitoring initialization failed (may work in limited mode):', error);
            // Don't fail completely - frontend monitoring can still work
        }
    }

    /**
     * Start periodic health checks
     * @private
     */
    startHealthChecks() {
        
        this.healthCheckInterval = setInterval(async () => {
            if (this.isRunning) {
                await this.performHealthCheck();
            }
        }, this.config.HEALTH_CHECK_INTERVAL);
    }

    /**
     * Start model status monitoring
     * @private
     */
    startModelMonitoring() {
        
        this.modelCheckInterval = setInterval(async () => {
            if (this.isRunning && this.currentStatus === OllamaConnectionMonitor.STATUS.CONNECTED) {
                await this.checkModelStatus();
            }
        }, this.config.MODEL_CHECK_INTERVAL);
    }

    /**
     * Perform health check
     * @private
     */
    async performHealthCheck() {
        const startTime = performance.now();
        
        try {
            console.log('🔍 Performing Ollama health check...');
            
            // Check Ollama status via backend
            const connectionState = await invoke('check_ollama_status');
            const responseTime = performance.now() - startTime;
            
            // Update performance metrics
            this.updatePerformanceMetrics(responseTime, true);
            
            // Process status update
            await this.processStatusUpdate(connectionState);
            
            // Reset failure counters on success
            this.consecutiveFailures = 0;
            this.reconnectAttempts = 0;
            
            // Emit health check completed event
            this.emit(OllamaConnectionMonitor.EVENTS.HEALTH_CHECK_COMPLETED, {
                connectionState,
                responseTime,
                timestamp: Date.now()
            });
            
        } catch (error) {
            const responseTime = performance.now() - startTime;
            console.error('❌ Health check failed:', error);
            
            // Update performance metrics for failure
            this.updatePerformanceMetrics(responseTime, false);
            
            // Handle connection failure
            await this.handleConnectionFailure(error);
            
            this.emit(OllamaConnectionMonitor.EVENTS.ERROR_OCCURRED, {
                error: error.message,
                type: 'health_check_failed',
                timestamp: Date.now()
            });
        }
        
        this.lastHealthCheck = Date.now();
    }

    /**
     * Process status update from health check
     * @param {Object} connectionState - Connection state from backend
     * @private
     */
    async processStatusUpdate(connectionState) {
        const statusType = this.getStatusType(connectionState.status);
        const previousStatus = this.currentStatus;
        
        // Update current status
        this.currentStatus = statusType;
        
        console.log(`📊 Ollama status: ${previousStatus} → ${statusType}`);
        
        // Emit status change if different
        if (previousStatus !== statusType) {
            this.emit(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, {
                previousStatus,
                currentStatus: statusType,
                connectionState,
                timestamp: Date.now()
            });
        }
        
        // Handle reconnection success
        if (statusType === OllamaConnectionMonitor.STATUS.CONNECTED && 
            previousStatus !== OllamaConnectionMonitor.STATUS.CONNECTED) {
            console.log('🎉 Ollama connection restored!');
            
            // Check model status when connection is restored
            setTimeout(() => this.checkModelStatus(), 1000);
        }
    }

    /**
     * Handle connection failure
     * @param {Error} error - Connection error
     * @private
     */
    async handleConnectionFailure(error) {
        this.consecutiveFailures++;
        
        console.log(`❌ Connection failure #${this.consecutiveFailures}: ${error.message}`);
        
        // Update status to failed/retrying
        const previousStatus = this.currentStatus;
        this.currentStatus = this.reconnectAttempts > 0 ? 
            OllamaConnectionMonitor.STATUS.RETRYING : 
            OllamaConnectionMonitor.STATUS.FAILED;
        
        // Emit status change
        if (previousStatus !== this.currentStatus) {
            this.emit(OllamaConnectionMonitor.EVENTS.STATUS_CHANGED, {
                previousStatus,
                currentStatus: this.currentStatus,
                error: error.message,
                consecutiveFailures: this.consecutiveFailures,
                timestamp: Date.now()
            });
        }
        
        // Schedule reconnection attempt
        this.scheduleReconnection();
    }

    /**
     * Schedule reconnection attempt with exponential backoff
     * @private
     */
    scheduleReconnection() {
        if (this.reconnectAttempts >= this.config.MAX_RECONNECT_ATTEMPTS) {
            console.log('❌ Maximum reconnection attempts reached');
            this.currentStatus = OllamaConnectionMonitor.STATUS.FAILED;
            return;
        }
        
        // Calculate backoff delay
        const baseDelay = this.config.RECONNECT_INTERVAL;
        const backoffMultiplier = Math.pow(this.config.RECONNECT_BACKOFF, this.reconnectAttempts);
        const delay = Math.min(baseDelay * backoffMultiplier, 60000); // Cap at 1 minute
        
        this.reconnectAttempts++;
        this.nextReconnectTime = Date.now() + delay;
        
        
        // Clear existing timeout
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
        }
        
        // Schedule reconnection
        this.reconnectTimeout = setTimeout(async () => {
            if (this.isRunning) {
                
                this.emit(OllamaConnectionMonitor.EVENTS.RECONNECTION_ATTEMPT, {
                    attempt: this.reconnectAttempts,
                    maxAttempts: this.config.MAX_RECONNECT_ATTEMPTS,
                    delay,
                    timestamp: Date.now()
                });
                
                await this.performHealthCheck();
            }
        }, delay);
    }

    /**
     * Check model status
     * @private
     */
    async checkModelStatus() {
        try {
            
            const modelVerification = await invoke('verify_model', { 
                modelName: this.config.REQUIRED_MODEL 
            });
            
            const wasAvailable = this.modelStatus.isAvailable;
            
            // Update model status
            this.modelStatus = {
                isAvailable: modelVerification.is_available,
                isCompatible: modelVerification.is_compatible === 'Compatible',
                lastChecked: Date.now(),
                downloadInProgress: false,
                info: modelVerification.info
            };
            
            
            // Emit model status update if changed
            if (wasAvailable !== this.modelStatus.isAvailable) {
                this.emit(OllamaConnectionMonitor.EVENTS.MODEL_STATUS_UPDATED, {
                    modelName: this.config.REQUIRED_MODEL,
                    ...this.modelStatus,
                    timestamp: Date.now()
                });
            }
            
        } catch (error) {
            console.warn('⚠️ Model status check failed:', error);
            this.modelStatus.lastChecked = Date.now();
        }
    }

    /**
     * Update performance metrics
     * @param {number} responseTime - Response time in milliseconds  
     * @param {boolean} success - Whether the request was successful
     * @private
     */
    updatePerformanceMetrics(responseTime, success) {
        this.performanceMetrics.healthCheckCount++;
        
        if (success) {
            this.performanceMetrics.successfulChecks++;
            
            // Update response time history
            this.performanceMetrics.responseTimeHistory.push(responseTime);
            
            // Keep only recent samples
            if (this.performanceMetrics.responseTimeHistory.length > this.config.PERFORMANCE_SAMPLE_SIZE) {
                this.performanceMetrics.responseTimeHistory.shift();
            }
            
            // Calculate average response time
            this.performanceMetrics.averageResponseTime = 
                this.performanceMetrics.responseTimeHistory.reduce((a, b) => a + b, 0) / 
                this.performanceMetrics.responseTimeHistory.length;
                
        } else {
            this.performanceMetrics.failedChecks++;
        }
        
        // Calculate uptime percentage
        const totalChecks = this.performanceMetrics.healthCheckCount;
        this.performanceMetrics.uptime = totalChecks > 0 ? 
            (this.performanceMetrics.successfulChecks / totalChecks) * 100 : 0;
        
        // Emit performance update periodically
        if (this.performanceMetrics.healthCheckCount % 5 === 0) {
            this.emit(OllamaConnectionMonitor.EVENTS.PERFORMANCE_UPDATE, {
                ...this.performanceMetrics,
                timestamp: Date.now()
            });
        }
    }

    /**
     * Get simplified status type from connection status
     * @param {Object} status - Status object from backend
     * @returns {string} Status type
     * @private
     */
    getStatusType(status) {
        if (typeof status === 'string') {
            return status;
        }
        
        if (status.Connected !== undefined) return OllamaConnectionMonitor.STATUS.CONNECTED;
        if (status.Disconnected !== undefined) return OllamaConnectionMonitor.STATUS.DISCONNECTED;
        if (status.Connecting !== undefined) return OllamaConnectionMonitor.STATUS.CONNECTING;
        if (status.Retrying !== undefined) return OllamaConnectionMonitor.STATUS.RETRYING;
        if (status.Failed !== undefined) return OllamaConnectionMonitor.STATUS.FAILED;
        
        return OllamaConnectionMonitor.STATUS.DISCONNECTED;
    }

    /**
     * Handle service errors
     * @param {string} message - Error message
     * @param {Error} error - Error object
     * @private
     */
    handleError(message, error) {
        console.error(`❌ Ollama Connection Monitor: ${message}`, error);
        
        this.emit(OllamaConnectionMonitor.EVENTS.ERROR_OCCURRED, {
            message,
            error: error.message,
            stack: error.stack,
            timestamp: Date.now()
        });
    }

    /**
     * Manually trigger health check
     * @returns {Promise<Object>} Connection state
     */
    async checkNow() {
        console.log('🔍 Manual health check requested');
        await this.performHealthCheck();
        return this.getStatus();
    }

    /**
     * Force reconnection attempt
     */
    async forceReconnect() {
        
        // Cancel any pending reconnection
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
        
        // Reset attempts and perform check
        this.reconnectAttempts = 0;
        await this.performHealthCheck();
    }

    /**
     * Get current monitoring status
     * @returns {Object} Current status and metrics
     */
    getStatus() {
        return {
            isRunning: this.isRunning,
            currentStatus: this.currentStatus,
            lastHealthCheck: this.lastHealthCheck,
            consecutiveFailures: this.consecutiveFailures,
            reconnectAttempts: this.reconnectAttempts,
            nextReconnectTime: this.nextReconnectTime,
            performanceMetrics: { ...this.performanceMetrics },
            modelStatus: { ...this.modelStatus },
            config: { ...this.config }
        };
    }

    /**
     * Update monitoring configuration
     * @param {Object} newConfig - Configuration updates
     */
    updateConfig(newConfig) {
        const oldConfig = { ...this.config };
        this.config = { ...this.config, ...newConfig };
        
        console.log('⚙️ Ollama Connection Monitor configuration updated:', {
            old: oldConfig,
            new: this.config
        });
        
        // Restart monitoring with new configuration if needed
        if (this.isRunning) {
            // Restart intervals if timing changed
            if (oldConfig.HEALTH_CHECK_INTERVAL !== this.config.HEALTH_CHECK_INTERVAL) {
                this.stop();
                this.start();
            }
        }
    }

    /**
     * Add event listener
     * @param {string} eventType - Event type
     * @param {Function} handler - Event handler
     */
    addEventListener(eventType, handler) {
        if (!this.eventListeners.has(eventType)) {
            this.eventListeners.set(eventType, new Set());
        }
        this.eventListeners.get(eventType).add(handler);
    }

    /**
     * Remove event listener
     * @param {string} eventType - Event type
     * @param {Function} handler - Event handler
     */
    removeEventListener(eventType, handler) {
        const listeners = this.eventListeners.get(eventType);
        if (listeners) {
            listeners.delete(handler);
            if (listeners.size === 0) {
                this.eventListeners.delete(eventType);
            }
        }
    }

    /**
     * Emit event to listeners
     * @param {string} eventType - Event type
     * @param {Object} data - Event data
     * @private
     */
    emit(eventType, data) {
        const listeners = this.eventListeners.get(eventType);
        if (!listeners) return;

        listeners.forEach(handler => {
            try {
                handler(data);
            } catch (error) {
                console.error(`Error in Ollama monitor event handler for ${eventType}:`, error);
            }
        });
    }

    /**
     * Clean up resources
     */
    destroy() {
        console.log('🧹 Destroying Ollama Connection Monitor...');
        
        this.stop();
        this.eventListeners.clear();
        
    }
}

export default OllamaConnectionMonitor;