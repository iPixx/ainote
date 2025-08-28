/**
 * AI Panel Component
 * 
 * Manages the AI panel lifecycle, activation, and integration with the layout system.
 * This component serves as the main controller for the AI panel's visibility and state.
 * 
 * @author aiNote Development Team
 * @version 1.0.0
 */

/**
 * AI Panel Component Class
 * Handles AI panel activation, visibility management, and integration with layout system
 */
class AiPanel {
    
    /**
     * Event names for the AI Panel
     */
    static EVENTS = {
        PANEL_ACTIVATED: 'ai_panel_activated',
        PANEL_DEACTIVATED: 'ai_panel_deactivated',
        PANEL_READY: 'ai_panel_ready'
    };

    /**
     * Initialize the AI Panel
     * @param {HTMLElement} panelElement - The AI panel container element
     * @param {Object} layoutManager - The layout manager instance
     */
    constructor(panelElement, layoutManager) {
        this.panel = panelElement;
        this.layoutManager = layoutManager;
        this.isActivated = false;
        this.isInitialized = false;
        
        // Configuration
        this.config = {
            defaultVisible: true, // Show AI panel by default per issue requirements
            animationDuration: 250,
            autoActivate: true // Activate panel on initialization
        };
        
        // Bind methods
        this.activate = this.activate.bind(this);
        this.deactivate = this.deactivate.bind(this);
        this.toggle = this.toggle.bind(this);
        this.handleVisibilityChange = this.handleVisibilityChange.bind(this);
        
        // Initialize the panel
        this.init();
    }

    /**
     * Initialize the AI Panel
     */
    init() {
        if (this.isInitialized) {
            console.warn('⚠️ AI Panel already initialized');
            return;
        }
        
        console.log('🤖 Initializing AI Panel...');
        
        // Ensure panel element exists
        if (!this.panel) {
            console.error('❌ AI Panel element not found');
            return;
        }
        
        // Set up initial state
        this.setupInitialState();
        
        // Set up event listeners
        this.setupEventListeners();
        
        // Activate the panel if auto-activation is enabled
        if (this.config.autoActivate) {
            // Delay activation slightly to ensure all components are ready
            setTimeout(() => {
                this.activate();
            }, 100);
        }
        
        this.isInitialized = true;
        console.log('✅ AI Panel initialized successfully');
        
        // Emit ready event
        this.emitEvent(AiPanel.EVENTS.PANEL_READY, {
            panel: this.panel,
            isActivated: this.isActivated
        });
    }

    /**
     * Set up initial panel state
     */
    setupInitialState() {
        // Ensure panel has proper structure and classes
        if (!this.panel.classList.contains('ai-panel')) {
            this.panel.classList.add('ai-panel');
        }
        
        // Set initial visibility based on configuration or saved state
        const savedState = this.getSavedVisibilityState();
        const shouldBeVisible = savedState !== null ? savedState : this.config.defaultVisible;
        
        if (shouldBeVisible) {
            this.show();
        } else {
            this.hide();
        }
        
        console.log('📋 AI Panel initial state set:', { visible: shouldBeVisible });
    }

    /**
     * Set up event listeners
     */
    setupEventListeners() {
        // Listen for layout changes that might affect the AI panel
        if (this.layoutManager) {
            // We'll integrate with layout manager events if available
            console.log('🔗 AI Panel event listeners setup with layout manager');
        }
        
        // Listen for window resize to handle responsive behavior
        window.addEventListener('resize', this.handleVisibilityChange);
        
        console.log('🔗 AI Panel event listeners attached');
    }

    /**
     * Activate the AI panel (make it available and potentially visible)
     */
    activate() {
        if (this.isActivated) {
            console.log('ℹ️ AI Panel already activated');
            return;
        }
        
        console.log('🚀 Activating AI Panel...');
        
        // Mark as activated
        this.isActivated = true;
        
        // Update panel classes
        this.panel.classList.add('ai-panel-activated');
        this.panel.classList.remove('ai-panel-deactivated');
        
        // Show the panel (this will also trigger layout updates)
        this.show();
        
        // Emit activation event
        this.emitEvent(AiPanel.EVENTS.PANEL_ACTIVATED, {
            panel: this.panel,
            timestamp: new Date().toISOString()
        });
        
        console.log('✅ AI Panel activated successfully');
    }

    /**
     * Deactivate the AI panel (hide it and mark as inactive)
     */
    deactivate() {
        if (!this.isActivated) {
            console.log('ℹ️ AI Panel already deactivated');
            return;
        }
        
        console.log('🔄 Deactivating AI Panel...');
        
        // Mark as deactivated
        this.isActivated = false;
        
        // Update panel classes
        this.panel.classList.add('ai-panel-deactivated');
        this.panel.classList.remove('ai-panel-activated');
        
        // Hide the panel
        this.hide();
        
        // Emit deactivation event
        this.emitEvent(AiPanel.EVENTS.PANEL_DEACTIVATED, {
            panel: this.panel,
            timestamp: new Date().toISOString()
        });
        
        console.log('✅ AI Panel deactivated successfully');
    }

    /**
     * Toggle AI panel activation state
     */
    toggle() {
        if (this.isActivated) {
            this.deactivate();
        } else {
            this.activate();
        }
    }

    /**
     * Show the AI panel (make it visible)
     */
    show() {
        console.log('👁️ Showing AI Panel...');
        
        // Remove display: none to make it visible
        this.panel.style.display = 'flex';
        
        // Add CSS class for styling
        this.panel.classList.add('ai-panel-visible');
        this.panel.classList.remove('ai-panel-hidden');
        
        // Update layout manager if available
        if (this.layoutManager && typeof this.layoutManager.toggleAiPanel === 'function') {
            // Only toggle if the layout manager state doesn't match
            if (!this.layoutManager.panelState?.aiPanelVisible) {
                console.log('🔄 Synchronizing with layout manager...');
                this.layoutManager.toggleAiPanel();
            }
        }
        
        // Save state
        this.saveVisibilityState(true);
        
        console.log('✅ AI Panel shown');
    }

    /**
     * Hide the AI panel (make it invisible)
     */
    hide() {
        console.log('🙈 Hiding AI Panel...');
        
        // Set display: none to hide it
        this.panel.style.display = 'none';
        
        // Update CSS classes
        this.panel.classList.add('ai-panel-hidden');
        this.panel.classList.remove('ai-panel-visible');
        
        // Update layout manager if available
        if (this.layoutManager && typeof this.layoutManager.toggleAiPanel === 'function') {
            // Only toggle if the layout manager state doesn't match
            if (this.layoutManager.panelState?.aiPanelVisible) {
                console.log('🔄 Synchronizing with layout manager...');
                this.layoutManager.toggleAiPanel();
            }
        }
        
        // Save state
        this.saveVisibilityState(false);
        
        console.log('✅ AI Panel hidden');
    }

    /**
     * Check if the AI panel is currently visible
     * @returns {boolean} True if visible, false otherwise
     */
    isVisible() {
        return this.panel.style.display !== 'none' && 
               this.panel.classList.contains('ai-panel-visible');
    }

    /**
     * Check if the AI panel is activated
     * @returns {boolean} True if activated, false otherwise
     */
    isActive() {
        return this.isActivated;
    }

    /**
     * Handle visibility change events (like window resize)
     */
    handleVisibilityChange() {
        // Handle responsive behavior
        const isMobile = window.innerWidth < 768;
        
        if (isMobile && this.isVisible()) {
            console.log('📱 Mobile mode detected - managing AI panel visibility');
            // On mobile, the AI panel might need special handling
            // This follows the existing mobile behavior in the CSS
        }
    }

    /**
     * Get saved visibility state from local storage or layout manager
     * @returns {boolean|null} Saved state or null if no saved state
     */
    getSavedVisibilityState() {
        try {
            // Try to get from layout manager first
            if (this.layoutManager && this.layoutManager.panelState) {
                return this.layoutManager.panelState.aiPanelVisible;
            }
            
            // Fallback to localStorage
            const saved = localStorage.getItem('aiPanel.visible');
            return saved ? JSON.parse(saved) : null;
        } catch (error) {
            console.warn('⚠️ Failed to get saved AI panel state:', error);
            return null;
        }
    }

    /**
     * Save visibility state to storage
     * @param {boolean} isVisible - Whether the panel is visible
     */
    saveVisibilityState(isVisible) {
        try {
            // Save to localStorage as fallback
            localStorage.setItem('aiPanel.visible', JSON.stringify(isVisible));
            console.log('💾 AI Panel visibility state saved:', isVisible);
        } catch (error) {
            console.warn('⚠️ Failed to save AI panel state:', error);
        }
    }

    /**
     * Update panel configuration
     * @param {Object} newConfig - Configuration updates
     */
    updateConfig(newConfig) {
        this.config = { ...this.config, ...newConfig };
        console.log('⚙️ AI Panel configuration updated:', this.config);
    }

    /**
     * Get current panel configuration
     * @returns {Object} Current configuration
     */
    getConfig() {
        return { ...this.config };
    }

    /**
     * Get panel element reference
     * @returns {HTMLElement} The panel element
     */
    getElement() {
        return this.panel;
    }

    /**
     * Emit custom event
     * @param {string} eventName - Name of the event
     * @param {Object} detail - Event detail data
     */
    emitEvent(eventName, detail) {
        const event = new CustomEvent(eventName, { detail });
        this.panel.dispatchEvent(event);
        
        // Also emit on document for global listeners
        document.dispatchEvent(event);
    }

    /**
     * Add event listener for panel events
     * @param {string} eventName - Name of the event to listen for
     * @param {Function} handler - Event handler function
     */
    addEventListener(eventName, handler) {
        this.panel.addEventListener(eventName, handler);
    }

    /**
     * Remove event listener
     * @param {string} eventName - Name of the event
     * @param {Function} handler - Event handler function
     */
    removeEventListener(eventName, handler) {
        this.panel.removeEventListener(eventName, handler);
    }

    /**
     * Refresh panel content (delegate to child components)
     */
    refresh() {
        console.log('🔄 Refreshing AI Panel content...');
        
        // Emit refresh event for child components to handle
        this.emitEvent('ai_panel_refresh', {
            timestamp: new Date().toISOString()
        });
        
        console.log('✅ AI Panel refresh completed');
    }

    /**
     * Clean up the component
     */
    destroy() {
        console.log('🧹 Destroying AI Panel...');
        
        // Remove event listeners
        window.removeEventListener('resize', this.handleVisibilityChange);
        
        // Clear state
        this.isActivated = false;
        this.isInitialized = false;
        
        // Remove CSS classes
        this.panel.classList.remove(
            'ai-panel-activated', 
            'ai-panel-deactivated',
            'ai-panel-visible',
            'ai-panel-hidden'
        );
        
        console.log('✅ AI Panel destroyed');
    }
}

export default AiPanel;