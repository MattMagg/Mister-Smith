export type AgentStatus = 'created' | 'running' | 'terminated' | 'error';

export interface Agent {
  id: string;
  type: string;
  status: AgentStatus;
  createdAt: Date;
  lastActivity?: Date;
  capabilities: string[];
  metadata?: Record<string, any>;
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
  timestamp: Date;
  type: 'discovery' | 'status_change' | 'error' | 'message';
  details: any;
}