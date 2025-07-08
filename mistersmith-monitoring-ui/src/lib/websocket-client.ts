import { Subject, Observable, BehaviorSubject } from 'rxjs';
import { filter, map } from 'rxjs/operators';

export interface WSMessage {
  type: string;
  payload: any;
  correlationId?: string;
  timestamp: Date;
}

export interface WSCommand {
  type: 'subscribe' | 'unsubscribe' | 'filter' | 'command';
  payload: any;
  correlationId: string;
}

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectDelay = 30000;
  private reconnectTimer?: NodeJS.Timeout;
  
  private messages$ = new Subject<WSMessage>();
  private connectionStatus$ = new BehaviorSubject<'disconnected' | 'connecting' | 'connected' | 'error'>('disconnected');
  
  constructor(url: string) {
    this.url = url;
  }
  
  connect(): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      return;
    }
    
    this.connectionStatus$.next('connecting');
    
    try {
      this.ws = new WebSocket(this.url);
      
      this.ws.onopen = () => {
        console.log('WebSocket connected');
        this.connectionStatus$.next('connected');
        this.reconnectAttempts = 0;
      };
      
      this.ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WSMessage;
          message.timestamp = new Date(message.timestamp);
          this.messages$.next(message);
        } catch (error) {
          console.error('Failed to parse WebSocket message:', error);
        }
      };
      
      this.ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        this.connectionStatus$.next('error');
      };
      
      this.ws.onclose = () => {
        console.log('WebSocket disconnected');
        this.connectionStatus$.next('disconnected');
        this.scheduleReconnect();
      };
    } catch (error) {
      console.error('Failed to create WebSocket:', error);
      this.connectionStatus$.next('error');
      this.scheduleReconnect();
    }
  }
  
  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
    
    const delay = Math.min(
      1000 * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    );
    
    this.reconnectAttempts++;
    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
    
    this.reconnectTimer = setTimeout(() => {
      this.connect();
    }, delay);
  }
  
  disconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
    
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    
    this.connectionStatus$.next('disconnected');
  }
  
  send(command: WSCommand): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(command));
    } else {
      console.error('WebSocket is not connected');
    }
  }
  
  onMessage<T = any>(type: string): Observable<T> {
    return this.messages$.pipe(
      filter(msg => msg.type === type),
      map(msg => msg.payload as T)
    );
  }
  
  getConnectionStatus(): Observable<string> {
    return this.connectionStatus$.asObservable();
  }
  
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }
}