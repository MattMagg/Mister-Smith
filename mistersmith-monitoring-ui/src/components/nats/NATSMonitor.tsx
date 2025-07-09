import React, { useState, useEffect } from 'react';
import { useSystemMetrics, useSystemHealth } from '../../hooks/useMonitoring';
import { MetricsChart } from '../metrics/MetricsChart';
import { AlertCircle, CheckCircle, Activity, Users, MessageSquare, Zap } from 'lucide-react';
import { clsx } from 'clsx';

interface NATSConnectionMetrics {
  timestamp: Date;
  connections: number;
  throughput: number;
  errors: number;
}

export const NATSMonitor: React.FC = () => {
  const { data: systemMetrics } = useSystemMetrics();
  const { data: systemHealth } = useSystemHealth();
  const [natsHistory, setNatsHistory] = useState<NATSConnectionMetrics[]>([]);
  
  // Update NATS history when new metrics come in
  useEffect(() => {
    if (systemMetrics) {
      const newDataPoint: NATSConnectionMetrics = {
        timestamp: systemMetrics.timestamp,
        connections: systemMetrics.nats_connections,
        throughput: systemMetrics.message_throughput,
        errors: 0 // We'll get this from error tracking
      };
      
      setNatsHistory(prev => [...prev, newDataPoint].slice(-50));
    }
  }, [systemMetrics]);
  
  const getNATSStatus = () => {
    if (!systemHealth) return 'unknown';
    return systemHealth.components.nats_server;
  };
  
  const getStatusColor = (status: string) => {
    switch (status) {
      case 'up': return 'text-green-400';
      case 'down': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };
  
  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'up': return CheckCircle;
      case 'down': return AlertCircle;
      default: return Activity;
    }
  };
  
  const natsStatus = getNATSStatus();
  const StatusIcon = getStatusIcon(natsStatus);
  
  const connectionData = natsHistory.map(point => ({
    timestamp: point.timestamp,
    value: point.connections
  }));
  
  const throughputData = natsHistory.map(point => ({
    timestamp: point.timestamp,
    value: point.throughput
  }));
  
  return (
    <div className="space-y-6">
      {/* NATS Status Header */}
      <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <StatusIcon className={clsx('w-6 h-6', getStatusColor(natsStatus))} />
            <div>
              <h2 className="text-xl font-semibold text-gray-200">NATS Server</h2>
              <p className="text-sm text-gray-400">
                Status: <span className={getStatusColor(natsStatus)}>{natsStatus}</span>
              </p>
            </div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-bold text-gray-200">
              {systemMetrics?.nats_connections || 0}
            </div>
            <div className="text-sm text-gray-400">Active Connections</div>
          </div>
        </div>
      </div>
      
      {/* NATS Metrics Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-3">
            <Users className="w-5 h-5 text-blue-400" />
            <div>
              <div className="text-lg font-semibold text-gray-200">
                {systemMetrics?.nats_connections || 0}
              </div>
              <div className="text-sm text-gray-400">Connections</div>
            </div>
          </div>
        </div>
        
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-3">
            <MessageSquare className="w-5 h-5 text-green-400" />
            <div>
              <div className="text-lg font-semibold text-gray-200">
                {systemMetrics?.message_throughput || 0}
              </div>
              <div className="text-sm text-gray-400">Messages/sec</div>
            </div>
          </div>
        </div>
        
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-3">
            <Zap className="w-5 h-5 text-yellow-400" />
            <div>
              <div className="text-lg font-semibold text-gray-200">
                {systemMetrics?.active_agents || 0}
              </div>
              <div className="text-sm text-gray-400">Active Agents</div>
            </div>
          </div>
        </div>
        
        <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
          <div className="flex items-center gap-3">
            <Activity className="w-5 h-5 text-purple-400" />
            <div>
              <div className="text-lg font-semibold text-gray-200">
                {systemHealth?.uptime ? Math.floor(systemHealth.uptime / 60) : 0}m
              </div>
              <div className="text-sm text-gray-400">Uptime</div>
            </div>
          </div>
        </div>
      </div>
      
      {/* NATS Performance Charts */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <MetricsChart
          data={connectionData}
          type="line"
          title="NATS Connections"
          unit=""
          color="#3B82F6"
          height={250}
        />
        <MetricsChart
          data={throughputData}
          type="area"
          title="Message Throughput"
          unit="/s"
          color="#10B981"
          height={250}
        />
      </div>
      
      {/* Connection Details */}
      <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h3 className="text-lg font-semibold text-gray-200 mb-4">Connection Details</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <h4 className="text-sm font-medium text-gray-300 mb-2">Subjects</h4>
            <div className="space-y-1 text-sm text-gray-400">
              <div>• discoveries.> - Discovery sharing</div>
              <div>• agents.> - Agent communication</div>
              <div>• metrics.> - System metrics</div>
              <div>• logs.> - Log streaming</div>
            </div>
          </div>
          <div>
            <h4 className="text-sm font-medium text-gray-300 mb-2">Connection Info</h4>
            <div className="space-y-1 text-sm text-gray-400">
              <div>Server: localhost:4222</div>
              <div>Version: {systemHealth?.version || 'Unknown'}</div>
              <div>Cluster: Single Node</div>
              <div>TLS: Disabled</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
