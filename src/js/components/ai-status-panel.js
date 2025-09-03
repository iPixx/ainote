/**
 * AI Status Panel Component
 * 
 * Manages the AI panel status indicators and connection UI for Ollama service
 * monitoring and configuration. Provides real-time updates of connection status
 * with user-friendly error messages and guidance.
 * 
 * @author aiNote Development Team
 * @version 1.0.0
 */

const { invoke } = window.__TAURI__.core;

/**
 * AI Status Panel Component Class
 * Handles display of Ollama connection status and user interactions
 */
class AiStatusPanel {
    
    /**
     * Event names for the AI Status Panel
     */
    static EVENTS = {
        STATUS_CHANGED: 'ai_status_changed',
        CONNECTION_REQUESTED: 'ai_connection_requested',
        SETTINGS_CHANGED: 'ai_settings_changed'
    };

    /**
     * Connection status types and their display configurations
     */
    static STATUS_CONFIG = {
        Connected: {
            icon: '🟢',
            color: 'var(--color-success)',
            label: 'Connected',
            description: 'AI features are available'
        },
        Disconnected: {
            icon: '🔴',
            color: 'var(--color-error)',
            label: 'Disconnected',
            description: 'AI features are unavailable'
        },
        Connecting: {
            icon: '🟡',
            color: 'var(--color-warning)',
            label: 'Connecting...',
            description: 'Establishing connection to AI service'
        },
        Retrying: {
            icon: '🟠',
            color: 'var(--color-warning)',
            label: 'Retrying...',
            description: 'Attempting to reconnect'
        },
        Failed: {
            icon: '❌',
            color: 'var(--color-error)',
            label: 'Failed',
            description: 'Connection failed - check service status'
        }
    };

    /**
     * Initialize the AI Status Panel
     * @param {HTMLElement} containerElement - The AI panel container element
     */
    constructor(containerElement) {
        this.container = containerElement;
        this.currentStatus = null;
        this.currentConfig = { base_url: 'http://localhost:11434' };
        this.statusCheckInterval = null;
        this.isMonitoring = false;
        
        // Model management properties
        this.currentModelStatus = null;
        this.downloadProgress = null;
        this.progressCheckInterval = null;
        this.currentModelName = 'nomic-embed-text';
        
        // Bind methods
        this.handleStatusCheck = this.handleStatusCheck.bind(this);
        this.handleConfigChange = this.handleConfigChange.bind(this);
        this.handleRetryConnection = this.handleRetryConnection.bind(this);
        this.handleSettingsToggle = this.handleSettingsToggle.bind(this);
        
        // Bind model management methods
        this.handleModelRefresh = this.handleModelRefresh.bind(this);
        this.handleDownloadModel = this.handleDownloadModel.bind(this);
        this.handleCancelDownload = this.handleCancelDownload.bind(this);
        this.handleVerifyModel = this.handleVerifyModel.bind(this);
        this.updateModelStatus = this.updateModelStatus.bind(this);
        this.updateDownloadProgress = this.updateDownloadProgress.bind(this);
        
        // Initialize the UI
        this.init();
    }

    /**
     * Initialize the AI Status Panel UI
     */
    init() {
        this.render();
        this.attachEventListeners();
        this.startStatusMonitoring();
        
    }

    /**
     * Render the AI status panel UI structure
     */
    render() {
        this.container.innerHTML = `
            <div class="ai-status-container">
                <!-- Status Display Section -->
                <div class="ai-status-display" id="aiStatusDisplay">
                    <div class="status-header">
                        <h3 class="status-title">AI Service Status</h3>
                        <button class="status-refresh-btn" id="aiStatusRefresh" aria-label="Refresh status" title="Refresh connection status">
                            🔄
                        </button>
                    </div>
                    
                    <div class="status-indicator" id="aiStatusIndicator">
                        <div class="status-icon" id="aiStatusIcon">🔄</div>
                        <div class="status-info">
                            <div class="status-label" id="aiStatusLabel">Checking...</div>
                            <div class="status-description" id="aiStatusDescription">Verifying AI service connection</div>
                        </div>
                    </div>
                    
                    <!-- Connection Details -->
                    <div class="connection-details" id="aiConnectionDetails">
                        <div class="detail-item">
                            <span class="detail-label">Endpoint:</span>
                            <span class="detail-value" id="aiEndpointValue">http://localhost:11434</span>
                        </div>
                        <div class="detail-item">
                            <span class="detail-label">Last Check:</span>
                            <span class="detail-value" id="aiLastCheckValue">Never</span>
                        </div>
                        <div class="detail-item" id="aiRetryInfo" style="display: none;">
                            <span class="detail-label">Next Retry:</span>
                            <span class="detail-value" id="aiRetryValue">-</span>
                        </div>
                    </div>
                    
                    <!-- Action Buttons -->
                    <div class="status-actions">
                        <button class="btn-primary ai-action-btn" id="aiRetryBtn" style="display: none;">
                            Retry Connection
                        </button>
                        <button class="btn-secondary ai-action-btn" id="aiSettingsBtn">
                            ⚙️ Settings
                        </button>
                    </div>
                </div>

                <!-- Model Management Section -->
                <div class="model-status-panel" id="modelStatusPanel" style="display: none;">
                    <div class="model-header">
                        <h3 class="model-title">Model Management</h3>
                        <button class="model-refresh-btn" id="modelRefreshBtn" aria-label="Refresh models" title="Refresh model list">
                            🔄
                        </button>
                    </div>
                    
                    <!-- Model Status Display -->
                    <div class="model-status-display" id="modelStatusDisplay">
                        <div class="model-indicator" id="modelIndicator">
                            <div class="model-status-icon" id="modelStatusIcon">📦</div>
                            <div class="model-info">
                                <div class="model-name" id="modelName">nomic-embed-text</div>
                                <div class="model-status-label" id="modelStatusLabel">Checking...</div>
                            </div>
                        </div>
                        
                        <!-- Model Actions -->
                        <div class="model-actions" id="modelActions">
                            <button class="btn-primary model-action-btn" id="downloadModelBtn" style="display: none;">
                                📥 Download Model
                            </button>
                            <button class="btn-secondary model-action-btn" id="cancelDownloadBtn" style="display: none;">
                                ❌ Cancel Download
                            </button>
                            <button class="btn-secondary model-action-btn" id="verifyModelBtn" style="display: none;">
                                ✅ Verify Model
                            </button>
                        </div>
                    </div>
                    
                    <!-- Download Progress Section -->
                    <div class="download-progress-section" id="downloadProgressSection" style="display: none;">
                        <div class="progress-header">
                            <span class="progress-title" id="progressTitle">Downloading Model...</span>
                            <span class="progress-percentage" id="progressPercentage">0%</span>
                        </div>
                        
                        <div class="progress-bar-container">
                            <div class="progress-bar" id="progressBar">
                                <div class="progress-fill" id="progressFill" style="width: 0%;"></div>
                            </div>
                        </div>
                        
                        <div class="progress-details">
                            <div class="progress-detail-item">
                                <span class="detail-label">Downloaded:</span>
                                <span class="detail-value" id="downloadedSize">0 MB</span>
                            </div>
                            <div class="progress-detail-item">
                                <span class="detail-label">Total Size:</span>
                                <span class="detail-value" id="totalSize">Unknown</span>
                            </div>
                            <div class="progress-detail-item">
                                <span class="detail-label">Speed:</span>
                                <span class="detail-value" id="downloadSpeed">0 MB/s</span>
                            </div>
                            <div class="progress-detail-item">
                                <span class="detail-label">ETA:</span>
                                <span class="detail-value" id="estimatedTime">Unknown</span>
                            </div>
                        </div>
                    </div>
                    
                    <!-- Model Performance Metrics -->
                    <div class="model-metrics-section" id="modelMetricsSection" style="display: none;">
                        <h4 class="metrics-title">Performance Metrics</h4>
                        <div class="metrics-grid">
                            <div class="metric-item">
                                <span class="metric-label">Model Size:</span>
                                <span class="metric-value" id="modelSizeMetric">Unknown</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">Load Time:</span>
                                <span class="metric-value" id="loadTimeMetric">Unknown</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">Compatibility:</span>
                                <span class="metric-value" id="compatibilityMetric">Unknown</span>
                            </div>
                            <div class="metric-item">
                                <span class="metric-label">Last Verified:</span>
                                <span class="metric-value" id="lastVerifiedMetric">Never</span>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Settings Configuration Section -->
                <div class="ai-settings-panel" id="aiSettingsPanel" style="display: none;">
                    <div class="settings-header">
                        <h3 class="settings-title">AI Service Configuration</h3>
                        <button class="settings-close-btn" id="aiSettingsClose" aria-label="Close settings">
                            ✕
                        </button>
                    </div>
                    
                    <form class="ai-settings-form" id="aiSettingsForm">
                        <div class="form-group">
                            <label for="ollamaUrl" class="form-label">Ollama Service URL</label>
                            <input 
                                type="url" 
                                id="ollamaUrl" 
                                class="form-input" 
                                value="http://localhost:11434" 
                                placeholder="http://localhost:11434"
                                required
                            >
                            <div class="form-help">
                                Enter the URL where your Ollama service is running
                            </div>
                        </div>
                        
                        <div class="form-actions">
                            <button type="submit" class="btn-primary">Save & Test Connection</button>
                            <button type="button" class="btn-secondary" id="aiSettingsCancel">Cancel</button>
                        </div>
                    </form>
                    
                    <div class="settings-help">
                        <h4>Need Help?</h4>
                        <ul>
                            <li>Make sure Ollama is installed and running</li>
                            <li>Default URL is usually <code>http://localhost:11434</code></li>
                            <li>Check your firewall settings if connection fails</li>
                            <li>Verify Ollama is not running on a different port</li>
                        </ul>
                    </div>
                </div>

                <!-- Error Messages -->
                <div class="ai-error-message" id="aiErrorMessage" style="display: none;">
                    <div class="error-icon">⚠️</div>
                    <div class="error-content">
                        <div class="error-title" id="aiErrorTitle">Connection Error</div>
                        <div class="error-description" id="aiErrorDescription">Unable to connect to AI service</div>
                    </div>
                    <button class="error-close" id="aiErrorClose" aria-label="Close error">✕</button>
                </div>
            </div>
        `;
        
        console.log('🎨 AI Status Panel UI rendered');
    }

    /**
     * Attach event listeners to UI elements
     */
    attachEventListeners() {
        // Status refresh button
        const refreshBtn = document.getElementById('aiStatusRefresh');
        if (refreshBtn) {
            refreshBtn.addEventListener('click', this.handleStatusCheck);
        }

        // Retry connection button
        const retryBtn = document.getElementById('aiRetryBtn');
        if (retryBtn) {
            retryBtn.addEventListener('click', this.handleRetryConnection);
        }

        // Settings panel toggle
        const settingsBtn = document.getElementById('aiSettingsBtn');
        if (settingsBtn) {
            settingsBtn.addEventListener('click', this.handleSettingsToggle);
        }

        // Settings panel close
        const settingsClose = document.getElementById('aiSettingsClose');
        if (settingsClose) {
            settingsClose.addEventListener('click', this.handleSettingsToggle);
        }

        // Settings form submission
        const settingsForm = document.getElementById('aiSettingsForm');
        if (settingsForm) {
            settingsForm.addEventListener('submit', this.handleConfigChange);
        }

        // Settings cancel
        const settingsCancel = document.getElementById('aiSettingsCancel');
        if (settingsCancel) {
            settingsCancel.addEventListener('click', this.handleSettingsToggle);
        }

        // Error message close
        const errorClose = document.getElementById('aiErrorClose');
        if (errorClose) {
            errorClose.addEventListener('click', () => this.hideErrorMessage());
        }

        // Model management event listeners
        const modelRefreshBtn = document.getElementById('modelRefreshBtn');
        if (modelRefreshBtn) {
            modelRefreshBtn.addEventListener('click', this.handleModelRefresh);
        }

        const downloadModelBtn = document.getElementById('downloadModelBtn');
        if (downloadModelBtn) {
            downloadModelBtn.addEventListener('click', this.handleDownloadModel);
        }

        const cancelDownloadBtn = document.getElementById('cancelDownloadBtn');
        if (cancelDownloadBtn) {
            cancelDownloadBtn.addEventListener('click', this.handleCancelDownload);
        }

        const verifyModelBtn = document.getElementById('verifyModelBtn');
        if (verifyModelBtn) {
            verifyModelBtn.addEventListener('click', this.handleVerifyModel);
        }
        
    }

    /**
     * Start periodic status monitoring
     */
    startStatusMonitoring() {
        if (this.isMonitoring) return;
        
        this.isMonitoring = true;
        
        // Initial status check
        this.checkStatus();
        
        // Start monitoring background process
        this.startOllamaMonitoring();
        
        // Set up periodic status checks (every 5 seconds)
        this.statusCheckInterval = setInterval(() => {
            this.checkStatus();
        }, 5000);
        
        console.log('📊 AI Status monitoring started');
    }

    /**
     * Stop status monitoring
     */
    stopStatusMonitoring() {
        if (!this.isMonitoring) return;
        
        this.isMonitoring = false;
        
        if (this.statusCheckInterval) {
            clearInterval(this.statusCheckInterval);
            this.statusCheckInterval = null;
        }
        
        console.log('📊 AI Status monitoring stopped');
    }

    /**
     * Start Ollama background monitoring
     */
    async startOllamaMonitoring() {
        try {
            await invoke('start_ollama_monitoring');
        } catch (error) {
            console.warn('⚠️ Failed to start Ollama monitoring:', error);
        }
    }

    /**
     * Check current Ollama service status
     */
    async checkStatus() {
        console.log('📊 [DEBUG] checkStatus() started - invoking backend check_ollama_status command');
        console.log('📊 [DEBUG] Current config:', { base_url: this.currentConfig.base_url });
        
        try {
            console.log('📊 [DEBUG] Calling Tauri invoke("check_ollama_status")');
            const connectionState = await invoke('check_ollama_status');
            
            console.log('📊 [DEBUG] Backend returned connection state:', connectionState);
            console.log('📊 [DEBUG] Connection status type:', this.getStatusType(connectionState.status));
            console.log('📊 [DEBUG] Last check time:', connectionState.last_check);
            console.log('📊 [DEBUG] Retry count:', connectionState.retry_count);
            
            console.log('📊 [DEBUG] Calling updateStatus() with received connection state');
            this.updateStatus(connectionState);
            
            console.log('📊 [DEBUG] checkStatus() completed successfully');
        } catch (error) {
            console.error('❌ [DEBUG] Failed to check Ollama status:', error);
            console.log('❌ [DEBUG] Error type:', typeof error);
            console.log('❌ [DEBUG] Error details:', error);
            
            // Handle different types of errors gracefully
            let errorMessage = error.toString();
            console.log('❌ [DEBUG] Original error message:', errorMessage);
            
            // Check if this is a Tauri command error
            if (errorMessage.includes('command not found') || errorMessage.includes('invoke error')) {
                console.log('❌ [DEBUG] Detected Tauri command error - backend may be unavailable');
                errorMessage = 'AI service commands not available. This may be a development build or backend issue.';
            } else if (errorMessage.includes('fetch') || errorMessage.includes('network')) {
                console.log('❌ [DEBUG] Detected network error');
                errorMessage = 'Network error: Unable to reach AI service. Check your connection.';
            } else if (errorMessage.includes('permission')) {
                console.log('❌ [DEBUG] Detected permission error');
                errorMessage = 'Permission error: Unable to access AI service. Check application permissions.';
            }
            
            console.log('❌ [DEBUG] Processed error message:', errorMessage);
            
            // Create a fallback connection state
            const fallbackState = {
                status: { Failed: { error: errorMessage } },
                last_check: new Date().toISOString(),
                last_successful_connection: null,
                retry_count: 0,
                next_retry_at: null,
                health_info: null
            };
            
            console.log('❌ [DEBUG] Creating fallback connection state:', fallbackState);
            console.log('❌ [DEBUG] Calling updateStatus() with fallback state');
            this.updateStatus(fallbackState);
        }
    }

    /**
     * Update the UI with current status information
     * @param {Object} connectionState - The connection state from backend
     */
    updateStatus(connectionState) {
        console.log('🎨 [DEBUG] updateStatus() called with connection state:', connectionState);
        
        this.currentStatus = connectionState;
        
        const statusType = this.getStatusType(connectionState.status);
        console.log('🎨 [DEBUG] Determined status type:', statusType);
        
        const config = AiStatusPanel.STATUS_CONFIG[statusType];
        
        if (!config) {
            console.warn('🎨 [DEBUG] Unknown status type:', statusType);
            return;
        }
        
        console.log('🎨 [DEBUG] Using status config:', config);

        // Update status indicator
        console.log('🎨 [DEBUG] Updating status indicator');
        this.updateStatusIndicator(statusType, config, connectionState);
        
        // Update connection details
        console.log('🎨 [DEBUG] Updating connection details');
        this.updateConnectionDetails(connectionState);
        
        // Update action buttons
        console.log('🎨 [DEBUG] Updating action buttons for status:', statusType);
        this.updateActionButtons(statusType);
        
        // Handle error states
        if (statusType === 'Failed' || statusType === 'Disconnected') {
            console.log('🎨 [DEBUG] Handling connection error for status:', statusType);
            this.handleConnectionError(connectionState);
        } else {
            console.log('🎨 [DEBUG] Hiding error message - status is healthy:', statusType);
            this.hideErrorMessage();
        }
        
        // Update model panel visibility based on connection status
        console.log('🎨 [DEBUG] Updating model panel for status:', statusType);
        this.showModelPanel(statusType);
        
        // Emit status change event
        console.log('🎨 [DEBUG] Emitting STATUS_CHANGED event');
        this.emitEvent(AiStatusPanel.EVENTS.STATUS_CHANGED, {
            status: statusType,
            connectionState: connectionState
        });
        
        console.log('🎨 [DEBUG] updateStatus() completed');
    }

    /**
     * Get simplified status type from connection status
     * @param {Object} status - The status object from backend
     * @returns {string} Status type string
     */
    getStatusType(status) {
        if (typeof status === 'string') {
            return status;
        }
        
        if (status.Connected !== undefined) return 'Connected';
        if (status.Disconnected !== undefined) return 'Disconnected';
        if (status.Connecting !== undefined) return 'Connecting';
        if (status.Retrying !== undefined) return 'Retrying';
        if (status.Failed !== undefined) return 'Failed';
        
        return 'Disconnected';
    }

    /**
     * Update the status indicator UI elements
     * @param {string} statusType - The status type
     * @param {Object} config - The status configuration
     * @param {Object} connectionState - The full connection state
     */
    updateStatusIndicator(statusType, config, connectionState) {
        const icon = document.getElementById('aiStatusIcon');
        const label = document.getElementById('aiStatusLabel');
        const description = document.getElementById('aiStatusDescription');
        
        if (icon) {
            icon.textContent = config.icon;
            icon.style.color = config.color;
        }
        
        if (label) {
            label.textContent = config.label;
            label.style.color = config.color;
        }
        
        if (description) {
            let desc = config.description;
            
            // Add retry information for retrying state
            if (statusType === 'Retrying' && connectionState.status.Retrying) {
                const attempt = connectionState.status.Retrying.attempt;
                desc = `${desc} (attempt ${attempt})`;
            }
            
            description.textContent = desc;
        }
    }

    /**
     * Update connection details section
     * @param {Object} connectionState - The connection state
     */
    updateConnectionDetails(connectionState) {
        const endpointValue = document.getElementById('aiEndpointValue');
        const lastCheckValue = document.getElementById('aiLastCheckValue');
        const retryInfo = document.getElementById('aiRetryInfo');
        const retryValue = document.getElementById('aiRetryValue');
        
        if (endpointValue) {
            endpointValue.textContent = this.currentConfig.base_url;
        }
        
        if (lastCheckValue && connectionState.last_check) {
            const lastCheck = new Date(connectionState.last_check);
            lastCheckValue.textContent = lastCheck.toLocaleTimeString();
        }
        
        if (retryInfo && retryValue && connectionState.next_retry_at) {
            const nextRetry = new Date(connectionState.next_retry_at);
            const now = new Date();
            
            if (nextRetry > now) {
                retryInfo.style.display = 'block';
                const secondsUntilRetry = Math.ceil((nextRetry - now) / 1000);
                retryValue.textContent = `${secondsUntilRetry}s`;
            } else {
                retryInfo.style.display = 'none';
            }
        } else if (retryInfo) {
            retryInfo.style.display = 'none';
        }
    }

    /**
     * Update action buttons based on status
     * @param {string} statusType - The current status type
     */
    updateActionButtons(statusType) {
        console.log('🎯 [DEBUG] updateActionButtons() called with status type:', statusType);
        
        const retryBtn = document.getElementById('aiRetryBtn');
        
        if (retryBtn) {
            const shouldShowRetry = statusType === 'Failed' || statusType === 'Disconnected';
            console.log('🎯 [DEBUG] Should show retry button:', shouldShowRetry, '(status:', statusType, ')');
            
            const previousDisplay = retryBtn.style.display;
            retryBtn.style.display = shouldShowRetry ? 'block' : 'none';
            
            console.log('🎯 [DEBUG] Retry button display changed from', previousDisplay, 'to', retryBtn.style.display);
        } else {
            console.warn('🎯 [DEBUG] Retry button element not found in DOM');
        }
    }

    /**
     * Handle connection errors and display appropriate messages
     * @param {Object} connectionState - The connection state
     */
    handleConnectionError(connectionState) {
        let errorTitle = 'Connection Error';
        let errorDescription = 'Unable to connect to AI service';
        
        if (connectionState.status.Failed && connectionState.status.Failed.error) {
            const error = connectionState.status.Failed.error;
            
            if (error.includes('Connection refused')) {
                errorTitle = 'Service Not Running';
                errorDescription = 'Ollama service is not running. Please start Ollama and try again.';
            } else if (error.includes('timeout')) {
                errorTitle = 'Connection Timeout';
                errorDescription = 'The AI service is not responding. Check your network connection.';
            } else if (error.includes('404') || error.includes('Not Found')) {
                errorTitle = 'Service Not Found';
                errorDescription = 'AI service endpoint not found. Check the URL configuration.';
            } else {
                errorDescription = error;
            }
        }
        
        this.showErrorMessage(errorTitle, errorDescription);
    }

    /**
     * Show error message to user
     * @param {string} title - Error title
     * @param {string} description - Error description
     */
    showErrorMessage(title, description) {
        const errorMessage = document.getElementById('aiErrorMessage');
        const errorTitle = document.getElementById('aiErrorTitle');
        const errorDescription = document.getElementById('aiErrorDescription');
        
        if (errorMessage && errorTitle && errorDescription) {
            errorTitle.textContent = title;
            errorDescription.textContent = description;
            errorMessage.style.display = 'flex';
        }
    }

    /**
     * Hide error message
     */
    hideErrorMessage() {
        const errorMessage = document.getElementById('aiErrorMessage');
        if (errorMessage) {
            errorMessage.style.display = 'none';
        }
    }

    /**
     * Handle status check button click
     */
    async handleStatusCheck() {
        console.log('🔍 [DEBUG] handleStatusCheck() called');
        
        const refreshBtn = document.getElementById('aiStatusRefresh');
        if (refreshBtn) {
            console.log('🔍 [DEBUG] Disabling refresh button and setting loading state');
            refreshBtn.disabled = true;
            refreshBtn.textContent = '⏳';
        }
        
        console.log('🔍 [DEBUG] Calling checkStatus() method');
        await this.checkStatus();
        
        if (refreshBtn) {
            console.log('🔍 [DEBUG] Re-enabling refresh button and resetting icon');
            refreshBtn.disabled = false;
            refreshBtn.textContent = '🔄';
        }
        
        console.log('🔍 [DEBUG] handleStatusCheck() completed');
    }

    /**
     * Handle retry connection button click
     */
    async handleRetryConnection() {
        
        // Disable retry button temporarily
        const retryBtn = document.getElementById('aiRetryBtn');
        if (retryBtn) {
            retryBtn.disabled = true;
            retryBtn.textContent = 'Retrying...';
        }
        
        this.emitEvent(AiStatusPanel.EVENTS.CONNECTION_REQUESTED, {
            action: 'retry'
        });
        
        await this.handleStatusCheck();
        
        // Re-enable retry button
        if (retryBtn) {
            retryBtn.disabled = false;
            retryBtn.textContent = 'Retry Connection';
        }
        
    }

    /**
     * Handle settings panel toggle
     */
    handleSettingsToggle() {
        const settingsPanel = document.getElementById('aiSettingsPanel');
        const statusDisplay = document.getElementById('aiStatusDisplay');
        
        if (settingsPanel && statusDisplay) {
            const isVisible = settingsPanel.style.display !== 'none';
            settingsPanel.style.display = isVisible ? 'none' : 'block';
            statusDisplay.style.display = isVisible ? 'block' : 'none';
            
            if (!isVisible) {
                // Populate current URL
                const urlInput = document.getElementById('ollamaUrl');
                if (urlInput) {
                    urlInput.value = this.currentConfig.base_url;
                }
            }
        }
    }

    /**
     * Handle configuration change
     * @param {Event} event - Form submission event
     */
    async handleConfigChange(event) {
        event.preventDefault();
        
        const urlInput = document.getElementById('ollamaUrl');
        const newUrl = urlInput?.value?.trim();
        
        if (!newUrl) {
            this.showErrorMessage('Invalid URL', 'Please enter a valid Ollama service URL');
            return;
        }
        
        // Basic URL validation
        try {
            const url = new URL(newUrl);
            if (!['http:', 'https:'].includes(url.protocol)) {
                throw new Error('URL must use http:// or https:// protocol');
            }
        } catch (urlError) {
            this.showErrorMessage('Invalid URL', 'Please enter a valid URL (e.g., http://localhost:11434)');
            return;
        }
        
        try {
            // Update configuration in backend
            await invoke('configure_ollama_url', { baseUrl: newUrl });
            
            // Update local config
            this.currentConfig.base_url = newUrl;
            
            // Hide settings panel
            this.handleSettingsToggle();
            
            // Check status with new configuration
            await this.handleStatusCheck();
            
            // Emit settings change event
            this.emitEvent(AiStatusPanel.EVENTS.SETTINGS_CHANGED, {
                baseUrl: newUrl
            });
            
            
        } catch (error) {
            console.error('❌ Failed to update AI configuration:', error);
            
            // Provide graceful degradation - update local config even if backend fails
            this.currentConfig.base_url = newUrl;
            
            let errorMessage = error.toString();
            if (errorMessage.includes('command not found')) {
                errorMessage = 'Backend configuration unavailable. URL saved locally for when service becomes available.';
                
                // Still proceed with UI updates
                this.handleSettingsToggle();
                await this.handleStatusCheck();
                
                this.emitEvent(AiStatusPanel.EVENTS.SETTINGS_CHANGED, {
                    baseUrl: newUrl
                });
                
                console.warn('⚠️ Backend config failed, but proceeding with local config:', newUrl);
            } else {
                this.showErrorMessage('Configuration Error', `Failed to update settings: ${errorMessage}`);
            }
        }
    }

    /**
     * Emit custom event
     * @param {string} eventName - Name of the event
     * @param {Object} detail - Event detail data
     */
    emitEvent(eventName, detail) {
        const event = new CustomEvent(eventName, { detail });
        this.container.dispatchEvent(event);
    }

    /**
     * Add event listener for component events
     * @param {string} eventName - Name of the event to listen for
     * @param {Function} handler - Event handler function
     */
    addEventListener(eventName, handler) {
        this.container.addEventListener(eventName, handler);
    }

    /**
     * Remove event listener
     * @param {string} eventName - Name of the event
     * @param {Function} handler - Event handler function
     */
    removeEventListener(eventName, handler) {
        this.container.removeEventListener(eventName, handler);
    }

    /**
     * Get current connection status
     * @returns {Object|null} Current connection state
     */
    getConnectionStatus() {
        return this.currentStatus;
    }

    /**
     * Get current configuration
     * @returns {Object} Current AI service configuration
     */
    getConfiguration() {
        return { ...this.currentConfig };
    }

    // === MODEL MANAGEMENT METHODS ===

    /**
     * Show or hide the model status panel based on connection status
     * @param {string} connectionStatus - Current AI connection status
     */
    showModelPanel(connectionStatus) {
        const modelPanel = document.getElementById('modelStatusPanel');
        const statusDisplay = document.getElementById('aiStatusDisplay');
        
        if (modelPanel && statusDisplay) {
            const shouldShow = connectionStatus === 'Connected';
            console.log('🎨 [DEBUG] Model panel should show:', shouldShow, 'for status:', connectionStatus);
            
            if (shouldShow) {
                statusDisplay.style.display = 'block';
                modelPanel.style.display = 'block';
                // Start checking model status
                this.checkModelStatus();
            } else {
                modelPanel.style.display = 'none';
                // Stop any ongoing progress monitoring
                this.stopProgressMonitoring();
            }
        }
    }

    /**
     * Check current model status
     */
    async checkModelStatus() {
        try {
            console.log('🔍 Checking model status for:', this.currentModelName);
            
            // Update UI to show checking state
            this.updateModelIndicator('checking', 'Checking model status...');
            
            const verification = await invoke('verify_model', { modelName: this.currentModelName });
            this.currentModelStatus = verification;
            
            console.log('📊 Model verification result:', verification);
            
            // Update UI based on model status
            this.updateModelStatus(verification);
            this.updateModelMetrics(verification);
            
        } catch (error) {
            console.error('❌ Failed to check model status:', error);
            this.updateModelIndicator('error', `Error: ${error}`);
            this.showModelError('Model Status Check Failed', error.toString());
        }
    }

    /**
     * Update model status display
     * @param {Object} verification - Model verification result
     */
    updateModelStatus(verification) {
        if (!verification) return;

        const { is_available, is_compatible, model_name } = verification;
        
        // Update model name display
        const modelNameEl = document.getElementById('modelName');
        if (modelNameEl) {
            modelNameEl.textContent = model_name;
        }

        // Determine status and actions
        if (is_available) {
            if (is_compatible === 'Compatible') {
                this.updateModelIndicator('available', '✅ Model Available & Compatible');
                this.showModelActions(['verify']);
            } else if (is_compatible?.Incompatible) {
                this.updateModelIndicator('incompatible', `⚠️ Model Incompatible: ${is_compatible.Incompatible.reason}`);
                this.showModelActions(['download']);
            } else {
                this.updateModelIndicator('unknown', '❓ Model Available (Compatibility Unknown)');
                this.showModelActions(['verify', 'download']);
            }
        } else {
            this.updateModelIndicator('missing', '❌ Model Not Available');
            this.showModelActions(['download']);
        }
    }

    /**
     * Update model indicator UI
     * @param {string} status - Status type (checking, available, missing, incompatible, error)
     * @param {string} label - Status label text
     */
    updateModelIndicator(status, label) {
        const icon = document.getElementById('modelStatusIcon');
        const labelEl = document.getElementById('modelStatusLabel');
        const indicator = document.getElementById('modelIndicator');
        
        if (icon && labelEl && indicator) {
            // Remove all status classes
            indicator.classList.remove('status-checking', 'status-available', 'status-missing', 'status-incompatible', 'status-error');
            
            // Add current status class
            indicator.classList.add(`status-${status}`);
            
            // Update content
            labelEl.textContent = label;
            
            // Update icon based on status
            const statusIcons = {
                checking: '⏳',
                available: '✅',
                missing: '❌',
                incompatible: '⚠️',
                error: '💥'
            };
            icon.textContent = statusIcons[status] || '📦';
        }
    }

    /**
     * Show/hide model action buttons
     * @param {Array} actions - Array of action names to show (['download', 'cancel', 'verify'])
     */
    showModelActions(actions = []) {
        const downloadBtn = document.getElementById('downloadModelBtn');
        const cancelBtn = document.getElementById('cancelDownloadBtn');
        const verifyBtn = document.getElementById('verifyModelBtn');
        
        // Hide all buttons first
        [downloadBtn, cancelBtn, verifyBtn].forEach(btn => {
            if (btn) btn.style.display = 'none';
        });
        
        // Show requested actions
        actions.forEach(action => {
            const actionButtons = {
                download: downloadBtn,
                cancel: cancelBtn,
                verify: verifyBtn
            };
            
            const button = actionButtons[action];
            if (button) {
                button.style.display = 'block';
            }
        });
    }

    /**
     * Update model performance metrics
     * @param {Object} verification - Model verification result
     */
    updateModelMetrics(verification) {
        if (!verification?.info) return;

        const { info, verification_time_ms } = verification;
        const metricsSection = document.getElementById('modelMetricsSection');
        
        if (metricsSection) {
            metricsSection.style.display = 'block';
            
            // Update individual metrics
            this.updateMetric('modelSizeMetric', this.formatBytes(info.size));
            this.updateMetric('loadTimeMetric', `${verification_time_ms}ms`);
            this.updateMetric('compatibilityMetric', this.formatCompatibility(verification.is_compatible));
            this.updateMetric('lastVerifiedMetric', new Date().toLocaleTimeString());
        }
    }

    /**
     * Update a specific metric value
     * @param {string} metricId - Element ID of the metric
     * @param {string} value - New value to display
     */
    updateMetric(metricId, value) {
        const element = document.getElementById(metricId);
        if (element) {
            element.textContent = value || 'Unknown';
        }
    }

    /**
     * Handle model refresh button click
     */
    async handleModelRefresh() {
        
        const refreshBtn = document.getElementById('modelRefreshBtn');
        if (refreshBtn) {
            refreshBtn.disabled = true;
            refreshBtn.textContent = '⏳';
        }
        
        await this.checkModelStatus();
        
        if (refreshBtn) {
            refreshBtn.disabled = false;
            refreshBtn.textContent = '🔄';
        }
    }

    /**
     * Handle download model button click
     */
    async handleDownloadModel() {
        console.log('📥 Download model requested:', this.currentModelName);
        
        try {
            // Show download in progress
            this.updateModelIndicator('checking', 'Starting download...');
            this.showModelActions(['cancel']);
            
            // Start the download
            const downloadProgress = await invoke('download_model', { modelName: this.currentModelName });
            console.log('📊 Download started:', downloadProgress);
            
            // Show progress section and start monitoring
            this.showDownloadProgress(true);
            this.startProgressMonitoring();
            
        } catch (error) {
            console.error('❌ Failed to start download:', error);
            this.updateModelIndicator('error', `Download failed: ${error}`);
            this.showModelError('Download Failed', error.toString());
            this.showModelActions(['download']);
        }
    }

    /**
     * Handle cancel download button click
     */
    async handleCancelDownload() {
        console.log('❌ Cancel download requested:', this.currentModelName);
        
        try {
            await invoke('cancel_download', { modelName: this.currentModelName });
            
            // Update UI
            this.updateModelIndicator('missing', 'Download cancelled');
            this.showDownloadProgress(false);
            this.stopProgressMonitoring();
            this.showModelActions(['download']);
            
        } catch (error) {
            console.error('❌ Failed to cancel download:', error);
            this.showModelError('Cancel Failed', error.toString());
        }
    }

    /**
     * Handle verify model button click
     */
    async handleVerifyModel() {
        
        const verifyBtn = document.getElementById('verifyModelBtn');
        if (verifyBtn) {
            verifyBtn.disabled = true;
            verifyBtn.textContent = '⏳ Verifying...';
        }
        
        await this.checkModelStatus();
        
        if (verifyBtn) {
            verifyBtn.disabled = false;
            verifyBtn.textContent = '✅ Verify Model';
        }
    }

    /**
     * Show or hide download progress section
     * @param {boolean} show - Whether to show the progress section
     */
    showDownloadProgress(show) {
        const progressSection = document.getElementById('downloadProgressSection');
        if (progressSection) {
            progressSection.style.display = show ? 'block' : 'none';
        }
    }

    /**
     * Start monitoring download progress
     */
    startProgressMonitoring() {
        // Clear any existing interval
        this.stopProgressMonitoring();
        
        // Start checking progress every 500ms
        this.progressCheckInterval = setInterval(async () => {
            try {
                const progress = await invoke('get_download_progress', { modelName: this.currentModelName });
                if (progress) {
                    this.updateDownloadProgress(progress);
                    
                    // Check if download is complete
                    if (progress.status?.Completed) {
                        this.handleDownloadComplete(progress);
                    } else if (progress.status?.Failed) {
                        this.handleDownloadFailed(progress);
                    } else if (progress.status?.Cancelled) {
                        this.handleDownloadCancelled(progress);
                    }
                }
            } catch (error) {
                console.error('❌ Failed to get download progress:', error);
            }
        }, 500);
        
        console.log('📊 Started progress monitoring');
    }

    /**
     * Stop monitoring download progress
     */
    stopProgressMonitoring() {
        if (this.progressCheckInterval) {
            clearInterval(this.progressCheckInterval);
            this.progressCheckInterval = null;
            console.log('📊 Stopped progress monitoring');
        }
    }

    /**
     * Update download progress display
     * @param {Object} progress - Download progress information
     */
    updateDownloadProgress(progress) {
        this.downloadProgress = progress;
        
        if (progress.status?.Downloading) {
            const { progress_percent, downloaded_bytes, total_bytes, speed_bytes_per_sec } = progress.status.Downloading;
            
            // Update progress bar
            const progressFill = document.getElementById('progressFill');
            const progressPercentage = document.getElementById('progressPercentage');
            
            if (progressFill && progressPercentage) {
                progressFill.style.width = `${progress_percent}%`;
                progressPercentage.textContent = `${Math.round(progress_percent)}%`;
            }
            
            // Update progress details
            this.updateDownloadDetail('downloadedSize', this.formatBytes(downloaded_bytes));
            this.updateDownloadDetail('totalSize', this.formatBytes(total_bytes));
            this.updateDownloadDetail('downloadSpeed', this.formatSpeed(speed_bytes_per_sec));
            
            // Calculate and show ETA
            if (speed_bytes_per_sec && total_bytes && downloaded_bytes < total_bytes) {
                const remaining = total_bytes - downloaded_bytes;
                const etaSeconds = remaining / speed_bytes_per_sec;
                this.updateDownloadDetail('estimatedTime', this.formatTime(etaSeconds));
            }
            
            // Update status indicator
            this.updateModelIndicator('checking', `Downloading... ${Math.round(progress_percent)}%`);
        }
    }

    /**
     * Update a download detail field
     * @param {string} fieldId - Element ID of the detail field
     * @param {string} value - Value to display
     */
    updateDownloadDetail(fieldId, value) {
        const element = document.getElementById(fieldId);
        if (element) {
            element.textContent = value || 'Unknown';
        }
    }

    /**
     * Handle download completion
     * @param {Object} progress - Final progress information
     */
    async handleDownloadComplete(progress) {
        
        this.stopProgressMonitoring();
        this.showDownloadProgress(false);
        
        // Update UI to show completion
        this.updateModelIndicator('available', '✅ Download Complete - Verifying...');
        
        // Refresh model status to verify the download
        await this.checkModelStatus();
    }

    /**
     * Handle download failure
     * @param {Object} progress - Progress information with error
     */
    handleDownloadFailed(progress) {
        console.error('❌ Download failed:', progress);
        
        this.stopProgressMonitoring();
        this.showDownloadProgress(false);
        
        const errorMsg = progress.status?.Failed?.error || 'Unknown error';
        this.updateModelIndicator('error', `Download failed: ${errorMsg}`);
        this.showModelError('Download Failed', errorMsg);
        this.showModelActions(['download']);
    }

    /**
     * Handle download cancellation
     * @param {Object} progress - Progress information
     */
    handleDownloadCancelled(progress) {
        console.log('⚠️ Download cancelled:', progress);
        
        this.stopProgressMonitoring();
        this.showDownloadProgress(false);
        
        this.updateModelIndicator('missing', 'Download cancelled');
        this.showModelActions(['download']);
    }

    /**
     * Show model-specific error message
     * @param {string} title - Error title
     * @param {string} description - Error description
     */
    showModelError(title, description) {
        // Reuse the existing error message component
        this.showErrorMessage(title, description);
    }

    /**
     * Format bytes to human-readable format
     * @param {number} bytes - Number of bytes
     * @returns {string} Formatted string (e.g., "1.2 GB")
     */
    formatBytes(bytes) {
        if (!bytes || bytes === 0) return '0 B';
        
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(1024));
        
        return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
    }

    /**
     * Format speed to human-readable format
     * @param {number} bytesPerSecond - Speed in bytes per second
     * @returns {string} Formatted string (e.g., "1.2 MB/s")
     */
    formatSpeed(bytesPerSecond) {
        if (!bytesPerSecond || bytesPerSecond === 0) return '0 B/s';
        return this.formatBytes(bytesPerSecond) + '/s';
    }

    /**
     * Format time duration to human-readable format
     * @param {number} seconds - Duration in seconds
     * @returns {string} Formatted string (e.g., "2m 30s")
     */
    formatTime(seconds) {
        if (!seconds || seconds <= 0) return 'Unknown';
        
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        const secs = Math.floor(seconds % 60);
        
        if (hours > 0) {
            return `${hours}h ${minutes}m`;
        } else if (minutes > 0) {
            return `${minutes}m ${secs}s`;
        } else {
            return `${secs}s`;
        }
    }

    /**
     * Format model compatibility for display
     * @param {Object|string} compatibility - Compatibility information
     * @returns {string} Formatted compatibility string
     */
    formatCompatibility(compatibility) {
        if (typeof compatibility === 'string') {
            return compatibility;
        }
        
        if (compatibility === 'Compatible') {
            return '✅ Compatible';
        } else if (compatibility?.Incompatible) {
            return '❌ Incompatible';
        } else {
            return '❓ Unknown';
        }
    }

    /**
     * Clean up the component
     */
    destroy() {
        this.stopStatusMonitoring();
        this.stopProgressMonitoring();
        
        // Remove event listeners
        const elements = [
            'aiStatusRefresh', 'aiRetryBtn', 'aiSettingsBtn', 
            'aiSettingsClose', 'aiSettingsForm', 'aiSettingsCancel', 'aiErrorClose',
            'modelRefreshBtn', 'downloadModelBtn', 'cancelDownloadBtn', 'verifyModelBtn'
        ];
        
        elements.forEach(id => {
            const element = document.getElementById(id);
            if (element) {
                element.removeEventListener('click', this.handleStatusCheck);
                element.removeEventListener('click', this.handleRetryConnection);
                element.removeEventListener('click', this.handleSettingsToggle);
                element.removeEventListener('submit', this.handleConfigChange);
                element.removeEventListener('click', () => this.hideErrorMessage());
            }
        });
        
        console.log('🧹 AI Status Panel destroyed');
    }
}

export default AiStatusPanel;