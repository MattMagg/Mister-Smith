import { 
  PushMetricExporter, 
  ResourceMetrics, 
  ExportResult, 
  ExportResultCode,
  AggregationTemporality,
  InstrumentType,
  DataPointType,
  AggregationTemporalitySelector
} from '@opentelemetry/sdk-metrics';

export interface SSEMetricExporterConfig {
  endpoint?: string;
  headers?: Record<string, string>;
  batchSize?: number;
  exportIntervalMillis?: number;
}

interface TransformedMetric {
  name: string;
  description?: string;
  unit?: string;
  type: 'counter' | 'gauge' | 'histogram';
  value?: number;
  sum?: number;
  count?: number;
  min?: number;
  max?: number;
  buckets?: Array<{ boundaries: number; count: number }>;
  isMonotonic?: boolean;
  scope: string;
  service: string;
  timestamp: string;
  attributes: Record<string, any>;
}

export class SSEMetricExporter implements PushMetricExporter {
  private endpoint: string;
  private headers: Record<string, string>;
  private batchSize: number;
  private pendingMetrics: TransformedMetric[] = [];
  private exportTimer?: NodeJS.Timeout;
  private _aggregationTemporalitySelector: AggregationTemporalitySelector;

  constructor(config: SSEMetricExporterConfig = {}) {
    this.endpoint = config.endpoint || 'http://localhost:8080/api/v1/metrics';
    this.headers = config.headers || {};
    this.batchSize = config.batchSize || 100;
    
    // Configure aggregation temporality selector
    this._aggregationTemporalitySelector = (instrumentType: InstrumentType) => {
      return this.selectAggregationTemporality(instrumentType);
    };
    
    // Start batch export timer if configured
    if (config.exportIntervalMillis) {
      this.startBatchExport(config.exportIntervalMillis);
    }
  }

  async export(metrics: ResourceMetrics): Promise<ExportResult> {
    try {
      // Transform OpenTelemetry metrics to MisterSmith format
      const transformedMetrics = this.transformMetrics(metrics);
      
      // Add to pending batch
      this.pendingMetrics.push(...transformedMetrics);
      
      // Export if batch size reached
      if (this.pendingMetrics.length >= this.batchSize) {
        await this.flushMetrics();
      }
      
      return { code: ExportResultCode.SUCCESS };
    } catch (error) {
      console.error('SSE Metric Export Error:', error);
      return { 
        code: ExportResultCode.FAILED, 
        error: error as Error 
      };
    }
  }

  private transformMetrics(resourceMetrics: ResourceMetrics): TransformedMetric[] {
    const transformed: TransformedMetric[] = [];
    const resource = resourceMetrics.resource;
    const serviceName = resource.attributes['service.name'] as string || 'mistersmith-ui';
    
    for (const scopeMetrics of resourceMetrics.scopeMetrics) {
      const scope = scopeMetrics.scope;
      
      for (const metric of scopeMetrics.metrics) {
        // Process each data point
        if ('dataPoints' in metric && metric.dataPoints) {
          for (const dataPoint of metric.dataPoints) {
            const baseMetric: Partial<TransformedMetric> = {
              name: metric.name,
              description: metric.descriptor.description,
              unit: metric.descriptor.unit,
              scope: scope.name || 'unknown',
              service: serviceName,
              timestamp: new Date(dataPoint.startTime[0] * 1000 + dataPoint.startTime[1] / 1000000).toISOString(),
              attributes: {
                ...dataPoint.attributes,
                ...resource.attributes
              }
            };

            // Handle different metric types based on descriptor type
            switch (metric.dataPointType) {
              case DataPointType.SUM:
                if ('value' in dataPoint) {
                  transformed.push({
                    ...baseMetric,
                    type: 'counter',
                    value: dataPoint.value as number,
                    isMonotonic: metric.isMonotonic
                  } as TransformedMetric);
                }
                break;
                
              case DataPointType.GAUGE:
                if ('value' in dataPoint) {
                  transformed.push({
                    ...baseMetric,
                    type: 'gauge',
                    value: dataPoint.value as number
                  } as TransformedMetric);
                }
                break;
                
              case DataPointType.HISTOGRAM:
                if ('sum' in dataPoint && 'count' in dataPoint) {
                  transformed.push({
                    ...baseMetric,
                    type: 'histogram',
                    sum: dataPoint.sum,
                    count: dataPoint.count,
                    min: dataPoint.min,
                    max: dataPoint.max,
                    buckets: 'buckets' in dataPoint && dataPoint.buckets ? 
                      dataPoint.buckets.boundaries.map((boundary, i) => ({
                        boundaries: boundary,
                        count: dataPoint.buckets!.counts[i] || 0
                      })) : undefined
                  } as TransformedMetric);
                }
                break;
            }
          }
        }
      }
    }
    
    return transformed;
  }

  private async flushMetrics(): Promise<void> {
    if (this.pendingMetrics.length === 0) return;
    
    const batch = this.pendingMetrics.splice(0, this.batchSize);
    
    try {
      await this.sendViaHTTP(batch);
    } catch (error) {
      // Re-queue failed metrics
      console.warn('Failed to send metrics, re-queueing:', error);
      this.pendingMetrics.unshift(...batch);
    }
  }

  private async sendViaHTTP(metrics: TransformedMetric[]): Promise<void> {
    const response = await fetch(this.endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...this.headers
      },
      body: JSON.stringify({ 
        metrics,
        timestamp: new Date().toISOString(),
        source: 'opentelemetry-js'
      })
    });
    
    if (!response.ok) {
      throw new Error(`Failed to export metrics: ${response.statusText}`);
    }
  }

  private startBatchExport(intervalMillis: number): void {
    this.exportTimer = setInterval(() => {
      this.flushMetrics().catch(console.error);
    }, intervalMillis);
  }

  async forceFlush(): Promise<void> {
    await this.flushMetrics();
  }

  async shutdown(): Promise<void> {
    if (this.exportTimer) {
      clearInterval(this.exportTimer);
      this.exportTimer = undefined;
    }
    await this.flushMetrics();
  }

  selectAggregationTemporality(instrumentType: InstrumentType): AggregationTemporality {
    // Use delta temporality for counters and histograms for efficiency
    switch (instrumentType) {
      case InstrumentType.COUNTER:
      case InstrumentType.HISTOGRAM:
      case InstrumentType.OBSERVABLE_COUNTER:
        return AggregationTemporality.DELTA;
      case InstrumentType.UP_DOWN_COUNTER:
      case InstrumentType.OBSERVABLE_GAUGE:
      case InstrumentType.OBSERVABLE_UP_DOWN_COUNTER:
        return AggregationTemporality.CUMULATIVE;
      default:
        return AggregationTemporality.CUMULATIVE;
    }
  }
}