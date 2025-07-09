import { Subject, Observable, BehaviorSubject } from 'rxjs';

// Inline type to avoid import issues
type Discovery = {
  id: string;
  type: string;
  content: string;
  confidence: number;
  agentId: string;
  timestamp: string;
};

export interface SSEEvent {
  type: 'discovery' | 'metric' | 'alert' | 'heartbeat' | 'log' | 'trace';
  data: any;
  id?: string;
  retry?: number;
}

export class SSEClient {
  private eventSource: EventSource | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectDelay = 30000;
  
  private events$ = new Subject<SSEEvent>();
  private connectionStatus$ = new BehaviorSubject<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected');
  
  constructor(url: string) {
    this.url = url;
  }
  
  connect(token?: string): void {
    if (this.eventSource?.readyState === EventSource.OPEN) {
      return;
    }
    
    this.connectionStatus$.next('connecting');
    
    try {
      const urlWithToken = token ? `${this.url}?token=${token}` : this.url;
      this.eventSource = new EventSource(urlWithToken);
      
      this.eventSource.onopen = () => {
        console.log('SSE connected');
        this.connectionStatus$.next('connected');
        this.reconnectAttempts = 0;
      };
      
      this.eventSource.onerror = (error) => {
        console.error('SSE error:', error);
        this.connectionStatus$.next('error');
        
        if (this.eventSource?.readyState === EventSource.CLOSED) {
          this.scheduleReconnect();
        }
      };
      
      // Discovery events
      this.eventSource.addEventListener('discovery', (event) => {
        try {
          const discovery = JSON.parse(event.data) as Discovery;
          discovery.timestamp = new Date(discovery.timestamp);
          this.events$.next({
            type: 'discovery',
            data: discovery,
            id: event.lastEventId,
          });
        } catch (error) {
          console.error('Failed to parse discovery event:', error);
        }
      });
      
      // Metric events
      this.eventSource.addEventListener('metric', (event) => {
        try {
          const metric = JSON.parse(event.data);
          this.events$.next({
            type: 'metric',
            data: metric,
            id: event.lastEventId,
          });
        } catch (error) {
          console.error('Failed to parse metric event:', error);
        }
      });
      
      // Alert events
      this.eventSource.addEventListener('alert', (event) => {
        try {
          const alert = JSON.parse(event.data);
          this.events$.next({
            type: 'alert',
            data: alert,
            id: event.lastEventId,
          });
        } catch (error) {
          console.error('Failed to parse alert event:', error);
        }
      });
      
      // Heartbeat events
      this.eventSource.addEventListener('heartbeat', (event) => {
        this.events$.next({
          type: 'heartbeat',
          data: event.data,
          id: event.lastEventId,
        });
      });
      
      // Log events
      this.eventSource.addEventListener('log', (event) => {
        try {
          const logEntry = JSON.parse(event.data);
          this.events$.next({
            type: 'log',
            data: logEntry,
            id: event.lastEventId,
          });
        } catch (error) {
          console.error('Failed to parse log event:', error);
        }
      });
      
      // Trace events
      this.eventSource.addEventListener('trace', (event) => {
        try {
          const traceData = JSON.parse(event.data);
          this.events$.next({
            type: 'trace',
            data: traceData,
            id: event.lastEventId,
          });
        } catch (error) {
          console.error('Failed to parse trace event:', error);
        }
      });
      
    } catch (error) {
      console.error('Failed to create EventSource:', error);
      this.connectionStatus$.next('error');
      this.scheduleReconnect();
    }
  }
  
  private scheduleReconnect(): void {
    const delay = Math.min(
      1000 * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    );
    
    this.reconnectAttempts++;
    console.log(`SSE reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    
    setTimeout(() => {
      this.connect();
    }, delay);
  }
  
  disconnect(): void {
    if (this.eventSource) {
      this.eventSource.close();
      this.eventSource = null;
    }
    
    this.connectionStatus$.next('disconnected');
  }
  
  onEvent<T = any>(type: SSEEvent['type']): Observable<T> {
    return new Observable(subscriber => {
      const subscription = this.events$.subscribe(event => {
        if (event.type === type) {
          subscriber.next(event.data as T);
        }
      });
      
      return () => subscription.unsubscribe();
    });
  }
  
  getConnectionStatus(): Observable<string> {
    return this.connectionStatus$.asObservable();
  }
  
  isConnected(): boolean {
    return this.eventSource?.readyState === EventSource.OPEN;
  }
}