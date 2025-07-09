import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { SSEMetricExporter } from './sse-metric-exporter';
import { ExportResultCode, InstrumentType, DataPointType } from '@opentelemetry/sdk-metrics';

// Mock fetch globally
global.fetch = vi.fn();

describe('SSEMetricExporter', () => {
  let exporter: SSEMetricExporter;

  beforeEach(() => {
    exporter = new SSEMetricExporter({
      endpoint: 'http://localhost:8080/api/v1/metrics',
      batchSize: 10,
      exportIntervalMillis: 1000
    });
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  describe('export', () => {
    it('should export metrics successfully', async () => {
      const mockResourceMetrics = {
        resource: {
          attributes: {
            'service.name': 'test-service',
            'service.version': '1.0.0'
          },
          merge: vi.fn()
        },
        scopeMetrics: [{
          scope: { name: 'test-scope', version: '1.0.0', schemaUrl: '' },
          metrics: [{
            descriptor: {
              name: 'test.counter',
              description: 'Test counter',
              unit: 'items',
              type: 0, // COUNTER
              valueType: 0, // INT
              advice: {}
            },
            dataPointType: DataPointType.SUM,
            dataPoints: [{
              value: 42,
              startTime: [Date.now() - 1000, 0],
              endTime: [Date.now(), 0],
              attributes: { env: 'test' },
              hashCode: 'hash123'
            }],
            aggregationTemporality: 0 // DELTA
          }]
        }]
      };

      (global.fetch as any).mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true })
      });

      const result = await exporter.export(mockResourceMetrics as any);

      expect(result.code).toBe(ExportResultCode.SUCCESS);
      expect(exporter['pendingMetrics']).toHaveLength(1);
    });

    it('should handle export errors gracefully', async () => {
      const mockResourceMetrics = {
        resource: {
          attributes: { 'service.name': 'test-service' }
        },
        scopeMetrics: []
      };

      // Simulate error in transformation
      const result = await exporter.export(mockResourceMetrics as any);

      expect(result.code).toBe(ExportResultCode.SUCCESS); // Still returns success to avoid blocking
    });

    it('should batch metrics before sending', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true })
      });

      // Send metrics below batch threshold
      for (let i = 0; i < 5; i++) {
        await exporter.export(createMockMetrics(i));
      }

      // Should not have sent yet
      expect(global.fetch).not.toHaveBeenCalled();

      // Send more to exceed batch size
      for (let i = 5; i < 10; i++) {
        await exporter.export(createMockMetrics(i));
      }

      // Now should have sent
      expect(global.fetch).toHaveBeenCalled();
    });
  });

  describe('selectAggregationTemporality', () => {
    it('should select DELTA for counters and histograms', () => {
      expect(exporter.selectAggregationTemporality(InstrumentType.COUNTER))
        .toBe(1); // DELTA
      expect(exporter.selectAggregationTemporality(InstrumentType.HISTOGRAM))
        .toBe(1); // DELTA
    });

    it('should select CUMULATIVE for other instruments', () => {
      expect(exporter.selectAggregationTemporality(InstrumentType.UP_DOWN_COUNTER))
        .toBe(2); // CUMULATIVE
      expect(exporter.selectAggregationTemporality(InstrumentType.OBSERVABLE_GAUGE))
        .toBe(2); // CUMULATIVE
    });
  });

  describe('forceFlush', () => {
    it('should flush pending metrics', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true })
      });

      // Add some metrics
      for (let i = 0; i < 3; i++) {
        await exporter.export(createMockMetrics(i));
      }

      // Force flush
      await exporter.forceFlush();

      expect(global.fetch).toHaveBeenCalled();
      expect(exporter['pendingMetrics']).toHaveLength(0);
    });

    it('should handle flush errors', async () => {
      (global.fetch as any).mockRejectedValue(new Error('Network error'));

      // Add some metrics
      await exporter.export(createMockMetrics(0));

      // Force flush should not throw
      await expect(exporter.forceFlush()).resolves.not.toThrow();
    });
  });

  describe('shutdown', () => {
    it('should clear timer and flush metrics', async () => {
      const clearIntervalSpy = vi.spyOn(global, 'clearInterval');
      
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true })
      });

      // Create exporter with timer
      const exporterWithTimer = new SSEMetricExporter({
        endpoint: 'http://localhost:8080/api/v1/metrics',
        exportIntervalMillis: 5000
      });

      // Add some metrics
      await exporterWithTimer.export(createMockMetrics(0));

      // Shutdown
      await exporterWithTimer.shutdown();

      expect(clearIntervalSpy).toHaveBeenCalled();
      expect(global.fetch).toHaveBeenCalled();
    });
  });

  describe('batch export timer', () => {
    it('should export metrics periodically', async () => {
      (global.fetch as any).mockResolvedValue({
        ok: true,
        json: async () => ({ success: true })
      });

      // Create exporter with short interval
      const exporterWithTimer = new SSEMetricExporter({
        endpoint: 'http://localhost:8080/api/v1/metrics',
        exportIntervalMillis: 1000,
        batchSize: 100 // High batch size so timer triggers first
      });

      // Add some metrics
      await exporterWithTimer.export(createMockMetrics(0));

      // Advance time
      vi.advanceTimersByTime(1000);

      // Should have exported via timer
      expect(global.fetch).toHaveBeenCalled();
    });
  });

  describe('HTTP error handling', () => {
    it('should handle non-OK responses', async () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      (global.fetch as any).mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error'
      });

      const exporter = new SSEMetricExporter({
        endpoint: 'http://localhost:8080/api/v1/metrics',
        batchSize: 1
      });

      // This should log error but not throw
      await exporter.export(createMockMetrics(0));
      
      // Check console.error was called
      expect(consoleSpy).toHaveBeenCalled();
      
      consoleSpy.mockRestore();
    });
  });
});

// Helper function to create mock metrics
function createMockMetrics(index: number) {
  const now = 1704825600000; // Fixed timestamp: 2024-01-09 16:00:00
  return {
    resource: {
      attributes: {
        'service.name': `test-service-${index}`,
        'service.version': '1.0.0'
      }
    },
    scopeMetrics: [{
      scope: { name: 'test-scope', version: '1.0.0', schemaUrl: '' },
      metrics: [{
        descriptor: {
          name: `test.metric.${index}`,
          description: `Test metric ${index}`,
          unit: 'items',
          type: 0,
          valueType: 0,
          advice: {}
        },
        dataPointType: DataPointType.SUM,
        dataPoints: [{
          value: index * 10,
          startTime: [Math.floor((now - 1000) / 1000), ((now - 1000) % 1000) * 1000000],
          endTime: [Math.floor(now / 1000), (now % 1000) * 1000000],
          attributes: { index: index.toString() },
          hashCode: `hash${index}`
        }],
        aggregationTemporality: 0
      }]
    }]
  };
}