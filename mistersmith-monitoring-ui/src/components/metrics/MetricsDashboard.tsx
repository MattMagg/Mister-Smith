import React, { useState, useEffect } from 'react';
import { MetricsChart } from './MetricsChart';
import { useSystemMetrics } from '../../hooks/useMonitoring';
import { SystemMetrics, monitoringService } from '../../lib/monitoring-service';
import { usePerformanceMetrics } from '../../hooks/usePerformanceMetrics';

interface MetricDataPoint {
  timestamp: Date;
  value: number;
  label?: string;
}

interface MetricsHistory {
  cpu: MetricDataPoint[];
  memory: MetricDataPoint[];
  agents: MetricDataPoint[];
  discoveries: MetricDataPoint[];
  nats: MetricDataPoint[];
  throughput: MetricDataPoint[];
  // Claude Code metrics
  claudeSessions: MetricDataPoint[];
  claudeCost: MetricDataPoint[];
  claudeTokens: MetricDataPoint[];
  claudeErrors: MetricDataPoint[];
}

interface ClaudeCodeMetrics {
  sessions: number;
  totalCost: number;
  tokensUsed: number;
  errorRate: number;
  services: string[];
}

export const MetricsDashboard: React.FC = () => {
  usePerformanceMetrics('MetricsDashboard');
  const { data: systemMetrics, isLoading, error } = useSystemMetrics();
  const [metricsHistory, setMetricsHistory] = useState<MetricsHistory>({
    cpu: [],
    memory: [],
    agents: [],
    discoveries: [],
    nats: [],
    throughput: [],
    claudeSessions: [],
    claudeCost: [],
    claudeTokens: [],
    claudeErrors: []
  });
  
  const [claudeMetrics, setClaudeMetrics] = useState<ClaudeCodeMetrics>({
    sessions: 0,
    totalCost: 0,
    tokensUsed: 0,
    errorRate: 0,
    services: []
  });

  // Update metrics history when new data comes in
  useEffect(() => {
    if (systemMetrics) {
      const newDataPoint = {
        timestamp: systemMetrics.timestamp,
      };

      setMetricsHistory(prev => {
        const maxPoints = 50; // Keep last 50 data points
        
        return {
          ...prev,
          cpu: [...(prev.cpu || []), {
            ...newDataPoint,
            value: systemMetrics.cpu_usage
          }].slice(-maxPoints),
          memory: [...(prev.memory || []), {
            ...newDataPoint,
            value: systemMetrics.memory_usage
          }].slice(-maxPoints),
          agents: [...(prev.agents || []), {
            ...newDataPoint,
            value: systemMetrics.active_agents
          }].slice(-maxPoints),
          discoveries: [...(prev.discoveries || []), {
            ...newDataPoint,
            value: systemMetrics.total_discoveries
          }].slice(-maxPoints),
          nats: [...(prev.nats || []), {
            ...newDataPoint,
            value: systemMetrics.nats_connections
          }].slice(-maxPoints),
          throughput: [...(prev.throughput || []), {
            ...newDataPoint,
            value: systemMetrics.message_throughput
          }].slice(-maxPoints)
        };
      });
    }
  }, [systemMetrics]);
  
  // Fetch Claude Code metrics from Jaeger
  useEffect(() => {
    const fetchClaudeMetrics = async () => {
      try {
        // Get services from Jaeger
        const services = await monitoringService.getJaegerServices();
        const claudeServices = services.filter(s => s.includes('claude-code'));
        
        // For demo purposes, using mock data
        // In production, this would aggregate real metrics from Jaeger
        setClaudeMetrics({
          sessions: Math.floor(Math.random() * 10) + 5,
          totalCost: Math.random() * 100,
          tokensUsed: Math.floor(Math.random() * 10000) + 5000,
          errorRate: Math.random() * 5,
          services: claudeServices
        });
        
        // Update history with new Claude metrics
        const timestamp = new Date();
        setMetricsHistory(prev => ({
          ...prev,
          claudeSessions: [...(prev.claudeSessions || []), {
            timestamp,
            value: Math.floor(Math.random() * 10) + 5
          }].slice(-50),
          claudeCost: [...(prev.claudeCost || []), {
            timestamp,
            value: Math.random() * 10
          }].slice(-50),
          claudeTokens: [...(prev.claudeTokens || []), {
            timestamp,
            value: Math.floor(Math.random() * 1000) + 500
          }].slice(-50),
          claudeErrors: [...(prev.claudeErrors || []), {
            timestamp,
            value: Math.random() * 5
          }].slice(-50)
        }));
      } catch (error) {
        console.error('Failed to fetch Claude metrics:', error);
      }
    };
    
    // Fetch initially and then every 30 seconds
    fetchClaudeMetrics();
    const interval = setInterval(fetchClaudeMetrics, 30000);
    
    return () => clearInterval(interval);
  }, []);

  if (isLoading) {
    return (
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-4">
        {[...Array(6)].map((_, i) => (
          <div key={i} className="bg-gray-800 rounded-lg p-4 border border-gray-700 animate-pulse">
            <div className="h-4 bg-gray-700 rounded mb-4"></div>
            <div className="h-32 bg-gray-700 rounded"></div>
          </div>
        ))}
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex items-center justify-center h-64 bg-gray-800 rounded-lg border border-gray-700">
        <div className="text-center">
          <p className="text-red-400 mb-2">Failed to load metrics</p>
          <p className="text-gray-400 text-sm">{error.message}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Current Values */}
      <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-green-400">
            {systemMetrics?.cpu_usage.toFixed(1)}%
          </div>
          <div className="text-sm text-gray-400">CPU Usage</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-blue-400">
            {systemMetrics?.memory_usage.toFixed(1)}%
          </div>
          <div className="text-sm text-gray-400">Memory Usage</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-purple-400">
            {systemMetrics?.active_agents}
          </div>
          <div className="text-sm text-gray-400">Active Agents</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-yellow-400">
            {systemMetrics?.total_discoveries}
          </div>
          <div className="text-sm text-gray-400">Total Discoveries</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-orange-400">
            {systemMetrics?.nats_connections}
          </div>
          <div className="text-sm text-gray-400">NATS Connections</div>
        </div>
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="text-2xl font-bold text-teal-400">
            {systemMetrics?.message_throughput}
          </div>
          <div className="text-sm text-gray-400">Messages/sec</div>
        </div>
      </div>
      
      {/* Claude Code Metrics */}
      <div className="mt-6">
        <h3 className="text-xl font-semibold text-gray-200 mb-4">Claude Code Metrics</h3>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <div className="text-2xl font-bold text-cyan-400">
              {claudeMetrics.sessions}
            </div>
            <div className="text-sm text-gray-400">Active Sessions</div>
          </div>
          <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <div className="text-2xl font-bold text-pink-400">
              ${claudeMetrics.totalCost.toFixed(2)}
            </div>
            <div className="text-sm text-gray-400">Total Cost</div>
          </div>
          <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <div className="text-2xl font-bold text-indigo-400">
              {claudeMetrics.tokensUsed.toLocaleString()}
            </div>
            <div className="text-sm text-gray-400">Tokens Used</div>
          </div>
          <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
            <div className="text-2xl font-bold text-red-400">
              {claudeMetrics.errorRate.toFixed(1)}%
            </div>
            <div className="text-sm text-gray-400">Error Rate</div>
          </div>
        </div>
      </div>

      {/* Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <MetricsChart
          data={metricsHistory.cpu}
          type="area"
          title="CPU Usage"
          unit="%"
          color="#10B981"
          height={250}
        />
        <MetricsChart
          data={metricsHistory.memory}
          type="area"
          title="Memory Usage"
          unit="%"
          color="#3B82F6"
          height={250}
        />
        <MetricsChart
          data={metricsHistory.agents}
          type="line"
          title="Active Agents"
          unit=""
          color="#8B5CF6"
          height={250}
        />
        <MetricsChart
          data={metricsHistory.discoveries}
          type="line"
          title="Total Discoveries"
          unit=""
          color="#F59E0B"
          height={250}
        />
        <MetricsChart
          data={metricsHistory.nats}
          type="bar"
          title="NATS Connections"
          unit=""
          color="#F97316"
          height={250}
        />
        <MetricsChart
          data={metricsHistory.throughput}
          type="area"
          title="Message Throughput"
          unit="/s"
          color="#14B8A6"
          height={250}
        />
      </div>
      
      {/* Claude Code Charts */}
      <div className="mt-6">
        <h3 className="text-xl font-semibold text-gray-200 mb-4">Claude Code Performance</h3>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <MetricsChart
            data={metricsHistory.claudeSessions}
            type="line"
            title="Claude Sessions"
            unit=""
            color="#06B6D4"
            height={250}
          />
          <MetricsChart
            data={metricsHistory.claudeCost}
            type="area"
            title="Cost Over Time"
            unit="$"
            color="#EC4899"
            height={250}
          />
          <MetricsChart
            data={metricsHistory.claudeTokens}
            type="bar"
            title="Token Usage"
            unit=""
            color="#6366F1"
            height={250}
          />
          <MetricsChart
            data={metricsHistory.claudeErrors}
            type="line"
            title="Error Rate"
            unit="%"
            color="#EF4444"
            height={250}
          />
        </div>
      </div>
    </div>
  );
};
