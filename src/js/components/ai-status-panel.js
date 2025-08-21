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
        
        // Bind methods
        this.handleStatusCheck = this.handleStatusCheck.bind(this);
        this.handleConfigChange = this.handleConfigChange.bind(this);
        this.handleRetryConnection = this.handleRetryConnection.bind(this);
        this.handleSettingsToggle = this.handleSettingsToggle.bind(this);
        
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
        
        console.log('✅ AI Status Panel initialized');
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
        
        console.log('🔗 AI Status Panel event listeners attached');
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
            console.log('🤖 Ollama background monitoring started');
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
        console.log('🔄 [DEBUG] Retry Connection button clicked');
        
        // Disable retry button temporarily
        const retryBtn = document.getElementById('aiRetryBtn');
        if (retryBtn) {
            console.log('🔄 [DEBUG] Disabling retry button temporarily');
            retryBtn.disabled = true;
            retryBtn.textContent = 'Retrying...';
        }
        
        console.log('🔄 [DEBUG] Emitting CONNECTION_REQUESTED event with retry action');
        this.emitEvent(AiStatusPanel.EVENTS.CONNECTION_REQUESTED, {
            action: 'retry'
        });
        
        console.log('🔄 [DEBUG] Starting status check after retry button click');
        await this.handleStatusCheck();
        
        // Re-enable retry button
        if (retryBtn) {
            console.log('🔄 [DEBUG] Re-enabling retry button after status check');
            retryBtn.disabled = false;
            retryBtn.textContent = 'Retry Connection';
        }
        
        console.log('🔄 [DEBUG] Retry Connection process completed');
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
            
            console.log('✅ AI service configuration updated:', newUrl);
            
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

    /**
     * Clean up the component
     */
    destroy() {
        this.stopStatusMonitoring();
        
        // Remove event listeners
        const elements = [
            'aiStatusRefresh', 'aiRetryBtn', 'aiSettingsBtn', 
            'aiSettingsClose', 'aiSettingsForm', 'aiSettingsCancel', 'aiErrorClose'
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