/**
 * Indexing Status Panel Component
 * 
 * Manages the display of vault indexing progress and status within the AI panel.
 * Provides real-time updates on indexing operations with user-friendly progress
 * indicators and controls for managing indexing operations.
 * 
 * @author aiNote Development Team - Phase 2C Implementation
 * @version 1.0.0
 */

const { invoke } = window.__TAURI__.core;

/**
 * Indexing Status Panel Component Class
 * Handles display of indexing progress, file monitoring status, and user interactions
 */
class IndexingStatusPanel {
    
    /**
     * Event names for the Indexing Status Panel
     */
    static EVENTS = {
        STATUS_CHANGED: 'indexing_status_changed',
        PROGRESS_UPDATED: 'indexing_progress_updated',
        INDEXING_STARTED: 'indexing_started',
        INDEXING_COMPLETED: 'indexing_completed',
        INDEXING_CANCELLED: 'indexing_cancelled'
    };

    /**
     * Status configurations for different indexing states
     */
    static STATUS_CONFIG = {
        Idle: {
            icon: '⏸️',
            color: 'var(--color-text-muted)',
            label: 'Ready',
            description: 'Indexing system ready'
        },
        Indexing: {
            icon: '🔄',
            color: 'var(--color-primary)',
            label: 'Indexing...',
            description: 'Processing vault files for AI search'
        },
        Completed: {
            icon: '✅',
            color: 'var(--color-success)',
            label: 'Completed',
            description: 'All files indexed successfully'
        },
        Error: {
            icon: '❌',
            color: 'var(--color-error)',
            label: 'Error',
            description: 'Indexing failed - check logs for details'
        },
        Cancelling: {
            icon: '🛑',
            color: 'var(--color-warning)',
            label: 'Stopping...',
            description: 'Cancelling indexing operation'
        }
    };

    /**
     * Initialize the Indexing Status Panel
     * @param {HTMLElement} containerElement - The AI panel container element
     */
    constructor(containerElement) {
        this.container = containerElement;
        this.currentStatus = 'Idle';
        this.currentProgress = null;
        this.progressInterval = null;
        this.isMonitoring = false;
        
        // Configuration
        this.config = {
            updateInterval: 1000, // Update every second
            enableFileMonitoringDisplay: true,
            showDetailedProgress: true
        };
        
        // Bind methods
        this.updateProgress = this.updateProgress.bind(this);
        this.handleStartIndexing = this.handleStartIndexing.bind(this);
        this.handleCancelIndexing = this.handleCancelIndexing.bind(this);
        this.refreshStatus = this.refreshStatus.bind(this);
        
        // Initialize the UI
        this.init();
    }

    /**
     * Initialize the component
     */
    init() {
        console.log('🔍 Initializing Indexing Status Panel...');
        
        // Ensure container element exists
        if (!this.container) {
            console.error('❌ Indexing Status Panel container element not found');
            return;
        }
        
        // Create the UI structure
        this.createUI();
        
        // Set up initial state
        this.updateStatus('Idle');
        
        // Start monitoring for progress updates
        this.startMonitoring();
        
        // Initial status refresh
        this.refreshStatus();
        
    }

    /**
     * Create the UI structure for the indexing status panel
     */
    createUI() {
        const html = `
            <div class="indexing-status-panel">
                <div class="indexing-header">
                    <h3 class="indexing-title">
                        <span class="indexing-icon" id="indexingIcon">🔍</span>
                        Vault Indexing
                    </h3>
                    <div class="indexing-controls">
                        <button class="control-btn" id="refreshIndexingBtn" onclick="indexingStatusPanel.refreshStatus()" 
                                aria-label="Refresh indexing status" title="Refresh Status">
                            🔄
                        </button>
                    </div>
                </div>
                
                <div class="indexing-status-section">
                    <div class="status-indicator" id="indexingStatusIndicator">
                        <span class="status-icon" id="statusIcon">⏸️</span>
                        <div class="status-info">
                            <div class="status-label" id="statusLabel">Ready</div>
                            <div class="status-description" id="statusDescription">Indexing system ready</div>
                        </div>
                    </div>
                </div>

                <div class="indexing-progress-section" id="progressSection" style="display: none;">
                    <div class="progress-header">
                        <span class="progress-title">Progress</span>
                        <span class="progress-percentage" id="progressPercentage">0%</span>
                    </div>
                    <div class="progress-bar-container">
                        <div class="progress-bar" id="progressBar">
                            <div class="progress-fill" id="progressFill" style="width: 0%;"></div>
                        </div>
                    </div>
                    <div class="progress-details" id="progressDetails">
                        <div class="progress-files">
                            <span class="detail-label">Files:</span>
                            <span class="detail-value" id="progressFiles">0 / 0</span>
                        </div>
                        <div class="progress-speed">
                            <span class="detail-label">Speed:</span>
                            <span class="detail-value" id="progressSpeed">0 files/sec</span>
                        </div>
                        <div class="progress-eta">
                            <span class="detail-label">ETA:</span>
                            <span class="detail-value" id="progressEta">-</span>
                        </div>
                    </div>
                </div>

                <div class="indexing-actions-section">
                    <div class="action-buttons">
                        <button class="btn-secondary" id="startIndexingBtn" onclick="indexingStatusPanel.handleStartIndexing()" 
                                style="display: none;">
                            Start Indexing
                        </button>
                        <button class="btn-warning" id="cancelIndexingBtn" onclick="indexingStatusPanel.handleCancelIndexing()" 
                                style="display: none;">
                            Cancel
                        </button>
                    </div>
                </div>

                <div class="file-monitoring-section">
                    <div class="monitoring-status">
                        <span class="monitoring-icon" id="monitoringIcon">👁️</span>
                        <div class="monitoring-info">
                            <div class="monitoring-label">File Monitoring</div>
                            <div class="monitoring-description" id="monitoringDescription">
                                Real-time file change detection
                            </div>
                        </div>
                        <div class="monitoring-toggle">
                            <span class="monitoring-status-text" id="monitoringStatusText">Active</span>
                        </div>
                    </div>
                </div>

                <div class="indexing-stats-section" id="statsSection" style="display: none;">
                    <div class="stats-header">
                        <span class="stats-title">Statistics</span>
                    </div>
                    <div class="stats-grid">
                        <div class="stat-item">
                            <span class="stat-label">Indexed Files</span>
                            <span class="stat-value" id="statIndexedFiles">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Failed Files</span>
                            <span class="stat-value" id="statFailedFiles">0</span>
                        </div>
                        <div class="stat-item">
                            <span class="stat-label">Queue Size</span>
                            <span class="stat-value" id="statQueueSize">0</span>
                        </div>
                    </div>
                </div>
            </div>
        `;
        
        this.container.innerHTML = html;
        
        // Store references to important elements
        this.statusIcon = document.getElementById('statusIcon');
        this.statusLabel = document.getElementById('statusLabel');
        this.statusDescription = document.getElementById('statusDescription');
        this.progressSection = document.getElementById('progressSection');
        this.progressPercentage = document.getElementById('progressPercentage');
        this.progressFill = document.getElementById('progressFill');
        this.progressFiles = document.getElementById('progressFiles');
        this.progressSpeed = document.getElementById('progressSpeed');
        this.progressEta = document.getElementById('progressEta');
        this.startIndexingBtn = document.getElementById('startIndexingBtn');
        this.cancelIndexingBtn = document.getElementById('cancelIndexingBtn');
        this.monitoringStatusText = document.getElementById('monitoringStatusText');
        this.statsSection = document.getElementById('statsSection');
    }

    /**
     * Update the indexing status display
     * @param {string} status - The current status
     * @param {string} [errorMessage] - Optional error message
     */
    updateStatus(status, errorMessage = null) {
        this.currentStatus = status;
        
        const config = IndexingStatusPanel.STATUS_CONFIG[status] || IndexingStatusPanel.STATUS_CONFIG.Idle;
        
        if (this.statusIcon) this.statusIcon.textContent = config.icon;
        if (this.statusLabel) this.statusLabel.textContent = config.label;
        if (this.statusDescription) {
            this.statusDescription.textContent = errorMessage || config.description;
        }
        
        // Update button visibility based on status
        this.updateActionButtons(status);
        
        // Emit status change event
        this.emitEvent(IndexingStatusPanel.EVENTS.STATUS_CHANGED, {
            status,
            config,
            errorMessage
        });
    }

    /**
     * Update action buttons based on current status
     * @param {string} status - Current indexing status
     */
    updateActionButtons(status) {
        if (this.startIndexingBtn && this.cancelIndexingBtn) {
            switch (status) {
                case 'Idle':
                case 'Completed':
                case 'Error':
                    this.startIndexingBtn.style.display = 'inline-block';
                    this.cancelIndexingBtn.style.display = 'none';
                    break;
                case 'Indexing':
                    this.startIndexingBtn.style.display = 'none';
                    this.cancelIndexingBtn.style.display = 'inline-block';
                    break;
                case 'Cancelling':
                    this.startIndexingBtn.style.display = 'none';
                    this.cancelIndexingBtn.style.display = 'none';
                    break;
            }
        }
    }

    /**
     * Update progress display with current indexing progress
     * @param {Object} progress - Progress information
     */
    updateProgress(progress) {
        this.currentProgress = progress;
        
        if (!progress) {
            if (this.progressSection) {
                this.progressSection.style.display = 'none';
            }
            return;
        }
        
        // Show progress section
        if (this.progressSection) {
            this.progressSection.style.display = 'block';
        }
        
        // Update progress percentage and bar
        const percentage = Math.round(progress.progress_percent || 0);
        if (this.progressPercentage) {
            this.progressPercentage.textContent = `${percentage}%`;
        }
        if (this.progressFill) {
            this.progressFill.style.width = `${percentage}%`;
        }
        
        // Update file counts
        if (this.progressFiles) {
            this.progressFiles.textContent = `${progress.completed_files || 0} / ${progress.total_files || 0}`;
        }
        
        // Update processing speed
        if (this.progressSpeed) {
            const speed = progress.files_per_second || 0;
            this.progressSpeed.textContent = `${speed.toFixed(1)} files/sec`;
        }
        
        // Update ETA
        if (this.progressEta) {
            const eta = progress.estimated_remaining_seconds || 0;
            if (eta > 0) {
                const minutes = Math.ceil(eta / 60);
                this.progressEta.textContent = `${minutes}m`;
            } else {
                this.progressEta.textContent = '-';
            }
        }
        
        // Update status based on progress
        if (progress.is_running) {
            if (progress.is_cancelling) {
                this.updateStatus('Cancelling');
            } else {
                this.updateStatus('Indexing');
            }
        } else if (progress.completed_files > 0 && progress.total_files > 0 && progress.completed_files >= progress.total_files) {
            this.updateStatus('Completed');
        }
        
        // Update statistics section
        this.updateStatistics(progress);
        
        // Emit progress event
        this.emitEvent(IndexingStatusPanel.EVENTS.PROGRESS_UPDATED, progress);
    }

    /**
     * Update statistics display
     * @param {Object} progress - Progress information
     */
    updateStatistics(progress) {
        if (!progress || !this.statsSection) return;
        
        // Show stats section if we have progress data
        this.statsSection.style.display = 'block';
        
        const statIndexedFiles = document.getElementById('statIndexedFiles');
        const statFailedFiles = document.getElementById('statFailedFiles');
        const statQueueSize = document.getElementById('statQueueSize');
        
        if (statIndexedFiles) statIndexedFiles.textContent = progress.completed_files || 0;
        if (statFailedFiles) statFailedFiles.textContent = progress.failed_files || 0;
        if (statQueueSize) statQueueSize.textContent = progress.queued_files || 0;
    }

    /**
     * Start monitoring indexing progress
     */
    startMonitoring() {
        if (this.isMonitoring) return;
        
        this.isMonitoring = true;
        this.progressInterval = setInterval(async () => {
            try {
                const progress = await invoke('get_indexing_progress');
                this.updateProgress(progress);
            } catch (error) {
                console.warn('⚠️ Failed to get indexing progress:', error);
            }
        }, this.config.updateInterval);
        
        console.log('👁️ Started indexing progress monitoring');
    }

    /**
     * Stop monitoring indexing progress
     */
    stopMonitoring() {
        if (!this.isMonitoring) return;
        
        this.isMonitoring = false;
        if (this.progressInterval) {
            clearInterval(this.progressInterval);
            this.progressInterval = null;
        }
        
        console.log('⏹️ Stopped indexing progress monitoring');
    }

    /**
     * Refresh current indexing status
     */
    async refreshStatus() {
        try {
            const status = await invoke('get_indexing_status');
            const progress = await invoke('get_indexing_progress');
            
            
            // Update progress first
            this.updateProgress(progress);
            
            // Update monitoring status
            if (this.monitoringStatusText) {
                // For now, assume monitoring is always active if we can get status
                this.monitoringStatusText.textContent = 'Active';
            }
            
        } catch (error) {
            console.warn('⚠️ Failed to refresh indexing status:', error);
            this.updateStatus('Error', 'Failed to get status');
        }
    }

    /**
     * Handle start indexing button click
     */
    async handleStartIndexing() {
        try {
            
            // For now, we'll just refresh status - actual start is handled by vault loading
            await this.refreshStatus();
            
            this.emitEvent(IndexingStatusPanel.EVENTS.INDEXING_STARTED);
            
        } catch (error) {
            console.error('❌ Failed to start indexing:', error);
            this.updateStatus('Error', 'Failed to start indexing');
        }
    }

    /**
     * Handle cancel indexing button click
     */
    async handleCancelIndexing() {
        try {
            console.log('🛑 Cancelling indexing...');
            
            this.updateStatus('Cancelling');
            
            await invoke('cancel_indexing');
            
            this.emitEvent(IndexingStatusPanel.EVENTS.INDEXING_CANCELLED);
            
            
            // Refresh status after a brief delay
            setTimeout(() => {
                this.refreshStatus();
            }, 1000);
            
        } catch (error) {
            console.error('❌ Failed to cancel indexing:', error);
            this.updateStatus('Error', 'Failed to cancel indexing');
        }
    }

    /**
     * Emit a custom event
     * @param {string} eventName - Name of the event
     * @param {*} data - Event data
     */
    emitEvent(eventName, data) {
        const event = new CustomEvent(eventName, { detail: data });
        document.dispatchEvent(event);
    }

    /**
     * Clean up resources
     */
    destroy() {
        this.stopMonitoring();
        
        if (this.container) {
            this.container.innerHTML = '';
        }
        
        console.log('🧹 Indexing Status Panel destroyed');
    }
}

// Global instance for easy access
let indexingStatusPanel = null;

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
    module.exports = IndexingStatusPanel;
}

// Auto-initialize when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
    // Initialize when AI panel content is available
    const initializeIfReady = () => {
        const aiContent = document.getElementById('aiContent');
        if (aiContent && !indexingStatusPanel) {
            indexingStatusPanel = new IndexingStatusPanel(aiContent);
        }
    };
    
    // Try immediate initialization
    initializeIfReady();
    
    // Also listen for potential AI panel activation
    document.addEventListener('ai_panel_activated', initializeIfReady);
});