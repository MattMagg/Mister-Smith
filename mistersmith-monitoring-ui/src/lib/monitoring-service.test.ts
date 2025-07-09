import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { MonitoringService, SystemHealth, SystemMetrics, Discovery, Agent, ErrorEvent, LogEntry, TraceEvent } from './monitoring-service';
import { SSEClient } from './sse-client';
import { JaegerClient } from './telemetry/jaeger-client';

// Mock SSEClient
vi.mock('./sse-client', () => ({
  SSEClient: vi.fn().mockImplementation(() => ({
    connect: vi.fn(),
    disconnect: vi.fn(),
    subscribe: vi.fn(),
    isConnected: vi.fn().mockReturnValue(false),
    getConnectionStatus: vi.fn().mockReturnValue({ subscribe: vi.fn() }),
    onEvent: vi.fn().mockReturnValue({ subscribe: vi.fn() }),
  })),
}));

// Mock JaegerClient
vi.mock('./telemetry/jaeger-client', () => ({
  JaegerClient: vi.fn().mockImplementation(() => ({
    getServices: vi.fn(),
    searchTraces: vi.fn(),
    getCallRates: vi.fn(),
    getErrorRates: vi.fn(),
    getLatencies: vi.fn(),
    getTrace: vi.fn(),
  })),
}));

// Mock telemetry functions
vi.mock('./telemetry/custom-metrics', () => ({
  recordDiscovery: vi.fn(),
  recordSSEEvent: vi.fn(),
  recordError: vi.fn(),
  updateActiveAgents: vi.fn(),
  updateSystemHealth: vi.fn(),
}));

// Mock fetch
global.fetch = vi.fn();

describe('MonitoringService', () => {
  let service: MonitoringService;
  let mockSSEClient: any;

  beforeEach(() => {
    vi.clearAllMocks();
    service = MonitoringService.getInstance();
    mockSSEClient = new SSEClient('test-url');
    (service as any).sseClient = mockSSEClient;
  });

  afterEach(() => {
    service.disconnect();
  });

  describe('getInstance', () => {
    it('returns the same instance (singleton)', () => {
      const instance1 = MonitoringService.getInstance();
      const instance2 = MonitoringService.getInstance();
      expect(instance1).toBe(instance2);
    });
  });

  describe('connect', () => {
    it('establishes SSE connection and subscribes to events', async () => {
      const mockConnect = vi.fn().mockResolvedValue(undefined);
      const mockSubscribe = vi.fn();
      mockSSEClient.connect = mockConnect;
      mockSSEClient.subscribe = mockSubscribe;

      await service.connect();

      expect(mockConnect).toHaveBeenCalled();
      expect(mockSubscribe).toHaveBeenCalledWith('discovery', expect.any(Function));
      expect(mockSubscribe).toHaveBeenCalledWith('agent', expect.any(Function));
      expect(mockSubscribe).toHaveBeenCalledWith('log', expect.any(Function));
      expect(mockSubscribe).toHaveBeenCalledWith('trace', expect.any(Function));
      expect(mockSubscribe).toHaveBeenCalledWith('error', expect.any(Function));
    });

    it('starts health check interval', async () => {
      vi.useFakeTimers();
      const mockHealthCheck = vi.spyOn(service, 'fetchSystemHealth').mockResolvedValue(undefined);

      await service.connect();

      vi.advanceTimersByTime(5000);
      expect(mockHealthCheck).toHaveBeenCalled();

      vi.useRealTimers();
    });
  });

  describe('disconnect', () => {
    it('disconnects SSE and clears intervals', () => {
      const mockDisconnect = vi.fn();
      mockSSEClient.disconnect = mockDisconnect;

      service.disconnect();

      expect(mockDisconnect).toHaveBeenCalled();
    });
  });

  describe('fetchSystemHealth', () => {
    it('fetches system health successfully', async () => {
      const mockHealth: SystemHealth = {
        status: 'healthy',
        uptime: 3600,
        version: '1.0.0',
        timestamp: new Date(),
        components: {
          nats_server: 'up',
          database: 'up',
          message_queue: 'up'
        }
      };

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockHealth)
      });

      await service.fetchSystemHealth();

      expect(fetch).toHaveBeenCalledWith('http://localhost:8080/system/health');
    });

    it('handles fetch errors gracefully', async () => {
      (global.fetch as any).mockRejectedValue(new Error('Network error'));

      await service.fetchSystemHealth();

      expect(fetch).toHaveBeenCalledWith('http://localhost:8080/system/health');
    });
  });

  describe('fetchSystemMetrics', () => {
    it('fetches system metrics successfully', async () => {
      const mockMetrics: SystemMetrics = {
        timestamp: new Date(),
        cpu_usage: 45.5,
        memory_usage: 67.2,
        active_agents: 3,
        total_discoveries: 42,
        nats_connections: 5,
        message_throughput: 12.5
      };

      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: () => Promise.resolve(mockMetrics)
      });

      await service.fetchSystemMetrics();

      expect(fetch).toHaveBeenCalledWith('http://localhost:8080/system/metrics');
    });
  });

  describe('event handling', () => {
    it('handles discovery events', async () => {
      const mockDiscovery: Discovery = {
        id: 'discovery-1',
        agent_id: 'agent-1',
        agent_role: 'analyzer',
        discovery_type: 'pattern',
        content: 'Pattern detected',
        confidence: 0.85,
        timestamp: new Date(),
        related_to: null
      };

      await service.connect();
      
      // Simulate discovery event
      const discoveryHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'discovery')?.[1];
      
      if (discoveryHandler) {
        discoveryHandler(mockDiscovery);
      }

      const discoveries = service.getDiscoveries();
      expect(discoveries).toHaveLength(1);
      expect(discoveries[0]).toEqual(mockDiscovery);
    });

    it('handles agent events', async () => {
      const mockAgent: Agent = {
        id: 'agent-1',
        type: 'analyzer',
        status: 'running',
        createdAt: new Date(),
        lastActivity: new Date(),
        capabilities: ['pattern-detection']
      };

      await service.connect();
      
      // Simulate agent event
      const agentHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'agent')?.[1];
      
      if (agentHandler) {
        agentHandler(mockAgent);
      }

      const agents = service.getAgents();
      expect(agents).toHaveLength(1);
      expect(agents[0]).toEqual(mockAgent);
    });

    it('handles error events', async () => {
      const mockError: ErrorEvent = {
        id: 'error-1',
        message: 'Test error',
        source: 'test-component',
        timestamp: new Date(),
        severity: 'high',
        stack: 'Error stack trace'
      };

      await service.connect();
      
      // Simulate error event
      const errorHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'error')?.[1];
      
      if (errorHandler) {
        errorHandler(mockError);
      }

      const errors = service.getErrors();
      expect(errors).toHaveLength(1);
      expect(errors[0]).toEqual(mockError);
    });

    it('handles log events', async () => {
      const mockLog: LogEntry = {
        id: 'log-1',
        message: 'Test log message',
        level: 'info',
        source: 'test-component',
        timestamp: new Date(),
        metadata: { key: 'value' }
      };

      await service.connect();
      
      // Simulate log event
      const logHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'log')?.[1];
      
      if (logHandler) {
        logHandler(mockLog);
      }

      const logs = service.getLogs();
      expect(logs).toHaveLength(1);
      expect(logs[0]).toEqual(mockLog);
    });

    it('handles trace events', async () => {
      const mockTrace: TraceEvent = {
        id: 'trace-1',
        message: 'Test trace',
        source: 'test-component',
        timestamp: new Date(),
        trace_id: 'trace-123',
        span_id: 'span-456',
        metadata: { operation: 'test' }
      };

      await service.connect();
      
      // Simulate trace event
      const traceHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'trace')?.[1];
      
      if (traceHandler) {
        traceHandler(mockTrace);
      }

      const traces = service.getTraces();
      expect(traces).toHaveLength(1);
      expect(traces[0]).toEqual(mockTrace);
    });
  });

  describe('observables', () => {
    it('emits connection status changes', (done) => {
      const statusSubscription = service.connectionStatus$.subscribe(status => {
        if (status === 'connected') {
          expect(status).toBe('connected');
          statusSubscription.unsubscribe();
          done();
        }
      });

      // Simulate connection
      (service as any).connectionStatus$.next('connected');
    });

    it('emits system health updates', (done) => {
      const mockHealth: SystemHealth = {
        status: 'healthy',
        uptime: 3600,
        version: '1.0.0',
        timestamp: new Date(),
        components: {
          nats_server: 'up',
          database: 'up',
          message_queue: 'up'
        }
      };

      const healthSubscription = service.systemHealth$.subscribe(health => {
        if (health) {
          expect(health).toEqual(mockHealth);
          healthSubscription.unsubscribe();
          done();
        }
      });

      // Simulate health update
      (service as any).systemHealth$.next(mockHealth);
    });
  });

  describe('data management', () => {
    it('maintains discovery history with limit', async () => {
      await service.connect();
      
      const discoveryHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'discovery')?.[1];
      
      if (discoveryHandler) {
        // Add more than 100 discoveries
        for (let i = 0; i < 150; i++) {
          discoveryHandler({
            id: `discovery-${i}`,
            agent_id: 'agent-1',
            agent_role: 'analyzer',
            discovery_type: 'pattern',
            content: `Pattern ${i}`,
            confidence: 0.85,
            timestamp: new Date(),
            related_to: null
          });
        }
      }

      const discoveries = service.getDiscoveries();
      expect(discoveries).toHaveLength(100); // Should be limited to 100
    });

    it('updates existing agents instead of duplicating', async () => {
      const mockAgent: Agent = {
        id: 'agent-1',
        type: 'analyzer',
        status: 'running',
        createdAt: new Date(),
        lastActivity: new Date(),
        capabilities: ['pattern-detection']
      };

      await service.connect();
      
      const agentHandler = mockSSEClient.subscribe.mock.calls
        .find(call => call[0] === 'agent')?.[1];
      
      if (agentHandler) {
        // Add agent twice
        agentHandler(mockAgent);
        agentHandler({ ...mockAgent, status: 'terminated' });
      }

      const agents = service.getAgents();
      expect(agents).toHaveLength(1);
      expect(agents[0].status).toBe('terminated');
    });
  });

  describe('Jaeger integration', () => {
    let mockJaegerClient: any;

    beforeEach(() => {
      mockJaegerClient = (service as any).jaegerClient;
    });

    it('should get Jaeger services', async () => {
      const mockServices = ['claude-code', 'mistersmith-agent'];
      mockJaegerClient.getServices.mockResolvedValue(mockServices);

      const services = await service.getJaegerServices();

      expect(mockJaegerClient.getServices).toHaveBeenCalled();
      expect(services).toEqual(mockServices);
    });

    it('should search traces by service', async () => {
      const mockTraces = [
        { traceID: 'trace1', spans: [] },
        { traceID: 'trace2', spans: [] }
      ];
      mockJaegerClient.searchTraces.mockResolvedValue(mockTraces);

      const traces = await service.getTracesByService('claude-code', '2h');

      expect(mockJaegerClient.searchTraces).toHaveBeenCalledWith({
        service: 'claude-code',
        lookback: '2h',
        limit: 100
      });
      expect(traces).toEqual(mockTraces);
    });

    it('should get service metrics', async () => {
      const mockCallRate = { metrics: [{ name: 'calls', values: [] }] };
      const mockErrorRate = { metrics: [{ name: 'errors', values: [] }] };
      const mockLatency = { metrics: [{ name: 'latency', values: [] }] };

      mockJaegerClient.getCallRates.mockResolvedValue(mockCallRate);
      mockJaegerClient.getErrorRates.mockResolvedValue(mockErrorRate);
      mockJaegerClient.getLatencies.mockResolvedValue(mockLatency);

      const metrics = await service.getServiceMetrics('claude-code');

      expect(mockJaegerClient.getCallRates).toHaveBeenCalledWith({
        service: 'claude-code',
        lookback: 3600000,
        step: 60000,
        ratePer: 60000,
      });
      expect(mockJaegerClient.getErrorRates).toHaveBeenCalledWith({
        service: 'claude-code',
        lookback: 3600000,
        step: 60000,
        ratePer: 60000,
      });
      expect(mockJaegerClient.getLatencies).toHaveBeenCalledWith({
        service: 'claude-code',
        quantile: 0.95,
        lookback: 3600000,
        step: 60000,
        ratePer: 60000,
      });

      expect(metrics).toEqual({
        callRate: mockCallRate,
        errorRate: mockErrorRate,
        latency: mockLatency
      });
    });

    it('should get trace details', async () => {
      const mockTrace = {
        traceID: 'trace123',
        spans: [{ spanID: 'span1', operationName: 'test' }]
      };
      mockJaegerClient.getTrace.mockResolvedValue(mockTrace);

      const trace = await service.getTraceDetails('trace123');

      expect(mockJaegerClient.getTrace).toHaveBeenCalledWith('trace123');
      expect(trace).toEqual(mockTrace);
    });
  });

  describe('telemetry recording', () => {
    it('should record telemetry when discovery is received', async () => {
      const { recordDiscovery, recordSSEEvent } = await import('./telemetry/custom-metrics');
      
      await service.connect();
      
      const mockDiscovery = {
        id: 'disc-1',
        type: 'pattern',
        content: 'Test discovery',
        confidence: 0.9,
        agentId: 'agent-1',
        timestamp: new Date().toISOString()
      };

      // Get the discovery event handler
      const eventHandlers = mockSSEClient.onEvent.mock.calls;
      const discoveryHandler = eventHandlers.find(call => call[0] === 'discovery')?.[1];
      
      if (discoveryHandler) {
        // Simulate receiving a discovery
        const mockSubscription = { subscribe: vi.fn(cb => cb(mockDiscovery)) };
        mockSSEClient.onEvent.mockReturnValue(mockSubscription);
        
        // Re-initialize to trigger the subscription
        await service['initializeConnection']();
      }

      expect(recordDiscovery).toHaveBeenCalledWith(mockDiscovery);
      expect(recordSSEEvent).toHaveBeenCalledWith('discovery');
    });

    it('should record telemetry when error is received', async () => {
      const { recordError, recordSSEEvent } = await import('./telemetry/custom-metrics');
      
      await service.connect();
      
      const mockError = {
        id: 'err-1',
        message: 'Test error',
        source: 'test-source',
        severity: 'high' as const,
        timestamp: new Date()
      };

      // Get the error event handler
      const eventHandlers = mockSSEClient.onEvent.mock.calls;
      const errorHandler = eventHandlers.find(call => call[0] === 'alert')?.[1];
      
      if (errorHandler) {
        // Simulate receiving an error
        const mockSubscription = { subscribe: vi.fn(cb => cb(mockError)) };
        mockSSEClient.onEvent.mockReturnValue(mockSubscription);
        
        // Re-initialize to trigger the subscription
        await service['initializeConnection']();
      }

      expect(recordSSEEvent).toHaveBeenCalledWith('alert');
      expect(recordError).toHaveBeenCalledWith(
        expect.any(Error),
        'test-source',
        'high'
      );
    });
  });
});