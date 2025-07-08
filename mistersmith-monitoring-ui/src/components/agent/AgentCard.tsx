import React from 'react';
import { Agent, AgentMetrics } from '../../types/agent';
import { Activity, AlertCircle, CheckCircle, XCircle } from 'lucide-react';
import { format } from 'date-fns';
import { clsx } from 'clsx';

interface AgentCardProps {
  agent: Agent;
  metrics?: AgentMetrics;
  metricsLoading?: boolean;
  onClick?: (agent: Agent) => void;
  className?: string;
}

export const AgentCard: React.FC<AgentCardProps> = ({
  agent,
  metrics,
  metricsLoading = false,
  onClick,
  className,
}) => {
  const statusConfig = {
    created: { color: 'status-degraded', icon: AlertCircle, text: 'Created' },
    running: { color: 'status-healthy', icon: CheckCircle, text: 'Running' },
    terminated: { color: 'status-critical', icon: XCircle, text: 'Terminated' },
    error: { color: 'status-critical', icon: AlertCircle, text: 'Error' },
  };
  
  const config = statusConfig[agent.status];
  const StatusIcon = config.icon;
  
  const handleClick = () => {
    if (onClick) {
      onClick(agent);
    }
  };
  
  return (
    <article
      className={clsx(
        'metric-card cursor-pointer hover:border-gray-600 transition-colors',
        className
      )}
      onClick={handleClick}
      role="article"
    >
      <div className="flex items-start justify-between mb-4">
        <div>
          <h3 className="text-lg font-medium text-gray-200">{agent.id}</h3>
          <p className="text-sm text-gray-400">{agent.type}</p>
        </div>
        <div className="flex items-center gap-2">
          <span
            className={clsx('status-indicator', config.color)}
            data-testid="status-indicator"
          />
          <span className="text-sm text-gray-300">{agent.status}</span>
        </div>
      </div>
      
      {agent.capabilities.length > 0 && (
        <div className="mb-4">
          <div className="flex flex-wrap gap-2">
            {agent.capabilities.map((cap) => (
              <span
                key={cap}
                className="px-2 py-1 text-xs bg-gray-700 text-gray-300 rounded"
              >
                {cap}
              </span>
            ))}
          </div>
        </div>
      )}
      
      {metricsLoading ? (
        <div className="text-center py-4">
          <Activity className="w-5 h-5 text-gray-400 animate-pulse mx-auto" />
          <p className="text-sm text-gray-400 mt-1">Loading metrics...</p>
        </div>
      ) : metrics ? (
        <div className="grid grid-cols-3 gap-4 pt-4 border-t border-gray-700">
          <div>
            <p className="text-xs text-gray-400">Discoveries</p>
            <p className="text-lg font-semibold text-gray-200">
              {metrics.discoveryCount}
            </p>
          </div>
          <div>
            <p className="text-xs text-gray-400">Success Rate</p>
            <p className="text-lg font-semibold text-gray-200">
              {Math.round(metrics.successRate * 100)}%
            </p>
          </div>
          <div>
            <p className="text-xs text-gray-400">Avg Response</p>
            <p className="text-lg font-semibold text-gray-200">
              {metrics.avgResponseTime.toFixed(1)}ms
            </p>
          </div>
        </div>
      ) : null}
      
      {agent.lastActivity && (
        <div className="mt-4 pt-4 border-t border-gray-700">
          <p className="text-xs text-gray-400">
            Last active: {format(agent.lastActivity, 'HH:mm:ss')}
          </p>
        </div>
      )}
    </article>
  );
};