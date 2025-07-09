// Jaeger REST API client for trace querying
// Using inline types to avoid TypeScript import issues

export interface JaegerConfig {
  baseUrl: string;
  headers?: Record<string, string>;
  timeout?: number;
  retryAttempts?: number;
}

export interface ServiceList {
  data: string[];
  total: number;
  limit: number;
  offset: number;
  errors: null | string[];
}

export interface Operation {
  name: string;
  spanKind: string;
}

export interface TraceSearchParams {
  service: string;
  operation?: string;
  tags?: Record<string, any>;
  start?: number; // microseconds
  end?: number;
  minDuration?: string;
  maxDuration?: string;
  limit?: number;
  lookback?: string; // "1h", "30m", etc.
}

export interface Trace {
  traceID: string;
  spans: Span[];
  processes: Record<string, Process>;
  warnings?: string[];
}

export interface Span {
  traceID: string;
  spanID: string;
  operationName: string;
  references: Reference[];
  startTime: number;
  duration: number;
  tags: Tag[];
  logs: Log[];
  processID: string;
  warnings?: string[];
}

export interface Process {
  serviceName: string;
  tags: Tag[];
}

export interface Reference {
  refType: string;
  traceID: string;
  spanID: string;
}

export interface Tag {
  key: string;
  type: string;
  value: any;
}

export interface Log {
  timestamp: number;
  fields: Tag[];
}

export interface TraceSearchResult {
  data: Trace[];
  total: number;
  limit: number;
  offset: number;
  errors: null | string[];
}

export interface MetricsParams {
  service: string;
  spanKind?: string[];
  groupByOperation?: boolean;
  endTs?: number;
  lookback?: number; // milliseconds
  step?: number; // milliseconds
  ratePer?: number; // milliseconds
  quantile?: number; // for latency metrics
}

export interface MetricPoint {
  x: number; // timestamp
  y: number; // value
}

export interface MetricSeries {
  name: string;
  values: MetricPoint[];
}

export interface MetricsResult {
  metrics: MetricSeries[];
}

export class JaegerClient {
  private config: JaegerConfig;
  private cache: Map<string, { data: any; timestamp: number }> = new Map();
  private readonly CACHE_TTL = 60000; // 1 minute

  constructor(config: JaegerConfig) {
    this.config = {
      ...config,
      headers: {
        'Content-Type': 'application/json',
        ...config.headers,
      },
      timeout: config.timeout || 30000,
      retryAttempts: config.retryAttempts || 3,
    };
  }

  async getServices(): Promise<string[]> {
    const cacheKey = 'services';
    const cached = this.getFromCache<ServiceList>(cacheKey);
    if (cached) return cached.data || [];

    try {
      const response = await this.fetchWithRetry('/api/services');
      const data: ServiceList = await response.json();
      this.setCache(cacheKey, data);
      return data.data || [];
    } catch (error) {
      console.error('Failed to fetch services:', error);
      return [];
    }
  }

  async getOperations(service: string, spanKind?: string): Promise<Operation[]> {
    const cacheKey = `operations:${service}:${spanKind || 'all'}`;
    const cached = this.getFromCache<{ data: Operation[] }>(cacheKey);
    if (cached) return cached.data || [];

    try {
      const params = new URLSearchParams({ service });
      if (spanKind) params.append('spanKind', spanKind);
      
      const response = await this.fetchWithRetry(`/api/operations?${params}`);
      const data = await response.json();
      this.setCache(cacheKey, data);
      return data.data || [];
    } catch (error) {
      console.error('Failed to fetch operations:', error);
      return [];
    }
  }

  async searchTraces(params: TraceSearchParams): Promise<Trace[]> {
    try {
      const queryParams = this.buildTraceQueryParams(params);
      const response = await this.fetchWithRetry(`/api/traces?${queryParams}`);
      const data: TraceSearchResult = await response.json();
      return data.data || [];
    } catch (error) {
      console.error('Failed to search traces:', error);
      return [];
    }
  }

  async getTrace(traceId: string): Promise<Trace | null> {
    const cacheKey = `trace:${traceId}`;
    const cached = this.getFromCache<{ data: Trace[] }>(cacheKey);
    if (cached && cached.data?.length) return cached.data[0];

    try {
      const response = await this.fetchWithRetry(`/api/traces/${traceId}`);
      const data = await response.json();
      this.setCache(cacheKey, data);
      return data.data?.[0] || null;
    } catch (error) {
      console.error('Failed to fetch trace:', error);
      return null;
    }
  }

  async getMetrics(params: MetricsParams & { metric: 'calls' | 'latencies' | 'errors' }): Promise<MetricsResult> {
    try {
      const queryParams = this.buildMetricsQueryParams(params);
      const response = await this.fetchWithRetry(`/api/metrics/${params.metric}?${queryParams}`);
      const data = await response.json();
      return this.transformMetricsResponse(data);
    } catch (error) {
      console.error(`Failed to fetch ${params.metric} metrics:`, error);
      return { metrics: [] };
    }
  }

  async getCallRates(params: MetricsParams): Promise<MetricsResult> {
    return this.getMetrics({ ...params, metric: 'calls' });
  }

  async getErrorRates(params: MetricsParams): Promise<MetricsResult> {
    return this.getMetrics({ ...params, metric: 'errors' });
  }

  async getLatencies(params: MetricsParams & { quantile: number }): Promise<MetricsResult> {
    return this.getMetrics({ ...params, metric: 'latencies' });
  }

  private async fetchWithRetry(path: string, attempt = 0): Promise<Response> {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(`${this.config.baseUrl}${path}`, {
        headers: this.config.headers,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return response;
    } catch (error) {
      clearTimeout(timeoutId);

      if (attempt < this.config.retryAttempts - 1) {
        const delay = Math.min(1000 * Math.pow(2, attempt), 10000);
        await new Promise(resolve => setTimeout(resolve, delay));
        return this.fetchWithRetry(path, attempt + 1);
      }

      throw error;
    }
  }

  private buildTraceQueryParams(params: TraceSearchParams): URLSearchParams {
    const queryParams = new URLSearchParams();
    queryParams.append('service', params.service);

    if (params.operation) queryParams.append('operation', params.operation);
    if (params.lookback) queryParams.append('lookback', params.lookback);
    if (params.limit) queryParams.append('limit', params.limit.toString());
    if (params.start) queryParams.append('start', params.start.toString());
    if (params.end) queryParams.append('end', params.end.toString());
    if (params.minDuration) queryParams.append('minDuration', params.minDuration);
    if (params.maxDuration) queryParams.append('maxDuration', params.maxDuration);
    if (params.tags) queryParams.append('tags', JSON.stringify(params.tags));

    return queryParams;
  }

  private buildMetricsQueryParams(params: MetricsParams): URLSearchParams {
    const queryParams = new URLSearchParams();
    queryParams.append('service', params.service);
    queryParams.append('endTs', (params.endTs || Date.now()).toString());

    if (params.spanKind?.length) {
      params.spanKind.forEach(kind => queryParams.append('spanKind', kind));
    }
    if (params.groupByOperation) queryParams.append('groupByOperation', 'true');
    if (params.lookback) queryParams.append('lookback', params.lookback.toString());
    if (params.step) queryParams.append('step', params.step.toString());
    if (params.ratePer) queryParams.append('ratePer', params.ratePer.toString());
    if (params.quantile) queryParams.append('quantile', params.quantile.toString());

    return queryParams;
  }

  private transformMetricsResponse(data: any): MetricsResult {
    if (!data.metrics) return { metrics: [] };

    return {
      metrics: data.metrics.map((metric: any) => ({
        name: metric.name || metric.labels?.operation_name || 'unknown',
        values: metric.metricPoints?.map((point: any) => ({
          x: point.timestamp || point.x,
          y: point.value?.y || point.y || 0,
        })) || [],
      })),
    };
  }

  private getFromCache<T>(key: string): T | null {
    const cached = this.cache.get(key);
    if (!cached) return null;

    if (Date.now() - cached.timestamp > this.CACHE_TTL) {
      this.cache.delete(key);
      return null;
    }

    return cached.data as T;
  }

  private setCache(key: string, data: any): void {
    this.cache.set(key, { data, timestamp: Date.now() });
  }

  clearCache(): void {
    this.cache.clear();
  }
}

// Factory function to create client instance
export function createJaegerClient(config: JaegerConfig): JaegerClient {
  return new JaegerClient(config);
}