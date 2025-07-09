import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { JaegerClient } from './jaeger-client';

// Mock fetch globally
global.fetch = vi.fn();

describe('JaegerClient', () => {
  let client: JaegerClient;
  
  beforeEach(() => {
    client = new JaegerClient({
      baseUrl: 'http://localhost:16686',
      timeout: 1000,
      retryAttempts: 2
    });
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('getServices', () => {
    it('should fetch and return services list', async () => {
      const mockServices = {
        data: ['mistersmith-agent', 'claude-code', 'discovery-service'],
        total: 3,
        limit: 0,
        offset: 0,
        errors: null
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockServices
      });

      const services = await client.getServices();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:16686/api/services',
        expect.objectContaining({
          headers: { 'Content-Type': 'application/json' }
        })
      );
      expect(services).toEqual(['mistersmith-agent', 'claude-code', 'discovery-service']);
    });

    it('should return empty array on error', async () => {
      (global.fetch as any).mockRejectedValueOnce(new Error('Network error'));

      const services = await client.getServices();

      expect(services).toEqual([]);
    });

    it('should use cached data within TTL', async () => {
      const mockServices = {
        data: ['service1', 'service2'],
        total: 2,
        limit: 0,
        offset: 0,
        errors: null
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockServices
      });

      // First call
      await client.getServices();
      
      // Second call (should use cache)
      const services = await client.getServices();

      expect(global.fetch).toHaveBeenCalledTimes(1);
      expect(services).toEqual(['service1', 'service2']);
    });
  });

  describe('searchTraces', () => {
    it('should search traces with correct parameters', async () => {
      const mockTraces = {
        data: [
          {
            traceID: 'trace1',
            spans: [],
            processes: {},
            warnings: []
          }
        ],
        total: 1,
        limit: 100,
        offset: 0,
        errors: null
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockTraces
      });

      const traces = await client.searchTraces({
        service: 'claude-code',
        operation: 'submitTask',
        lookback: '1h',
        limit: 50,
        tags: { error: 'true' }
      });

      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/traces?'),
        expect.any(Object)
      );
      
      const url = (global.fetch as any).mock.calls[0][0];
      expect(url).toContain('service=claude-code');
      expect(url).toContain('operation=submitTask');
      expect(url).toContain('lookback=1h');
      expect(url).toContain('limit=50');
      expect(url).toContain('tags=%7B%22error%22%3A%22true%22%7D');
      
      expect(traces).toHaveLength(1);
      expect(traces[0].traceID).toBe('trace1');
    });

    it('should handle search errors gracefully', async () => {
      (global.fetch as any).mockRejectedValueOnce(new Error('API error'));

      const traces = await client.searchTraces({ service: 'test-service' });

      expect(traces).toEqual([]);
    });
  });

  describe('getMetrics', () => {
    it('should fetch call rate metrics', async () => {
      const mockMetrics = {
        metrics: [
          {
            name: 'operation1',
            metricPoints: [
              { timestamp: 1234567890, value: { y: 100 } },
              { timestamp: 1234567900, value: { y: 120 } }
            ]
          }
        ]
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockMetrics
      });

      const result = await client.getCallRates({
        service: 'claude-code',
        lookback: 3600000,
        step: 60000,
        ratePer: 60000
      });

      const url = (global.fetch as any).mock.calls[0][0];
      expect(url).toContain('/api/metrics/calls?');
      expect(url).toContain('service=claude-code');
      expect(url).toContain('lookback=3600000');
      
      expect(result.metrics).toHaveLength(1);
      expect(result.metrics[0].name).toBe('operation1');
      expect(result.metrics[0].values).toHaveLength(2);
      expect(result.metrics[0].values[0].y).toBe(100);
    });

    it('should handle latency metrics with quantile', async () => {
      const mockMetrics = {
        metrics: [
          {
            name: 'P95 Latency',
            metricPoints: [
              { timestamp: 1234567890, value: { y: 250 } }
            ]
          }
        ]
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => mockMetrics
      });

      const result = await client.getLatencies({
        service: 'claude-code',
        quantile: 0.95,
        lookback: 3600000,
        step: 60000,
        ratePer: 60000
      });

      const url = (global.fetch as any).mock.calls[0][0];
      expect(url).toContain('/api/metrics/latencies?');
      expect(url).toContain('quantile=0.95');
      
      expect(result.metrics[0].values[0].y).toBe(250);
    });
  });

  describe('retry mechanism', () => {
    it('should retry on failure', async () => {
      const mockServices = {
        data: ['service1'],
        total: 1,
        limit: 0,
        offset: 0,
        errors: null
      };

      // First call fails, second succeeds
      (global.fetch as any)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => mockServices
        });

      const services = await client.getServices();

      expect(global.fetch).toHaveBeenCalledTimes(2);
      expect(services).toEqual(['service1']);
    });

    it('should respect retry limit', async () => {
      (global.fetch as any)
        .mockRejectedValue(new Error('Network error'));

      const services = await client.getServices();

      // Initial call + 1 retry (retryAttempts = 2)
      expect(global.fetch).toHaveBeenCalledTimes(2);
      expect(services).toEqual([]);
    });

    it('should timeout long requests', async () => {
      // Mock fetch to simulate a request that is aborted
      (global.fetch as any).mockImplementation(() => {
        return new Promise((_, reject) => {
          setTimeout(() => {
            reject(new DOMException('The user aborted a request.', 'AbortError'));
          }, 100);
        });
      });

      // This should timeout and return empty array
      const services = await client.getServices();
      expect(services).toEqual([]);
    });
  });

  describe('cache management', () => {
    it('should clear cache on demand', async () => {
      const mockServices1 = {
        data: ['service1'],
        total: 1,
        limit: 0,
        offset: 0,
        errors: null
      };

      const mockServices2 = {
        data: ['service2'],
        total: 1,
        limit: 0,
        offset: 0,
        errors: null
      };

      (global.fetch as any)
        .mockResolvedValueOnce({
          ok: true,
          json: async () => mockServices1
        })
        .mockResolvedValueOnce({
          ok: true,
          json: async () => mockServices2
        });

      // First call
      const services1 = await client.getServices();
      expect(services1).toEqual(['service1']);

      // Clear cache
      client.clearCache();

      // Second call (should not use cache)
      const services2 = await client.getServices();
      expect(services2).toEqual(['service2']);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });
  });
});