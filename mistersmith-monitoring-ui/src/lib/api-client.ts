import axios, { AxiosInstance } from 'axios';
import { Discovery, DiscoveryFilter } from '../types/discovery';
import { Agent, AgentMetrics } from '../types/agent';

export interface PaginationParams {
  page?: number;
  pageSize?: number;
}

export interface PaginatedResponse<T> {
  data: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

class ApiClient {
  private client: AxiosInstance;
  
  constructor(baseURL = '/api/v1') {
    this.client = axios.create({
      baseURL,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    // Request interceptor for auth
    this.client.interceptors.request.use((config) => {
      const token = localStorage.getItem('auth_token');
      if (token) {
        config.headers.Authorization = `Bearer ${token}`;
      }
      return config;
    });
    
    // Response interceptor for error handling
    this.client.interceptors.response.use(
      (response) => response,
      (error) => {
        if (error.response?.status === 401) {
          // Handle unauthorized
          window.location.href = '/login';
        }
        return Promise.reject(error);
      }
    );
  }
  
  // Discovery endpoints
  async getDiscoveries(
    filter?: DiscoveryFilter,
    pagination?: PaginationParams
  ): Promise<PaginatedResponse<Discovery>> {
    const { data } = await this.client.get('/discoveries', {
      params: {
        ...filter,
        ...pagination,
      },
    });
    return data;
  }
  
  async getDiscovery(id: string): Promise<Discovery> {
    const { data } = await this.client.get(`/discoveries/${id}`);
    return data;
  }
  
  async filterDiscoveries(filter: DiscoveryFilter): Promise<Discovery[]> {
    const { data } = await this.client.post('/discoveries/filter', filter);
    return data;
  }
  
  // Agent endpoints
  async getAgents(): Promise<Agent[]> {
    const { data } = await this.client.get('/agents');
    return data;
  }
  
  async getAgent(id: string): Promise<Agent> {
    const { data } = await this.client.get(`/agents/${id}`);
    return data;
  }
  
  async getAgentDiscoveries(
    agentId: string,
    pagination?: PaginationParams
  ): Promise<PaginatedResponse<Discovery>> {
    const { data } = await this.client.get(`/agents/${agentId}/discoveries`, {
      params: pagination,
    });
    return data;
  }
  
  async getAgentMetrics(agentId: string): Promise<AgentMetrics> {
    const { data } = await this.client.get(`/agents/${agentId}/metrics`);
    return data;
  }
  
  // Metrics endpoints
  async getSystemMetrics(): Promise<any> {
    const { data } = await this.client.get('/metrics/system');
    return data;
  }
  
  async getNatsMetrics(): Promise<any> {
    const { data } = await this.client.get('/metrics/nats');
    return data;
  }
  
  async getMetricsHistory(timeRange: { start: Date; end: Date }): Promise<any> {
    const { data } = await this.client.get('/metrics/history', {
      params: {
        start: timeRange.start.toISOString(),
        end: timeRange.end.toISOString(),
      },
    });
    return data;
  }
  
  // Debug endpoints
  async getLogs(filters?: any): Promise<any> {
    const { data } = await this.client.get('/debug/logs', { params: filters });
    return data;
  }
  
  async submitTestDiscovery(discovery: Partial<Discovery>): Promise<Discovery> {
    const { data } = await this.client.post('/debug/test-discovery', discovery);
    return data;
  }
  
  async traceDiscovery(id: string): Promise<any> {
    const { data } = await this.client.get(`/debug/trace/${id}`);
    return data;
  }
  
  // Health endpoints
  async getHealth(): Promise<any> {
    const { data } = await this.client.get('/health');
    return data;
  }
  
  async getComponentHealth(): Promise<any> {
    const { data } = await this.client.get('/health/components');
    return data;
  }
}

export const apiClient = new ApiClient();