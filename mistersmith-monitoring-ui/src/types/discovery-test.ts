// Minimal test version of Discovery types
export type DiscoveryType = 'pattern' | 'anomaly' | 'connection' | 'solution' | 'question' | 'insight';

export interface Discovery {
  id: string;
  type: DiscoveryType;
  content: string;
  confidence: number;
  agentId: string;
  timestamp: Date;
  relatedTo?: string;
  metadata?: Record<string, any>;
}