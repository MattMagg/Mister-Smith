import React, { useEffect, useRef } from 'react';
import { Discovery } from '../../types/discovery';
import { DiscoveryCard } from './DiscoveryCard';
import { Activity } from 'lucide-react';
import { clsx } from 'clsx';

interface DiscoveryStreamProps {
  discoveries: Discovery[];
  onDiscoveryClick?: (discovery: Discovery) => void;
  autoScroll?: boolean;
  maxItems?: number;
  isLoading?: boolean;
  className?: string;
}

export const DiscoveryStream: React.FC<DiscoveryStreamProps> = ({
  discoveries,
  onDiscoveryClick,
  autoScroll = false,
  maxItems,
  isLoading = false,
  className,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [discoveries, autoScroll]);
  
  const displayedDiscoveries = maxItems 
    ? discoveries.slice(0, maxItems)
    : discoveries;
  
  if (isLoading) {
    return (
      <div className={clsx('flex items-center justify-center h-64', className)}>
        <div className="text-center">
          <Activity className="w-8 h-8 text-gray-400 animate-pulse mx-auto mb-2" />
          <p className="text-gray-400">Loading discoveries...</p>
        </div>
      </div>
    );
  }
  
  if (discoveries.length === 0) {
    return (
      <div className={clsx('flex items-center justify-center h-64', className)}>
        <div className="text-center">
          <Activity className="w-8 h-8 text-gray-600 mx-auto mb-2" />
          <p className="text-gray-400 font-medium">No discoveries yet</p>
          <p className="text-gray-500 text-sm mt-1">
            Discoveries will appear here in real-time
          </p>
        </div>
      </div>
    );
  }
  
  return (
    <div className={clsx('flex flex-col h-full', className)}>
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <h3 className="text-lg font-medium text-gray-200">Discovery Stream</h3>
        <span className="text-sm text-gray-400">
          {discoveries.length} discoveries
        </span>
      </div>
      
      <div
        ref={containerRef}
        data-testid="stream-container"
        className="flex-1 overflow-y-auto p-4 space-y-3"
      >
        {displayedDiscoveries.map((discovery) => (
          <DiscoveryCard
            key={discovery.id}
            discovery={discovery}
            onClick={onDiscoveryClick}
          />
        ))}
      </div>
    </div>
  );
};