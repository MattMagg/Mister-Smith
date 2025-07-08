import React, { useEffect, useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Discovery } from './types/discovery';
import { DiscoveryCard } from './components/discovery/DiscoveryCard';
import { DiscoveryStream } from './components/discovery/DiscoveryStream';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 10 * 1000,
      gcTime: 5 * 60 * 1000,
    },
  },
});

function AppContent() {
  // Mock discovery data to test components
  const mockDiscoveries: Discovery[] = [
    {
      id: '1',
      type: 'pattern',
      content: 'Detected recurring error pattern in user authentication flow',
      confidence: 0.89,
      agentId: 'agent-001',
      timestamp: new Date(),
    },
    {
      id: '2', 
      type: 'solution',
      content: 'Implemented circuit breaker pattern to prevent cascade failures',
      confidence: 0.95,
      agentId: 'agent-002',
      timestamp: new Date(),
    }
  ];

  return (
    <div style={{ 
      minHeight: '100vh', 
      backgroundColor: '#1a1a1a', 
      color: 'white', 
      padding: '24px' 
    }}>
      <h1 style={{ fontSize: '2rem', fontWeight: 'bold', marginBottom: '24px' }}>
        MisterSmith Monitoring UI - Testing Components
      </h1>
      
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '24px' }}>
        <div>
          <h2 style={{ fontSize: '1.25rem', marginBottom: '16px' }}>Discovery Card Test</h2>
          <DiscoveryCard discovery={mockDiscoveries[0]} />
        </div>
        
        <div>
          <h2 style={{ fontSize: '1.25rem', marginBottom: '16px' }}>Discovery Stream Test</h2>
          <div style={{ height: '384px', border: '1px solid #374151', borderRadius: '4px' }}>
            <DiscoveryStream discoveries={mockDiscoveries} />
          </div>
        </div>
      </div>
    </div>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
    </QueryClientProvider>
  );
}

export default App;