/**
 * Performance Monitoring Dashboard Component
 * 
 * Real-time performance monitoring dashboard that displays comprehensive
 * metrics including memory usage, AI operation timing, UI responsiveness,
 * and exportable performance reports.
 * 
 * Features:
 * - Real-time metrics display with trend tracking
 * - Memory usage monitoring with threshold alerts
 * - AI operation timing and resource measurement  
 * - UI responsiveness metrics (frame time, input lag)
 * - Exportable performance reports
 * - Configurable monitoring intervals
 * - Performance alerts for threshold violations
 * - <1% CPU overhead requirement compliance
 */

// Use mocked invoke in test environment, actual Tauri invoke in production
const invoke = window.__TAURI__?.core?.invoke || (async () => {});

class PerformanceMonitoringDashboard {
    constructor() {
        this.isVisible = false;
        this.isMonitoring = false;
        this.monitoringInterval = null;
        this.metricsHistory = [];
        this.resourceHistory = [];
        this.frameTimeHistory = [];
        this.inputLagHistory = [];
        this.aiOperationHistory = [];
        
        // Performance thresholds
        this.thresholds = {
            memoryWarning: 75, // MB
            memoryCritical: 95, // MB
            cpuWarning: 60,    // %
            cpuCritical: 80,   // %
            frameTimeWarning: 16,  // ms (60fps)
            frameTimeCritical: 33, // ms (30fps)
            inputLagWarning: 50,   // ms
            inputLagCritical: 100, // ms
            aiOperationWarning: 500,   // ms
            aiOperationCritical: 1000, // ms
        };
        
        // Update intervals
        this.updateInterval = 100; // 100ms for real-time feel
        this.resourceUpdateInterval = 1000; // 1s for resource tracking
        
        this.initializeDashboard();
    }

    /**
     * Initialize the performance monitoring dashboard
     */
    async initializeDashboard() {
        this.createDashboardElements();
        this.attachEventListeners();
        this.startUIResponsivenessTracking();
        
        // Check if performance monitoring is already active
        try {
            const status = await invoke('get_monitoring_status');
            if (status.is_active) {
                this.isMonitoring = true;
                this.startMetricsCollection();
            }
        } catch (error) {
            console.warn('Performance monitoring not available:', error);
        }
    }

    /**
     * Create dashboard DOM elements
     */
    createDashboardElements() {
        // No longer create floating toggle button - using main UI button instead

        // Dashboard container
        this.dashboardElement = document.createElement('div');
        this.dashboardElement.className = 'performance-monitoring-dashboard';
        this.dashboardElement.innerHTML = this.getDashboardHTML();
        document.body.appendChild(this.dashboardElement);

        // Get references to key elements
        this.elements = {
            content: this.dashboardElement.querySelector('.performance-dashboard-content'),
            memoryValue: this.dashboardElement.querySelector('[data-metric="memory"] .metric-value'),
            memoryTrend: this.dashboardElement.querySelector('[data-metric="memory"] .metric-trend'),
            cpuValue: this.dashboardElement.querySelector('[data-metric="cpu"] .metric-value'),
            cpuTrend: this.dashboardElement.querySelector('[data-metric="cpu"] .metric-trend'),
            frameTimeValue: this.dashboardElement.querySelector('[data-metric="frametime"] .metric-value'),
            inputLagValue: this.dashboardElement.querySelector('[data-metric="inputlag"] .metric-value'),
            resourceChart: this.dashboardElement.querySelector('.resource-chart-canvas'),
            aiOperationsList: this.dashboardElement.querySelector('.ai-operations-list'),
            performanceAlerts: this.dashboardElement.querySelector('.performance-alerts'),
            startStopButton: this.dashboardElement.querySelector('[data-action="start-stop"]'),
            exportButton: this.dashboardElement.querySelector('[data-action="export"]'),
            frameTimeDot: this.dashboardElement.querySelector('.frame-time-dot'),
            inputLagFill: this.dashboardElement.querySelector('.input-lag-fill'),
        };

        // Setup resource chart canvas
        if (this.elements.resourceChart) {
            const ctx = this.elements.resourceChart.getContext('2d');
            this.setupResourceChart(ctx);
        }
    }

    /**
     * Get dashboard HTML structure
     */
    getDashboardHTML() {
        return `
            <div class="performance-dashboard-header">
                <div class="performance-dashboard-title">Performance Monitor</div>
                <button class="performance-dashboard-toggle" data-action="close">×</button>
            </div>
            <div class="performance-dashboard-content">
                <!-- Real-time Metrics -->
                <div class="performance-section">
                    <div class="performance-section-header">Real-time Metrics</div>
                    <div class="performance-section-content">
                        <div class="realtime-metrics-grid">
                            <div class="metric-item" data-metric="memory">
                                <div class="metric-value">--</div>
                                <div class="metric-label">Memory (MB)</div>
                                <div class="metric-trend">--</div>
                            </div>
                            <div class="metric-item" data-metric="cpu">
                                <div class="metric-value">--</div>
                                <div class="metric-label">CPU (%)</div>
                                <div class="metric-trend">--</div>
                            </div>
                            <div class="metric-item" data-metric="frametime">
                                <div class="metric-value">--</div>
                                <div class="metric-label">Frame (ms)</div>
                                <div class="metric-trend">--</div>
                            </div>
                            <div class="metric-item" data-metric="inputlag">
                                <div class="metric-value">--</div>
                                <div class="metric-label">Input (ms)</div>
                                <div class="metric-trend">--</div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Resource Utilization Chart -->
                <div class="performance-section">
                    <div class="performance-section-header">Resource Utilization</div>
                    <div class="performance-section-content">
                        <div class="resource-chart">
                            <canvas class="resource-chart-canvas" width="376" height="60"></canvas>
                        </div>
                        <div class="chart-legend">
                            <div class="legend-item">
                                <div class="legend-color legend-cpu"></div>
                                <span>CPU</span>
                            </div>
                            <div class="legend-item">
                                <div class="legend-color legend-memory"></div>
                                <span>Memory</span>
                            </div>
                            <div class="legend-item">
                                <div class="legend-color legend-disk"></div>
                                <span>I/O</span>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- AI Operations -->
                <div class="performance-section">
                    <div class="performance-section-header">AI Operations</div>
                    <div class="performance-section-content">
                        <div class="ai-operations-list">
                            <div class="loading-indicator">
                                <div class="loading-dots"></div>
                                <span style="margin-left: 8px;">Monitoring...</span>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- UI Responsiveness -->
                <div class="performance-section">
                    <div class="performance-section-header">UI Responsiveness</div>
                    <div class="performance-section-content">
                        <div class="ui-responsiveness">
                            <div class="frame-time-indicator">
                                <div class="frame-time-dot"></div>
                                <span>60fps</span>
                            </div>
                            <div style="flex: 1; margin: 0 12px;">
                                <div style="font-size: 8px; color: #999; margin-bottom: 2px;">Input Lag</div>
                                <div class="input-lag-meter">
                                    <div class="input-lag-fill" style="width: 0%"></div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Performance Alerts -->
                <div class="performance-section">
                    <div class="performance-section-header">Performance Alerts</div>
                    <div class="performance-section-content">
                        <div class="performance-alerts">
                            <div style="text-align: center; color: #999; font-size: 9px; padding: 10px;">
                                No alerts
                            </div>
                        </div>
                    </div>
                </div>

                <!-- Controls -->
                <div class="performance-section">
                    <div class="performance-section-header">Controls</div>
                    <div class="performance-section-content">
                        <div class="performance-reports">
                            <button class="report-button" data-action="start-stop">Start Monitoring</button>
                            <button class="report-button" data-action="export">Export Report</button>
                            <button class="report-button" data-action="clear-history">Clear History</button>
                        </div>
                    </div>
                </div>
            </div>
        `;
    }

    /**
     * Attach event listeners
     */
    attachEventListeners() {
        // No toggle button event listener needed - handled by main UI

        // Dashboard close button
        this.dashboardElement.querySelector('[data-action="close"]').addEventListener('click', () => {
            this.hide();
        });

        // Control buttons
        this.dashboardElement.querySelector('[data-action="start-stop"]').addEventListener('click', () => {
            this.toggleMonitoring();
        });

        this.dashboardElement.querySelector('[data-action="export"]').addEventListener('click', () => {
            this.exportReport();
        });

        this.dashboardElement.querySelector('[data-action="clear-history"]').addEventListener('click', () => {
            this.clearHistory();
        });

        // Keyboard shortcuts are now handled in main.js

        // Handle window resize
        window.addEventListener('resize', () => {
            if (this.elements.resourceChart) {
                this.resizeResourceChart();
            }
        });
    }

    /**
     * Setup resource utilization chart
     */
    setupResourceChart(ctx) {
        this.chartCtx = ctx;
        this.chartWidth = ctx.canvas.width;
        this.chartHeight = ctx.canvas.height;
        
        // Initialize chart with empty data
        this.drawResourceChart();
    }

    /**
     * Resize resource chart canvas
     */
    resizeResourceChart() {
        const canvas = this.elements.resourceChart;
        const container = canvas.parentElement;
        const rect = container.getBoundingClientRect();
        
        canvas.width = rect.width - 2; // Account for borders
        this.chartWidth = canvas.width;
        
        if (this.chartCtx) {
            this.drawResourceChart();
        }
    }

    /**
     * Draw resource utilization chart
     */
    drawResourceChart() {
        if (!this.chartCtx) return;

        const ctx = this.chartCtx;
        const width = this.chartWidth;
        const height = this.chartHeight;

        // Clear canvas
        ctx.clearRect(0, 0, width, height);

        // Draw background grid
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.1)';
        ctx.lineWidth = 0.5;
        
        // Horizontal grid lines
        for (let i = 0; i < 5; i++) {
            const y = (height / 4) * i;
            ctx.beginPath();
            ctx.moveTo(0, y);
            ctx.lineTo(width, y);
            ctx.stroke();
        }

        // Draw resource data if available
        if (this.resourceHistory.length > 1) {
            this.drawResourceLine(ctx, width, height, this.resourceHistory, 'cpu_usage_percent', '#00ff00', 100);
            this.drawResourceLine(ctx, width, height, this.resourceHistory, 'memory_usage_mb', '#0088ff', this.thresholds.memoryCritical);
        }
    }

    /**
     * Draw a resource utilization line on the chart
     */
    drawResourceLine(ctx, width, height, data, property, color, maxValue) {
        if (data.length < 2) return;

        ctx.strokeStyle = color;
        ctx.lineWidth = 1;
        ctx.beginPath();

        const pointSpacing = width / Math.max(data.length - 1, 1);
        
        data.forEach((point, index) => {
            const x = index * pointSpacing;
            const value = Math.min(point[property] || 0, maxValue);
            const y = height - (value / maxValue) * height;
            
            if (index === 0) {
                ctx.moveTo(x, y);
            } else {
                ctx.lineTo(x, y);
            }
        });

        ctx.stroke();
    }

    /**
     * Start UI responsiveness tracking
     */
    startUIResponsivenessTracking() {
        let lastFrameTime = performance.now();
        let frameCount = 0;
        let inputEventTime = null;

        // Frame time tracking
        const trackFrameTime = (currentTime) => {
            const frameTime = currentTime - lastFrameTime;
            lastFrameTime = currentTime;
            
            this.frameTimeHistory.push(frameTime);
            if (this.frameTimeHistory.length > 60) { // Keep 1 second of data at 60fps
                this.frameTimeHistory.shift();
            }

            // Update UI every few frames to avoid overhead
            if (frameCount % 6 === 0) { // Update ~10 times per second
                this.updateFrameTimeDisplay(frameTime);
            }
            frameCount++;
            
            requestAnimationFrame(trackFrameTime);
        };

        requestAnimationFrame(trackFrameTime);

        // Input lag tracking
        const trackInputEvents = ['click', 'keydown', 'mousemove'];
        trackInputEvents.forEach(eventType => {
            document.addEventListener(eventType, (e) => {
                inputEventTime = performance.now();
            }, { passive: true });
        });

        // Measure input lag during next frame
        const measureInputLag = () => {
            if (inputEventTime !== null) {
                const inputLag = performance.now() - inputEventTime;
                this.inputLagHistory.push(inputLag);
                if (this.inputLagHistory.length > 10) {
                    this.inputLagHistory.shift();
                }
                this.updateInputLagDisplay(inputLag);
                inputEventTime = null;
            }
            requestAnimationFrame(measureInputLag);
        };

        requestAnimationFrame(measureInputLag);
    }

    /**
     * Update frame time display
     */
    updateFrameTimeDisplay(frameTime) {
        if (!this.elements.frameTimeValue || !this.elements.frameTimeDot) return;

        const avgFrameTime = this.frameTimeHistory.reduce((a, b) => a + b, 0) / this.frameTimeHistory.length;
        
        this.elements.frameTimeValue.textContent = `${avgFrameTime.toFixed(1)}ms`;
        
        // Update frame rate indicator
        const fps = Math.round(1000 / avgFrameTime);
        const fpsText = this.elements.frameTimeDot.nextSibling;
        if (fpsText) {
            fpsText.textContent = `${fps}fps`;
        }

        // Update indicator colors
        this.elements.frameTimeDot.className = 'frame-time-dot';
        this.elements.frameTimeValue.className = 'metric-value';
        
        if (avgFrameTime > this.thresholds.frameTimeCritical) {
            this.elements.frameTimeDot.classList.add('critical');
            this.elements.frameTimeValue.classList.add('critical');
        } else if (avgFrameTime > this.thresholds.frameTimeWarning) {
            this.elements.frameTimeDot.classList.add('warning');
            this.elements.frameTimeValue.classList.add('warning');
        }
    }

    /**
     * Update input lag display
     */
    updateInputLagDisplay(inputLag) {
        if (!this.elements.inputLagValue || !this.elements.inputLagFill) return;

        const avgInputLag = this.inputLagHistory.reduce((a, b) => a + b, 0) / this.inputLagHistory.length;
        
        this.elements.inputLagValue.textContent = `${avgInputLag.toFixed(0)}ms`;
        
        // Update lag meter
        const lagPercentage = Math.min((avgInputLag / this.thresholds.inputLagCritical) * 100, 100);
        this.elements.inputLagFill.style.width = `${lagPercentage}%`;
        
        // Update colors
        this.elements.inputLagFill.className = 'input-lag-fill';
        
        if (avgInputLag > this.thresholds.inputLagCritical) {
            this.elements.inputLagFill.classList.add('critical');
        } else if (avgInputLag > this.thresholds.inputLagWarning) {
            this.elements.inputLagFill.classList.add('warning');
        }
    }

    /**
     * Start performance metrics collection
     */
    async startMetricsCollection() {
        if (this.monitoringInterval) {
            clearInterval(this.monitoringInterval);
        }

        this.monitoringInterval = setInterval(async () => {
            try {
                await this.collectMetrics();
            } catch (error) {
                console.error('Error collecting performance metrics:', error);
            }
        }, this.updateInterval);
    }

    /**
     * Stop performance metrics collection
     */
    stopMetricsCollection() {
        if (this.monitoringInterval) {
            clearInterval(this.monitoringInterval);
            this.monitoringInterval = null;
        }
    }

    /**
     * Collect performance metrics from backend
     */
    async collectMetrics() {
        try {
            // Get current performance metrics
            const currentMetrics = await invoke('get_current_performance_metrics');
            this.updateCurrentMetrics(currentMetrics);

            // Get resource utilization
            const resourceMetrics = await invoke('get_resource_utilization');
            this.updateResourceMetrics(resourceMetrics);

            // Get active alerts
            const alerts = await invoke('get_active_alerts');
            this.updatePerformanceAlerts(alerts);

            // Update AI operations if enhanced metrics is available
            try {
                const searchMetrics = await invoke('get_search_operation_metrics', { limit: 10 });
                this.updateAIOperations(searchMetrics);
            } catch (e) {
                // Enhanced metrics not available
            }

        } catch (error) {
            console.warn('Failed to collect some metrics:', error);
        }
    }

    /**
     * Update current metrics display
     */
    updateCurrentMetrics(metrics) {
        // For now, show aggregated metrics
        let totalMemory = 0;
        let totalCPU = 0;
        let operationCount = 0;

        for (const [operationType, operationMetrics] of Object.entries(metrics)) {
            totalMemory += operationMetrics.memory_peak_mb || 0;
            totalCPU += operationMetrics.cpu_usage_percent || 0;
            operationCount++;
        }

        const avgMemory = operationCount > 0 ? totalMemory / operationCount : 0;
        const avgCPU = operationCount > 0 ? totalCPU / operationCount : 0;

        this.updateMetricDisplay('memory', avgMemory);
        this.updateMetricDisplay('cpu', avgCPU);
    }

    /**
     * Update resource metrics display
     */
    updateResourceMetrics(resourceMetrics) {
        // Add to resource history for chart
        this.resourceHistory.push(resourceMetrics);
        if (this.resourceHistory.length > 100) { // Keep 100 data points
            this.resourceHistory.shift();
        }

        // Update memory and CPU from resource metrics
        this.updateMetricDisplay('memory', resourceMetrics.memory_usage_mb);
        this.updateMetricDisplay('cpu', resourceMetrics.cpu_usage_percent);

        // Redraw chart
        this.drawResourceChart();
    }

    /**
     * Update a specific metric display
     */
    updateMetricDisplay(metricName, value) {
        const metricElement = this.dashboardElement.querySelector(`[data-metric="${metricName}"]`);
        if (!metricElement) return;

        const valueElement = metricElement.querySelector('.metric-value');
        const trendElement = metricElement.querySelector('.metric-trend');

        if (valueElement) {
            const formattedValue = metricName === 'memory' 
                ? `${value.toFixed(1)}`
                : `${value.toFixed(0)}`;
            
            valueElement.textContent = formattedValue;
            
            // Update status based on thresholds
            let threshold = metricName === 'memory' ? this.thresholds.memoryWarning : this.thresholds.cpuWarning;
            let criticalThreshold = metricName === 'memory' ? this.thresholds.memoryCritical : this.thresholds.cpuCritical;
            
            valueElement.className = 'metric-value';
            metricElement.className = `metric-item`;
            
            if (value > criticalThreshold) {
                valueElement.classList.add('critical');
                metricElement.classList.add('critical');
            } else if (value > threshold) {
                valueElement.classList.add('warning');
                metricElement.classList.add('warning');
            }
        }

        // Calculate trend (simplified)
        if (trendElement && this.resourceHistory.length > 1) {
            const property = metricName === 'memory' ? 'memory_usage_mb' : 'cpu_usage_percent';
            const current = value;
            const previous = this.resourceHistory[this.resourceHistory.length - 2][property] || 0;
            const change = ((current - previous) / previous * 100);

            let trendText = '→';
            let trendClass = 'trend-stable';

            if (change > 5) {
                trendText = '↑';
                trendClass = 'trend-up';
            } else if (change < -5) {
                trendText = '↓';
                trendClass = 'trend-down';
            }

            trendElement.textContent = trendText;
            trendElement.className = `metric-trend ${trendClass}`;
        }
    }

    /**
     * Update AI operations display
     */
    updateAIOperations(searchMetrics) {
        if (!this.elements.aiOperationsList || !Array.isArray(searchMetrics)) return;

        // Clear loading indicator
        this.elements.aiOperationsList.innerHTML = '';

        if (searchMetrics.length === 0) {
            this.elements.aiOperationsList.innerHTML = `
                <div style="text-align: center; color: #999; font-size: 9px; padding: 10px;">
                    No AI operations
                </div>
            `;
            return;
        }

        // Display recent operations
        searchMetrics.slice(0, 10).forEach(operation => {
            const duration = operation.duration_ms || 0;
            let timeClass = '';
            
            if (duration > this.thresholds.aiOperationCritical) {
                timeClass = 'very-slow';
            } else if (duration > this.thresholds.aiOperationWarning) {
                timeClass = 'slow';
            }

            const operationElement = document.createElement('div');
            operationElement.className = 'ai-operation-item';
            operationElement.innerHTML = `
                <div class="ai-operation-name">${operation.operation_type || 'Search'}</div>
                <div class="ai-operation-time ${timeClass}">${duration.toFixed(0)}ms</div>
            `;
            
            this.elements.aiOperationsList.appendChild(operationElement);
        });
    }

    /**
     * Update performance alerts display
     */
    updatePerformanceAlerts(alerts) {
        if (!this.elements.performanceAlerts || !Array.isArray(alerts)) return;

        if (alerts.length === 0) {
            this.elements.performanceAlerts.innerHTML = `
                <div style="text-align: center; color: #999; font-size: 9px; padding: 10px;">
                    No alerts
                </div>
            `;
            return;
        }

        this.elements.performanceAlerts.innerHTML = '';

        alerts.slice(0, 5).forEach(alert => {
            const alertElement = document.createElement('div');
            alertElement.className = `performance-alert ${alert.severity.toLowerCase()}`;
            alertElement.innerHTML = `
                <div class="alert-message">${alert.message}</div>
                <div class="alert-timestamp">${new Date(alert.triggered_at).toLocaleTimeString()}</div>
            `;
            
            this.elements.performanceAlerts.appendChild(alertElement);
        });
    }

    /**
     * Toggle monitoring on/off
     */
    async toggleMonitoring() {
        try {
            if (this.isMonitoring) {
                // Stop monitoring
                await invoke('stop_performance_monitoring');
                this.stopMetricsCollection();
                this.isMonitoring = false;
                this.elements.startStopButton.textContent = 'Start Monitoring';
            } else {
                // Start monitoring
                const config = {
                    enable_monitoring: true,
                    collection_interval_ms: this.updateInterval,
                    enable_resource_tracking: true,
                    resource_tracking_interval_ms: this.resourceUpdateInterval,
                    enable_alerts: true,
                    max_overhead_percent: 1.0, // <1% overhead requirement
                };
                
                await invoke('start_performance_monitoring', { 
                    request: { config } 
                });
                
                this.startMetricsCollection();
                this.isMonitoring = true;
                this.elements.startStopButton.textContent = 'Stop Monitoring';
            }
        } catch (error) {
            console.error('Failed to toggle monitoring:', error);
            alert('Failed to toggle performance monitoring: ' + error);
        }
    }

    /**
     * Export performance report
     */
    async exportReport() {
        try {
            const report = await invoke('generate_performance_report', {
                request: {
                    period_hours: 1,
                    include_detailed_breakdown: true,
                    include_resource_analysis: true
                }
            });

            // Create and download report file
            const reportData = JSON.stringify(report, null, 2);
            const blob = new Blob([reportData], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            
            const a = document.createElement('a');
            a.href = url;
            a.download = `performance-report-${new Date().toISOString().slice(0, 19).replace(/:/g, '-')}.json`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
            
        } catch (error) {
            console.error('Failed to export report:', error);
            alert('Failed to export performance report: ' + error);
        }
    }

    /**
     * Clear metrics history
     */
    clearHistory() {
        this.metricsHistory = [];
        this.resourceHistory = [];
        this.frameTimeHistory = [];
        this.inputLagHistory = [];
        this.aiOperationHistory = [];
        
        // Clear charts
        if (this.chartCtx) {
            this.drawResourceChart();
        }
        
        // Clear AI operations list
        if (this.elements.aiOperationsList) {
            this.elements.aiOperationsList.innerHTML = `
                <div class="loading-indicator">
                    <div class="loading-dots"></div>
                    <span style="margin-left: 8px;">Monitoring...</span>
                </div>
            `;
        }
    }

    /**
     * Show dashboard
     */
    show() {
        this.isVisible = true;
        this.dashboardElement.classList.add('visible');
    }

    /**
     * Hide dashboard
     */
    hide() {
        this.isVisible = false;
        this.dashboardElement.classList.remove('visible');
    }

    /**
     * Toggle dashboard visibility
     */
    toggle() {
        if (this.isVisible) {
            this.hide();
        } else {
            this.show();
        }
    }

    /**
     * Destroy dashboard and cleanup resources
     */
    destroy() {
        this.stopMetricsCollection();
        
        if (this.dashboardElement) {
            document.body.removeChild(this.dashboardElement);
        }
    }
}

export { PerformanceMonitoringDashboard };