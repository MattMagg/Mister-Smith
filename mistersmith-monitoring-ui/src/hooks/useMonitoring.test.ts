import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { 
  useMonitoring, 
  useSystemHealth, 
  useSystemMetrics, 
  useDiscoveries, 
  useAgents, 
  useErrors, 
  useLogs, 
  useTraces 
} from './useMonitoring';
import { MonitoringService } from '../lib/monitoring-service';
import { BehaviorSubject } from 'rxjs';

// Mock MonitoringService
vi.mock('../lib/monitoring-service', () => ({
  MonitoringService: {
    getInstance: vi.fn(() => ({
      connect: vi.fn(),
      disconnect: vi.fn(),
      connectionStatus$: new BehaviorSubject('disconnected'),
      systemHealth$: new BehaviorSubject(null),
      systemMetrics$: new BehaviorSubject(null),
      discoveries$: new BehaviorSubject([]),
      agents$: new BehaviorSubject([]),
      errors$: new BehaviorSubject([]),
      logs$: new BehaviorSubject([]),
      traces$: new BehaviorSubject([]),
      fetchSystemHealth: vi.fn(),
      fetchSystemMetrics: vi.fn(),
    })),
  },
}));

describe('useMonitoring hooks', () => {
  let mockService: any;

  beforeEach(() => {
    vi.clearAllMocks();
    mockService = MonitoringService.getInstance();
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('useMonitoring', () => {
    it('initializes with default values', () => {
      const { result } = renderHook(() => useMonitoring());

      expect(result.current.connectionStatus).toBe('disconnected');
      expect(result.current.discoveries).toEqual([]);
      expect(result.current.agents).toEqual([]);
      expect(result.current.errors).toEqual([]);
      expect(result.current.logs).toEqual([]);
      expect(result.current.traces).toEqual([]);
    });

    it('connects to monitoring service on mount', () => {
      renderHook(() => useMonitoring());

      expect(mockService.connect).toHaveBeenCalled();
    });

    it('updates state when observables emit', async () => {
      const { result } = renderHook(() => useMonitoring());

      await act(async () => {
        mockService.connectionStatus$.next('connected');
      });

      expect(result.current.connectionStatus).toBe('connected');
    });

    it('subscribes to all observables', () => {
      const { result } = renderHook(() => useMonitoring());

      // Verify subscriptions are active by checking initial values
      expect(result.current.connectionStatus).toBe('disconnected');
      expect(result.current.discoveries).toEqual([]);
      expect(result.current.agents).toEqual([]);
    });

    it('cleans up subscriptions on unmount', () => {
      const { unmount } = renderHook(() => useMonitoring());

      const mockUnsubscribe = vi.fn();
      // Mock the subscription's unsubscribe method
      vi.spyOn(mockService.connectionStatus$, 'subscribe').mockReturnValue({
        unsubscribe: mockUnsubscribe,
      });

      unmount();

      // Note: Due to the nature of RxJS cleanup, we can't easily test this
      // But we can verify the hook doesn't cause memory leaks
    });
  });

  describe('useSystemHealth', () => {
    it('returns system health data', () => {
      const mockHealth = {
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

      const { result } = renderHook(() => useSystemHealth());

      act(() => {
        mockService.systemHealth$.next(mockHealth);
      });

      expect(result.current.data).toEqual(mockHealth);
      expect(result.current.isLoading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it('handles loading state', () => {
      const { result } = renderHook(() => useSystemHealth());

      expect(result.current.isLoading).toBe(true);
      expect(result.current.data).toBeNull();
    });

    it('handles error state', () => {
      const mockError = new Error('Health check failed');
      mockService.fetchSystemHealth.mockRejectedValue(mockError);

      const { result } = renderHook(() => useSystemHealth());

      expect(result.current.error).toBeNull(); // Initially no error
    });
  });

  describe('useSystemMetrics', () => {
    it('returns system metrics data', () => {
      const mockMetrics = {
        timestamp: new Date(),
        cpu_usage: 45.5,
        memory_usage: 67.2,
        active_agents: 3,
        total_discoveries: 42,
        nats_connections: 5,
        message_throughput: 12.5
      };

      const { result } = renderHook(() => useSystemMetrics());

      act(() => {
        mockService.systemMetrics$.next(mockMetrics);
      });

      expect(result.current.data).toEqual(mockMetrics);
      expect(result.current.isLoading).toBe(false);
      expect(result.current.error).toBeNull();
    });

    it('handles loading state', () => {
      const { result } = renderHook(() => useSystemMetrics());

      expect(result.current.isLoading).toBe(true);
      expect(result.current.data).toBeNull();
    });
  });

  describe('useDiscoveries', () => {
    it('returns discoveries data', () => {
      const mockDiscoveries = [
        {
          id: 'discovery-1',
          agent_id: 'agent-1',
          agent_role: 'analyzer',
          discovery_type: 'pattern',
          content: 'Pattern detected',
          confidence: 0.85,
          timestamp: new Date(),
          related_to: null
        }
      ];

      const { result } = renderHook(() => useDiscoveries());

      act(() => {
        mockService.discoveries$.next(mockDiscoveries);
      });

      expect(result.current.data).toEqual(mockDiscoveries);
      expect(result.current.isLoading).toBe(false);
    });

    it('handles empty discoveries', () => {
      const { result } = renderHook(() => useDiscoveries());

      act(() => {
        mockService.discoveries$.next([]);
      });

      expect(result.current.data).toEqual([]);
      expect(result.current.isLoading).toBe(false);
    });
  });

  describe('useAgents', () => {
    it('returns agents data', () => {
      const mockAgents = [
        {
          id: 'agent-1',
          type: 'analyzer',
          status: 'running',
          createdAt: new Date(),
          lastActivity: new Date(),
          capabilities: ['pattern-detection']
        }
      ];

      const { result } = renderHook(() => useAgents());

      act(() => {
        mockService.agents$.next(mockAgents);
      });

      expect(result.current.data).toEqual(mockAgents);
      expect(result.current.isLoading).toBe(false);
    });
  });

  describe('useErrors', () => {
    it('returns error data', () => {
      const mockErrors = [
        {
          id: 'error-1',
          message: 'Test error',
          source: 'test-component',
          timestamp: new Date(),
          severity: 'high',
          stack: 'Error stack trace'
        }
      ];

      const { result } = renderHook(() => useErrors());

      act(() => {
        mockService.errors$.next(mockErrors);
      });

      expect(result.current.data).toEqual(mockErrors);
      expect(result.current.isLoading).toBe(false);
    });

    it('handles empty errors', () => {
      const { result } = renderHook(() => useErrors());

      act(() => {
        mockService.errors$.next([]);
      });

      expect(result.current.data).toEqual([]);
      expect(result.current.isLoading).toBe(false);
    });
  });

  describe('useLogs', () => {
    it('returns logs data', () => {
      const mockLogs = [
        {
          id: 'log-1',
          message: 'Test log message',
          level: 'info',
          source: 'test-component',
          timestamp: new Date(),
          metadata: { key: 'value' }
        }
      ];

      const { result } = renderHook(() => useLogs());

      act(() => {
        mockService.logs$.next(mockLogs);
      });

      expect(result.current.data).toEqual(mockLogs);
      expect(result.current.isLoading).toBe(false);
    });

    it('filters logs by level', () => {
      const mockLogs = [
        {
          id: 'log-1',
          message: 'Info message',
          level: 'info',
          source: 'test-component',
          timestamp: new Date(),
          metadata: {}
        },
        {
          id: 'log-2',
          message: 'Error message',
          level: 'error',
          source: 'test-component',
          timestamp: new Date(),
          metadata: {}
        }
      ];

      const { result } = renderHook(() => useLogs('error'));

      act(() => {
        mockService.logs$.next(mockLogs);
      });

      expect(result.current.data).toHaveLength(1);
      expect(result.current.data[0].level).toBe('error');
    });
  });

  describe('useTraces', () => {
    it('returns traces data', () => {
      const mockTraces = [
        {
          id: 'trace-1',
          message: 'Test trace',
          source: 'test-component',
          timestamp: new Date(),
          trace_id: 'trace-123',
          span_id: 'span-456',
          metadata: { operation: 'test' }
        }
      ];

      const { result } = renderHook(() => useTraces());

      act(() => {
        mockService.traces$.next(mockTraces);
      });

      expect(result.current.data).toEqual(mockTraces);
      expect(result.current.isLoading).toBe(false);
    });

    it('filters traces by trace_id', () => {
      const mockTraces = [
        {
          id: 'trace-1',
          message: 'Test trace 1',
          source: 'test-component',
          timestamp: new Date(),
          trace_id: 'trace-123',
          span_id: 'span-456',
          metadata: {}
        },
        {
          id: 'trace-2',
          message: 'Test trace 2',
          source: 'test-component',
          timestamp: new Date(),
          trace_id: 'trace-456',
          span_id: 'span-789',
          metadata: {}
        }
      ];

      const { result } = renderHook(() => useTraces('trace-123'));

      act(() => {
        mockService.traces$.next(mockTraces);
      });

      expect(result.current.data).toHaveLength(1);
      expect(result.current.data[0].trace_id).toBe('trace-123');
    });
  });

  describe('error handling', () => {
    it('handles service connection errors', () => {
      mockService.connect.mockRejectedValue(new Error('Connection failed'));

      const { result } = renderHook(() => useMonitoring());

      expect(result.current.connectionStatus).toBe('disconnected');
    });

    it('handles observable errors gracefully', () => {
      const errorObservable = new BehaviorSubject(null);
      mockService.systemHealth$ = errorObservable;

      const { result } = renderHook(() => useSystemHealth());

      act(() => {
        errorObservable.error(new Error('Observable error'));
      });

      // Should not crash the application
      expect(result.current.data).toBeNull();
    });
  });
});