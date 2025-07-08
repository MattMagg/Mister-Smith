import React from 'react';
import { Discovery, DISCOVERY_COLORS } from '../../types/discovery';
import { Link2 } from 'lucide-react';
import { format } from 'date-fns';
import { clsx } from 'clsx';

interface DiscoveryCardProps {
  discovery: Discovery;
  onClick?: (discovery: Discovery) => void;
  className?: string;
}

export const DiscoveryCard: React.FC<DiscoveryCardProps> = ({
  discovery,
  onClick,
  className,
}) => {
  const typeColors = {
    pattern: 'bg-blue-500',
    anomaly: 'bg-red-500',
    connection: 'bg-purple-500',
    solution: 'bg-green-500',
    question: 'bg-amber-500',
    insight: 'bg-cyan-500',
  };
  
  const handleClick = () => {
    if (onClick) {
      onClick(discovery);
    }
  };
  
  return (
    <article
      className={clsx(
        'discovery-card cursor-pointer',
        onClick && 'hover:border-gray-600',
        className
      )}
      onClick={handleClick}
      role="article"
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2">
          <span
            className={clsx(
              'px-2 py-1 text-xs font-medium text-white rounded',
              typeColors[discovery.type]
            )}
          >
            {discovery.type}
          </span>
          <span className="text-xs text-gray-400">
            {discovery.agentId}
          </span>
          {discovery.relatedTo && (
            <Link2
              className="w-4 h-4 text-gray-400"
              title={`Related to ${discovery.relatedTo}`}
            />
          )}
        </div>
        <time className="text-xs text-gray-400">
          {format(discovery.timestamp, 'HH:mm:ss')}
        </time>
      </div>
      
      <p className="text-sm text-gray-200 mb-3">
        {discovery.content}
      </p>
      
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs text-gray-400">Confidence:</span>
          <span className="text-xs font-medium text-gray-200">
            {Math.round(discovery.confidence * 100)}%
          </span>
        </div>
        <div className="w-24 h-2 bg-gray-700 rounded-full overflow-hidden">
          <div
            className="h-full bg-gradient-to-r from-gray-500 to-blue-500 transition-all duration-300"
            style={{ width: `${discovery.confidence * 100}%` }}
            data-testid="confidence-bar"
          />
        </div>
      </div>
    </article>
  );
};