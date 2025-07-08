import React, { useState, useCallback } from 'react';
import { ChevronDown, ChevronRight, Copy, ArrowRight, ArrowLeft } from 'lucide-react';
import { format } from 'date-fns';
import { clsx } from 'clsx';

interface Message {
  id: string;
  subject: string;
  payload: any;
  timestamp: Date;
  direction: 'inbound' | 'outbound';
  size: number;
}

interface MessageInspectorProps {
  message: Message;
  searchTerm?: string;
  className?: string;
}

export const MessageInspector: React.FC<MessageInspectorProps> = ({
  message,
  searchTerm,
  className,
}) => {
  const [isExpanded, setIsExpanded] = useState(true);
  const [copySuccess, setCopySuccess] = useState(false);
  
  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(JSON.stringify(message.payload, null, 2));
      setCopySuccess(true);
      setTimeout(() => setCopySuccess(false), 2000);
    } catch (error) {
      console.error('Failed to copy:', error);
    }
  }, [message.payload]);
  
  const toggleExpanded = () => {
    setIsExpanded(!isExpanded);
  };
  
  const formatSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} bytes`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };
  
  const highlightSearchTerm = (text: string): React.ReactNode => {
    if (!searchTerm) return text;
    
    const parts = text.split(new RegExp(`(${searchTerm})`, 'gi'));
    return parts.map((part, index) =>
      part.toLowerCase() === searchTerm.toLowerCase() ? (
        <span key={index} className="bg-yellow-500 text-gray-900 px-1 rounded">
          {part}
        </span>
      ) : (
        part
      )
    );
  };
  
  const payloadString = JSON.stringify(message.payload, null, 2);
  
  return (
    <div className={clsx('bg-gray-800 rounded-lg border border-gray-700', className)}>
      <div className="flex items-center justify-between p-4 border-b border-gray-700">
        <div className="flex items-center gap-3">
          <button
            onClick={toggleExpanded}
            className="text-gray-400 hover:text-gray-200 transition-colors"
            title="Toggle payload"
          >
            {isExpanded ? (
              <ChevronDown className="w-4 h-4" />
            ) : (
              <ChevronRight className="w-4 h-4" />
            )}
          </button>
          
          <div className="flex items-center gap-2">
            {message.direction === 'inbound' ? (
              <ArrowRight 
                className="w-4 h-4 text-green-500" 
                title="Inbound message"
              />
            ) : (
              <ArrowLeft 
                className="w-4 h-4 text-blue-500" 
                title="Outbound message"
              />
            )}
            <span className="text-gray-400">
              {message.direction === 'inbound' ? '→' : '←'}
            </span>
          </div>
          
          <code className="text-sm font-mono text-gray-200">
            {highlightSearchTerm(message.subject)}
          </code>
        </div>
        
        <div className="flex items-center gap-4">
          <span className="text-xs text-gray-400">
            {formatSize(message.size)}
          </span>
          <time className="text-xs text-gray-400">
            {format(message.timestamp, 'HH:mm:ss.SSS')}
          </time>
          <button
            onClick={handleCopy}
            className={clsx(
              'p-1 rounded transition-colors',
              copySuccess
                ? 'text-green-500 bg-green-500/10'
                : 'text-gray-400 hover:text-gray-200 hover:bg-gray-700'
            )}
            title="Copy payload"
          >
            <Copy className="w-4 h-4" />
          </button>
        </div>
      </div>
      
      <div
        data-testid="message-payload"
        className={clsx(
          'transition-all duration-200 overflow-hidden',
          isExpanded ? 'max-h-96' : 'max-h-0'
        )}
      >
        <pre className="p-4 text-xs font-mono text-gray-300 overflow-x-auto">
          {searchTerm ? highlightSearchTerm(payloadString) : payloadString}
        </pre>
      </div>
    </div>
  );
};