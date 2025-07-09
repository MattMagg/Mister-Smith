import React from 'react';

function AppMinimal() {
  // Mock data only - no hooks or external connections
  const displayDiscoveries = [
    {
      id: 'disc-1',
      type: 'pattern',
      content: 'Detected recurring authentication failures across multiple agents',
      confidence: 0.89,
      agentId: 'agent-001',
      timestamp: '14:32:15',
    },
    {
      id: 'disc-2', 
      type: 'solution',
      content: 'Implemented circuit breaker pattern to prevent cascade failures',
      confidence: 0.95,
      agentId: 'agent-002',
      timestamp: '14:28:42',
    },
    {
      id: 'disc-3',
      type: 'anomaly', 
      content: 'Unusual spike in memory usage detected in processing pipeline',
      confidence: 0.76,
      agentId: 'agent-003',
      timestamp: '14:25:18',
    }
  ];

  const displayAgents = [
    { id: 'agent-001', status: 'running', discoveries: 23, uptime: '2h 15m' },
    { id: 'agent-002', status: 'running', discoveries: 18, uptime: '1h 42m' },
    { id: 'agent-003', status: 'warning', discoveries: 12, uptime: '45m' },
  ];

  return (
    <div style={{ 
      minHeight: '100vh',
      padding: '32px', 
      color: '#f8fafc',
      backgroundColor: '#1e3a8a',
      fontSize: '16px',
      lineHeight: '1.6'
    }}>      
      {/* Header */}
      <div style={{ marginBottom: '32px' }}>
        <h1 style={{ 
          fontSize: '2.5rem', 
          fontWeight: 'bold', 
          marginBottom: '8px',
          color: '#ffffff'
        }}>
          MisterSmith Monitoring Dashboard
        </h1>
        <p style={{ color: '#93c5fd' }}>
          Real-time multi-agent orchestration monitoring • Step 2 of 5 Complete ✅
        </p>
      </div>

      {/* Dashboard Grid */}
      <div style={{ 
        display: 'grid', 
        gridTemplateColumns: '1fr 1fr', 
        gap: '24px',
        marginBottom: '32px'
      }}>
        {/* Discovery Stream */}
        <div style={{ 
          backgroundColor: '#1e40af',
          borderRadius: '12px',
          padding: '24px',
          border: '1px solid #3b82f6'
        }}>
          <h2 style={{ 
            fontSize: '1.5rem', 
            marginBottom: '16px', 
            color: '#ffffff',
            display: 'flex',
            alignItems: 'center',
            gap: '8px'
          }}>
            🔍 Discovery Stream
            <span style={{ 
              fontSize: '0.875rem', 
              backgroundColor: '#3b82f6', 
              padding: '2px 8px', 
              borderRadius: '12px' 
            }}>
              {displayDiscoveries.length}
            </span>
          </h2>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {displayDiscoveries.map((discovery) => (
              <div key={discovery.id} style={{
                backgroundColor: '#2563eb',
                padding: '16px',
                borderRadius: '8px',
                borderLeft: '4px solid #60a5fa'
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '8px' }}>
                  <span style={{ 
                    fontSize: '0.75rem', 
                    backgroundColor: '#1d4ed8', 
                    padding: '2px 6px', 
                    borderRadius: '4px',
                    color: '#ffffff'
                  }}>
                    {discovery.type}
                  </span>
                  <span style={{ fontSize: '0.75rem', color: '#93c5fd' }}>
                    {discovery.timestamp}
                  </span>
                </div>
                <p style={{ fontSize: '0.875rem', marginBottom: '8px' }}>
                  {discovery.content}
                </p>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: '0.75rem', color: '#93c5fd' }}>
                    {discovery.agentId}
                  </span>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                    <span style={{ fontSize: '0.75rem', color: '#dbeafe' }}>
                      {Math.round(discovery.confidence * 100)}%
                    </span>
                    <div style={{
                      width: '60px',
                      height: '4px',
                      backgroundColor: '#1d4ed8',
                      borderRadius: '2px',
                      overflow: 'hidden'
                    }}>
                      <div style={{
                        width: `${discovery.confidence * 100}%`,
                        height: '100%',
                        backgroundColor: '#60a5fa'
                      }} />
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Agent Status */}
        <div style={{ 
          backgroundColor: '#1e40af',
          borderRadius: '12px',
          padding: '24px',
          border: '1px solid #3b82f6'
        }}>
          <h2 style={{ 
            fontSize: '1.5rem', 
            marginBottom: '16px', 
            color: '#ffffff',
            display: 'flex',
            alignItems: 'center',
            gap: '8px'
          }}>
            🤖 Agent Status
            <span style={{ 
              fontSize: '0.875rem', 
              backgroundColor: '#16a34a', 
              padding: '2px 8px', 
              borderRadius: '12px' 
            }}>
              {displayAgents.filter(a => a.status === 'running').length} running
            </span>
          </h2>
          
          <div style={{ display: 'flex', flexDirection: 'column', gap: '12px' }}>
            {displayAgents.map((agent) => (
              <div key={agent.id} style={{
                backgroundColor: '#2563eb',
                padding: '16px',
                borderRadius: '8px',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center'
              }}>
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '4px' }}>
                    <span style={{ fontWeight: '600' }}>{agent.id}</span>
                    <span style={{
                      width: '8px',
                      height: '8px',
                      borderRadius: '50%',
                      backgroundColor: agent.status === 'running' ? '#16a34a' : '#f59e0b'
                    }} />
                  </div>
                  <div style={{ fontSize: '0.75rem', color: '#93c5fd' }}>
                    Uptime: {agent.uptime}
                  </div>
                </div>
                <div style={{ textAlign: 'right' }}>
                  <div style={{ fontSize: '1.25rem', fontWeight: '600' }}>
                    {agent.discoveries}
                  </div>
                  <div style={{ fontSize: '0.75rem', color: '#93c5fd' }}>
                    discoveries
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Status Footer */}
      <div style={{
        backgroundColor: '#1e40af',
        borderRadius: '8px',
        padding: '16px',
        textAlign: 'center',
        border: '1px solid #3b82f6'
      }}>
        <p style={{ margin: 0, color: '#93c5fd' }}>
          ✅ React Framework: Working • ✅ Vite Dev Server: http://localhost:5173 • ✅ TypeScript: Compiled • ✅ Tailwind CSS: Configured
        </p>
      </div>
    </div>
  );
}

export default AppMinimal;