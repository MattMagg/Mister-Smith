import React, { useState, useEffect } from 'react';
import { useErrors, useLogs } from '../../hooks/useMonitoring';
import { ErrorEvent, LogEntry } from '../../lib/monitoring-service';
import { AlertTriangle, XCircle, AlertCircle, Info, ChevronDown, ChevronRight } from 'lucide-react';
import { format } from 'date-fns';
import { clsx } from 'clsx';

interface ErrorTrackerProps {
  maxErrors?: number;
  showStackTraces?: boolean;
}

export const ErrorTracker: React.FC<ErrorTrackerProps> = ({ 
  maxErrors = 50,
  showStackTraces = true 
}) => {
  const { data: errors } = useErrors();
  const { data: errorLogs } = useLogs();
  const [expandedErrors, setExpandedErrors] = useState<Set<string>>(new Set());
  const [filter, setFilter] = useState<'all' | 'critical' | 'high' | 'medium' | 'low'>('all');
  
  // Combine errors from both sources
  const allErrors = React.useMemo(() => {
    const combinedErrors: (ErrorEvent | LogEntry)[] = [];
    
    // Add error events
    if (errors) {
      combinedErrors.push(...errors);
    }
    
    // Add error logs
    if (errorLogs) {
      const errorLogEntries = errorLogs.filter(log => log.level === 'error');
      combinedErrors.push(...errorLogEntries.map(log => ({
        id: log.id,
        message: log.message,
        source: log.source,
        timestamp: log.timestamp,
        severity: 'medium' as const,
        stack: log.metadata?.stack
      })));
    }
    
    return combinedErrors
      .sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime())
      .slice(0, maxErrors);
  }, [errors, errorLogs, maxErrors]);
  
  const filteredErrors = allErrors.filter(error => {
    if (filter === 'all') return true;
    return 'severity' in error && error.severity === filter;
  });
  
  const toggleErrorExpansion = (errorId: string) => {
    setExpandedErrors(prev => {
      const newSet = new Set(prev);
      if (newSet.has(errorId)) {
        newSet.delete(errorId);
      } else {
        newSet.add(errorId);
      }
      return newSet;
    });
  };
  
  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return 'text-red-400 bg-red-500/10 border-red-500';
      case 'high': return 'text-orange-400 bg-orange-500/10 border-orange-500';
      case 'medium': return 'text-yellow-400 bg-yellow-500/10 border-yellow-500';
      case 'low': return 'text-blue-400 bg-blue-500/10 border-blue-500';
      default: return 'text-gray-400 bg-gray-500/10 border-gray-500';
    }
  };
  
  const getSeverityIcon = (severity: string) => {
    switch (severity) {
      case 'critical': return XCircle;
      case 'high': return AlertTriangle;
      case 'medium': return AlertCircle;
      case 'low': return Info;
      default: return AlertCircle;
    }
  };
  
  const getErrorCounts = () => {
    const counts = { critical: 0, high: 0, medium: 0, low: 0 };
    allErrors.forEach(error => {
      if ('severity' in error && error.severity) {
        counts[error.severity]++;
      }
    });
    return counts;
  };
  
  const errorCounts = getErrorCounts();
  
  return (
    <div className="space-y-6">
      {/* Error Summary */}
      <div className="bg-gray-800 rounded-lg p-6 border border-gray-700">
        <h2 className="text-xl font-semibold text-gray-200 mb-4">Error Tracking</h2>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-red-500/10 rounded-lg p-3 border border-red-500/20">
            <div className="text-lg font-bold text-red-400">{errorCounts.critical}</div>
            <div className="text-sm text-red-300">Critical</div>
          </div>
          <div className="bg-orange-500/10 rounded-lg p-3 border border-orange-500/20">
            <div className="text-lg font-bold text-orange-400">{errorCounts.high}</div>
            <div className="text-sm text-orange-300">High</div>
          </div>
          <div className="bg-yellow-500/10 rounded-lg p-3 border border-yellow-500/20">
            <div className="text-lg font-bold text-yellow-400">{errorCounts.medium}</div>
            <div className="text-sm text-yellow-300">Medium</div>
          </div>
          <div className="bg-blue-500/10 rounded-lg p-3 border border-blue-500/20">
            <div className="text-lg font-bold text-blue-400">{errorCounts.low}</div>
            <div className="text-sm text-blue-300">Low</div>
          </div>
        </div>
      </div>
      
      {/* Filter Controls */}
      <div className="bg-gray-800 rounded-lg p-4 border border-gray-700">
        <div className="flex flex-wrap gap-2">
          {['all', 'critical', 'high', 'medium', 'low'].map(severity => (
            <button
              key={severity}
              onClick={() => setFilter(severity as any)}
              className={clsx(
                'px-3 py-1 rounded-md text-sm font-medium transition-colors',
                filter === severity
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              )}
            >
              {severity.charAt(0).toUpperCase() + severity.slice(1)}
              {severity !== 'all' && (
                <span className="ml-1 text-xs opacity-75">
                  ({errorCounts[severity as keyof typeof errorCounts]})
                </span>
              )}
            </button>
          ))}
        </div>
      </div>
      
      {/* Error List */}
      <div className="space-y-3">
        {filteredErrors.length === 0 ? (
          <div className="bg-gray-800 rounded-lg p-8 border border-gray-700 text-center">
            <AlertCircle className="w-8 h-8 text-gray-400 mx-auto mb-2" />
            <p className="text-gray-400">
              {filter === 'all' ? 'No errors found' : `No ${filter} severity errors found`}
            </p>
          </div>
        ) : (
          filteredErrors.map(error => {
            const isExpanded = expandedErrors.has(error.id);
            const severity = 'severity' in error ? error.severity : 'medium';
            const SeverityIcon = getSeverityIcon(severity);
            
            return (
              <div
                key={error.id}
                className={clsx(
                  'bg-gray-800 rounded-lg border',
                  getSeverityColor(severity)
                )}
              >
                <div
                  className="p-4 cursor-pointer"
                  onClick={() => toggleErrorExpansion(error.id)}
                >
                  <div className="flex items-start gap-3">
                    <SeverityIcon className="w-5 h-5 mt-0.5 flex-shrink-0" />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-medium text-gray-200">
                          {error.source}
                        </span>
                        <span className="text-xs text-gray-400">
                          {format(error.timestamp, 'HH:mm:ss.SSS')}
                        </span>
                      </div>
                      <p className="text-sm text-gray-300 truncate">
                        {error.message}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      {'severity' in error && (
                        <span className="text-xs px-2 py-1 rounded-full bg-gray-700 text-gray-300">
                          {error.severity}
                        </span>
                      )}
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4 text-gray-400" />
                      ) : (
                        <ChevronRight className="w-4 h-4 text-gray-400" />
                      )}
                    </div>
                  </div>
                </div>
                
                {isExpanded && (
                  <div className="px-4 pb-4 border-t border-gray-700">
                    <div className="mt-3 space-y-2">
                      <div>
                        <span className="text-xs font-medium text-gray-400">Message:</span>
                        <p className="text-sm text-gray-300 mt-1">{error.message}</p>
                      </div>
                      
                      <div>
                        <span className="text-xs font-medium text-gray-400">Source:</span>
                        <p className="text-sm text-gray-300 mt-1">{error.source}</p>
                      </div>
                      
                      <div>
                        <span className="text-xs font-medium text-gray-400">Timestamp:</span>
                        <p className="text-sm text-gray-300 mt-1">
                          {format(error.timestamp, 'yyyy-MM-dd HH:mm:ss.SSS')}
                        </p>
                      </div>
                      
                      {showStackTraces && error.stack && (
                        <div>
                          <span className="text-xs font-medium text-gray-400">Stack Trace:</span>
                          <pre className="text-xs text-gray-300 mt-1 p-2 bg-gray-900 rounded overflow-x-auto">
                            {error.stack}
                          </pre>
                        </div>
                      )}
                    </div>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
    </div>
  );
};
