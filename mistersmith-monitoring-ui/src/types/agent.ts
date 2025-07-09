export type AgentStatus = 'created' | 'running' | 'terminated' | 'error';

export interface Agent {
  id: string;
  type?: string;
  status: AgentStatus | string;
  createdAt?: string;
  lastActivity?: string;
  capabilities?: string[];
  metadata?: Record<string, any>;
  // UI display fields
  discoveries?: number;
  uptime?: string;
}

export interface AgentMetrics {
  agentId: string;
  discoveryCount: number;
  successRate: number;
  avgResponseTime: number;
  errorCount: number;
  lastHourActivity: number[];
}

export interface AgentActivity {
  agentId: string;
  timestamp: string;
  type: 'discovery' | 'status_change' | 'error' | 'message';
  details: any;
}