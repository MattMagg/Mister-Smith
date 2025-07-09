export type DiscoveryType = 'pattern' | 'anomaly' | 'connection' | 'solution' | 'question' | 'insight';

export interface Discovery {
  id: string;
  type: DiscoveryType;
  content: string;
  confidence: number;
  agentId: string;
  timestamp: string;
  relatedTo?: string;
  metadata?: Record<string, any>;
}

export interface DiscoveryFilter {
  types?: DiscoveryType[];
  agents?: string[];
  confidenceMin?: number;
  confidenceMax?: number;
  timeRange?: {
    start: string;
    end: string;
  };
  keywords?: string[];
  relatedTo?: string;
}

export interface DiscoveryStats {
  total: number;
  byType: Record<DiscoveryType, number>;
  byAgent: Record<string, number>;
  averageConfidence: number;
}

export const DISCOVERY_COLORS: Record<DiscoveryType, string> = {
  pattern: '#3B82F6',      // blue
  anomaly: '#EF4444',      // red
  connection: '#8B5CF6',   // purple
  solution: '#10B981',     // green
  question: '#F59E0B',     // amber
  insight: '#06B6D4',      // cyan
};