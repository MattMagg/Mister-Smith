import { BehaviorSubject, Observable, combineLatest, interval, of } from 'rxjs';
import { catchError, map, switchMap, distinctUntilChanged, shareReplay, startWith } from 'rxjs/operators';
import { SSEClient, SSEEvent } from './sse-client';
import { apiClient } from './api-client';
import { JaegerClient } from './telemetry/jaeger-client';
import { 
  recordDiscovery, 
  recordSSEEvent, 
  recordError,
  updateActiveAgents,
  updateSystemHealth
} from './telemetry/custom-metrics';
import { Discovery } from '../types/discovery';
import { Agent } from '../types/agent';

export interface SystemHealth {
  status: 'healthy' | 'degraded' | 'unhealthy';
  components: {
    mcp_server: 'up' | 'down' | 'unknown';
    nats_server: 'up' | 'down' | 'unknown';
    database: 'up' | 'down' | 'unknown';
  };
  uptime: number;
  version: string;
}

export interface SystemMetrics {
  cpu_usage: number;
  memory_usage: number;
  active_agents: number;
  total_discoveries: number;
  nats_connections: number;
  message_throughput: number;
  timestamp: Date;
}

export interface ErrorEvent {
  id: string;
  message: string;
  stack?: string;
  source: string;
  timestamp: Date;
  severity: 'low' | 'medium' | 'high' | 'critical';
}

export interface LogEntry {
  id: string;
  timestamp: Date;
  level: 'debug' | 'info' | 'warn' | 'error';
  message: string;
  source: string;
  metadata?: Record<string, any>;
}

export interface TraceEvent {
  id: string;
  timestamp: Date;
  traceId: string;
  spanId: string;
  operation: string;
  duration: number;
  tags: Record<string, any>;
  logs: LogEntry[];
}

export class MonitoringService {
  private sseClient: SSEClient;
  private jaegerClient: JaegerClient;
  private connectionStatus$ = new BehaviorSubject<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected');
  private systemHealth$ = new BehaviorSubject<SystemHealth | null>(null);
  private systemMetrics$ = new BehaviorSubject<SystemMetrics | null>(null);
  private discoveries$ = new BehaviorSubject<Discovery[]>([]);
  private agents$ = new BehaviorSubject<Agent[]>([]);
  private errors$ = new BehaviorSubject<ErrorEvent[]>([]);
  private logs$ = new BehaviorSubject<LogEntry[]>([]);
  private traces$ = new BehaviorSubject<TraceEvent[]>([]);
  
  constructor(private config: { 
    sseEndpoint: string;
    apiEndpoint: string;
    mcpEndpoint: string;
    jaegerEndpoint?: string;
  }) {
    this.sseClient = new SSEClient(config.sseEndpoint);
    this.jaegerClient = new JaegerClient({
      baseUrl: config.jaegerEndpoint || 'http://localhost:16686',
      timeout: 30000
    });
    // Don't initialize connection automatically - wait for explicit connect() call
    // this.initializeConnection();
  }
  
  // Public method to start the connection
  public connect(): void {
    this.initializeConnection();
  }

  private initializeConnection(): void {
    // Initialize SSE connection
    this.sseClient.connect();
    
    // Monitor SSE connection status
    this.sseClient.getConnectionStatus().subscribe(status => {
      this.connectionStatus$.next(status as any);
    });
    
    // Setup discovery stream
    this.sseClient.onEvent<Discovery>('discovery').subscribe(discovery => {
      const currentDiscoveries = this.discoveries$.value;
      const newDiscoveries = [discovery, ...currentDiscoveries].slice(0, 100); // Keep last 100
      this.discoveries$.next(newDiscoveries);
      
      // Record telemetry
      try {
        recordDiscovery(discovery);
        recordSSEEvent('discovery');
      } catch (error) {
        console.error('Failed to record discovery telemetry:', error);
      }
    });
    
    // Setup metrics stream
    this.sseClient.onEvent<SystemMetrics>('metric').subscribe(metrics => {
      this.systemMetrics$.next({
        ...metrics,
        timestamp: new Date()
      });
      
      // Record telemetry
      try {
        recordSSEEvent('metric');
        if (metrics.active_agents !== undefined) {
          updateActiveAgents(metrics.active_agents);
        }
      } catch (error) {
        console.error('Failed to record metrics telemetry:', error);
      }
    });
    
    // Setup error stream
    this.sseClient.onEvent<ErrorEvent>('alert').subscribe(error => {
      const currentErrors = this.errors$.value;
      const newErrors = [error, ...currentErrors].slice(0, 50); // Keep last 50
      this.errors$.next(newErrors);
      
      // Record telemetry
      try {
        recordSSEEvent('alert');
        recordError(new Error(error.message), error.source, error.severity);
      } catch (err) {
        console.error('Failed to record error telemetry:', err);
      }
    });
    
    // Setup log stream
    this.sseClient.onEvent<LogEntry>('log').subscribe(logEntry => {
      const currentLogs = this.logs$.value;
      const newLogs = [{
        ...logEntry,
        timestamp: new Date(logEntry.timestamp)
      }, ...currentLogs].slice(0, 500); // Keep last 500 logs
      this.logs$.next(newLogs);
    });
    
    // Setup trace stream
    this.sseClient.onEvent<TraceEvent>('trace').subscribe(traceEvent => {
      const currentTraces = this.traces$.value;
      const newTraces = [{
        ...traceEvent,
        timestamp: new Date(traceEvent.timestamp)
      }, ...currentTraces].slice(0, 100); // Keep last 100 traces
      this.traces$.next(newTraces);
    });
    
    // Start periodic health checks
    this.startHealthChecks();
    
    // Start periodic metrics collection
    this.startMetricsCollection();
  }
  
  private startHealthChecks(): void {
    interval(30000) // Every 30 seconds
      .pipe(
        startWith(0),
        switchMap(() => this.fetchSystemHealth()),
        catchError(error => {
          console.error('Health check failed:', error);
          return of({
            status: 'unhealthy' as const,
            components: {
              mcp_server: 'down' as const,
              nats_server: 'unknown' as const,
              database: 'unknown' as const,
            },
            uptime: 0,
            version: 'unknown'
          });
        })
      )
      .subscribe(health => {
        this.systemHealth$.next(health);
        
        // Record telemetry
        try {
          updateSystemHealth(health.status);
        } catch (error) {
          console.error('Failed to record health telemetry:', error);
        }
      });
  }
  
  private startMetricsCollection(): void {
    interval(10000) // Every 10 seconds
      .pipe(
        startWith(0),
        switchMap(() => this.fetchSystemMetrics()),
        catchError(error => {
          console.error('Metrics collection failed:', error);
          return of(null);
        })
      )
      .subscribe(metrics => {
        if (metrics) {
          this.systemMetrics$.next(metrics);
        }
      });
  }
  
  private async fetchSystemHealth(): Promise<SystemHealth> {
    try {
      // Try to use MCP server health endpoint
      const response = await fetch(`${this.config.mcpEndpoint}/health`);
      if (response.ok) {
        const data = await response.json();
        return {
          status: data.status === 'ok' ? 'healthy' : 'degraded',
          components: {
            mcp_server: 'up',
            nats_server: data.nats_connected ? 'up' : 'down',
            database: data.database_connected ? 'up' : 'down',
          },
          uptime: data.uptime || 0,
          version: data.version || 'unknown'
        };
      }
    } catch (error) {
      console.warn('MCP health check failed, using fallback:', error);
    }
    
    // Fallback to API client
    try {
      const health = await apiClient.getHealth();
      return {
        status: health.status === 'ok' ? 'healthy' : 'degraded',
        components: {
          mcp_server: 'up',
          nats_server: health.nats_connected ? 'up' : 'down',
          database: health.database_connected ? 'up' : 'down',
        },
        uptime: health.uptime || 0,
        version: health.version || 'unknown'
      };
    } catch (error) {
      throw new Error('All health check methods failed');
    }
  }
  
  private async fetchSystemMetrics(): Promise<SystemMetrics | null> {
    try {
      // Try to use MCP server metrics endpoint
      const response = await fetch(`${this.config.mcpEndpoint}/metrics`);
      if (response.ok) {
        const data = await response.json();
        return {
          cpu_usage: data.cpu_usage || 0,
          memory_usage: data.memory_usage || 0,
          active_agents: data.active_agents || 0,
          total_discoveries: data.total_discoveries || 0,
          nats_connections: data.nats_connections || 0,
          message_throughput: data.message_throughput || 0,
          timestamp: new Date()
        };
      }
    } catch (error) {
      console.warn('MCP metrics failed, using fallback:', error);
    }
    
    // Fallback to API client
    try {
      const metrics = await apiClient.getSystemMetrics();
      return {
        cpu_usage: metrics.cpu_usage || 0,
        memory_usage: metrics.memory_usage || 0,
        active_agents: metrics.active_agents || 0,
        total_discoveries: metrics.total_discoveries || 0,
        nats_connections: metrics.nats_connections || 0,
        message_throughput: metrics.message_throughput || 0,
        timestamp: new Date()
      };
    } catch (error) {
      console.error('All metrics collection methods failed:', error);
      return null;
    }
  }
  
  // Public API
  getConnectionStatus(): Observable<string> {
    return this.connectionStatus$.asObservable();
  }
  
  getSystemHealth(): Observable<SystemHealth | null> {
    return this.systemHealth$.asObservable();
  }
  
  getSystemMetrics(): Observable<SystemMetrics | null> {
    return this.systemMetrics$.asObservable();
  }
  
  getDiscoveries(): Observable<Discovery[]> {
    return this.discoveries$.asObservable();
  }
  
  getAgents(): Observable<Agent[]> {
    return this.agents$.asObservable();
  }
  
  getErrors(): Observable<ErrorEvent[]> {
    return this.errors$.asObservable();
  }
  
  getLogs(): Observable<LogEntry[]> {
    return this.logs$.asObservable();
  }
  
  getTraces(): Observable<TraceEvent[]> {
    return this.traces$.asObservable();
  }
  
  // Filtered log streams
  getLogsByLevel(level: LogEntry['level']): Observable<LogEntry[]> {
    return this.logs$.pipe(
      map(logs => logs.filter(log => log.level === level))
    );
  }
  
  getErrorLogs(): Observable<LogEntry[]> {
    return this.getLogsByLevel('error');
  }
  
  getRecentLogs(count: number = 50): Observable<LogEntry[]> {
    return this.logs$.pipe(
      map(logs => logs.slice(0, count))
    );
  }
  
  // Real-time status for dashboard
  getDashboardStatus(): Observable<{
    isConnected: boolean;
    totalDiscoveries: number;
    activeAgents: number;
    systemHealth: SystemHealth | null;
    recentErrors: ErrorEvent[];
    recentLogs: LogEntry[];
  }> {
    return combineLatest([
      this.connectionStatus$,
      this.discoveries$,
      this.agents$,
      this.systemHealth$,
      this.errors$,
      this.logs$
    ]).pipe(
      map(([status, discoveries, agents, health, errors, logs]) => ({
        isConnected: status === 'connected',
        totalDiscoveries: discoveries.length,
        activeAgents: agents.filter(a => a.status === 'running').length,
        systemHealth: health,
        recentErrors: errors.slice(0, 5),
        recentLogs: logs.slice(0, 10)
      })),
      distinctUntilChanged(),
      shareReplay(1)
    );
  }
  
  // Control methods
  async shareDiscovery(discovery: Partial<Discovery>): Promise<Discovery> {
    return apiClient.submitTestDiscovery(discovery);
  }
  
  async getHistoricalLogs(filters?: any): Promise<any[]> {
    return apiClient.getLogs(filters);
  }
  
  async traceDiscovery(id: string): Promise<any> {
    return apiClient.traceDiscovery(id);
  }
  
  // Jaeger integration methods
  async getJaegerServices(): Promise<string[]> {
    return this.jaegerClient.getServices();
  }
  
  async getTracesByService(service: string, lookback = '1h'): Promise<any[]> {
    return this.jaegerClient.searchTraces({ service, lookback, limit: 100 });
  }
  
  async getServiceMetrics(service: string): Promise<{
    callRate: any;
    errorRate: any;
    latency: any;
  }> {
    const [callRate, errorRate, latency] = await Promise.all([
      this.jaegerClient.getCallRates({
        service,
        lookback: 3600000, // 1 hour
        step: 60000, // 1 minute
        ratePer: 60000, // per minute
      }),
      this.jaegerClient.getErrorRates({
        service,
        lookback: 3600000,
        step: 60000,
        ratePer: 60000,
      }),
      this.jaegerClient.getLatencies({
        service,
        quantile: 0.95,
        lookback: 3600000,
        step: 60000,
        ratePer: 60000,
      }),
    ]);

    return { callRate, errorRate, latency };
  }
  
  async getTraceDetails(traceId: string): Promise<any> {
    return this.jaegerClient.getTrace(traceId);
  }
  
  disconnect(): void {
    this.sseClient.disconnect();
    this.connectionStatus$.next('disconnected');
  }
}

// Singleton instance
export const monitoringService = new MonitoringService({
  sseEndpoint: 'http://localhost:8080/sse/stream',
  apiEndpoint: '/api/v1',
  mcpEndpoint: 'http://localhost:8080',
  jaegerEndpoint: 'http://localhost:16686'
});
