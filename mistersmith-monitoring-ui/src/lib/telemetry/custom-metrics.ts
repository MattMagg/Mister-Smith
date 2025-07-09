import { metrics } from '@opentelemetry/api';

// Get meter for custom metrics
const meter = metrics.getMeter('mistersmith-monitoring', '1.0.0');

// Discovery metrics
export const discoveryCounter = meter.createCounter('mistersmith.discoveries.total', {
  description: 'Total number of discoveries received',
  unit: 'discoveries'
});

export const discoveryConfidenceHistogram = meter.createHistogram('mistersmith.discovery.confidence', {
  description: 'Distribution of discovery confidence scores',
  unit: 'ratio'
});

// Agent metrics
export const activeAgentsGauge = meter.createUpDownCounter('mistersmith.agents.active', {
  description: 'Number of active agents',
  unit: 'agents'
});

// SSE Connection metrics
export const sseConnectionGauge = meter.createUpDownCounter('mistersmith.sse.connections', {
  description: 'Number of active SSE connections',
  unit: 'connections'
});

export const sseReconnectCounter = meter.createCounter('mistersmith.sse.reconnects', {
  description: 'Number of SSE reconnection attempts',
  unit: 'attempts'
});

export const sseMessageCounter = meter.createCounter('mistersmith.sse.messages', {
  description: 'Number of SSE messages received',
  unit: 'messages'
});

// Performance metrics
export const renderTimeHistogram = meter.createHistogram('mistersmith.ui.render_time', {
  description: 'Time taken to render UI components',
  unit: 'ms'
});

export const apiLatencyHistogram = meter.createHistogram('mistersmith.api.latency', {
  description: 'API call latency',
  unit: 'ms'
});

// Error tracking
export const errorCounter = meter.createCounter('mistersmith.errors.total', {
  description: 'Total number of errors',
  unit: 'errors'
});

// System metrics
export const systemHealthGauge = meter.createUpDownCounter('mistersmith.system.health', {
  description: 'System health status (1=healthy, 0=degraded, -1=unhealthy)',
  unit: 'status'
});

export const cpuUsageGauge = meter.createObservableGauge('mistersmith.system.cpu_usage', {
  description: 'CPU usage percentage',
  unit: '%'
});

export const memoryUsageGauge = meter.createObservableGauge('mistersmith.system.memory_usage', {
  description: 'Memory usage percentage',
  unit: '%'
});

// Helper functions to record metrics
export function recordDiscovery(discovery: any) {
  discoveryCounter.add(1, {
    type: discovery.type,
    agentId: discovery.agentId
  });
  
  discoveryConfidenceHistogram.record(discovery.confidence, {
    type: discovery.type
  });
}

export function recordSSEEvent(eventType: string) {
  sseMessageCounter.add(1, {
    event_type: eventType
  });
}

export function recordAPICall(endpoint: string, duration: number, success: boolean) {
  apiLatencyHistogram.record(duration, {
    endpoint,
    success: success.toString(),
    method: 'GET' // You can make this dynamic based on actual method
  });
}

export function recordError(error: Error, source: string, severity: 'low' | 'medium' | 'high' | 'critical' = 'medium') {
  errorCounter.add(1, {
    error_type: error.name,
    error_message: error.message,
    source,
    severity
  });
}

export function recordRenderTime(componentName: string, duration: number) {
  renderTimeHistogram.record(duration, {
    component: componentName,
    path: window.location.pathname
  });
}

export function updateActiveAgents(count: number) {
  // This is an up-down counter, so we need to track the delta
  // In a real app, you'd track the previous value
  activeAgentsGauge.add(count);
}

export function updateConnectionStatus(connected: boolean) {
  sseConnectionGauge.add(connected ? 1 : -1);
}

export function updateSystemHealth(status: 'healthy' | 'degraded' | 'unhealthy') {
  const value = status === 'healthy' ? 1 : status === 'degraded' ? 0 : -1;
  systemHealthGauge.add(value);
}

// Observable callbacks for system metrics
export function registerSystemMetricCallbacks(getSystemMetrics: () => { cpu: number; memory: number }) {
  // CPU usage observable
  cpuUsageGauge.addCallback((result) => {
    const metrics = getSystemMetrics();
    result.observe(metrics.cpu);
  });

  // Memory usage observable
  memoryUsageGauge.addCallback((result) => {
    const metrics = getSystemMetrics();
    result.observe(metrics.memory);
  });
}