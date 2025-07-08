import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { AgentCard } from './AgentCard';
import { Agent, AgentMetrics } from '../../types/agent';

const mockAgent: Agent = {
  id: 'agent-123',
  type: 'discovery-analyzer',
  status: 'running',
  createdAt: new Date('2025-01-08T09:00:00Z'),
  lastActivity: new Date('2025-01-08T10:45:00Z'),
  capabilities: ['pattern-detection', 'anomaly-detection'],
};

const mockMetrics: AgentMetrics = {
  agentId: 'agent-123',
  discoveryCount: 42,
  successRate: 0.92,
  avgResponseTime: 145.5,
  errorCount: 3,
  lastHourActivity: [5, 8, 12, 10, 7, 9, 11, 6, 8, 10, 12, 15],
};

describe('AgentCard', () => {
  it('renders agent information correctly', () => {
    render(<AgentCard agent={mockAgent} />);
    
    expect(screen.getByText('agent-123')).toBeInTheDocument();
    expect(screen.getByText('discovery-analyzer')).toBeInTheDocument();
    expect(screen.getByText('running')).toBeInTheDocument();
  });
  
  it('shows correct status indicator color', () => {
    render(<AgentCard agent={mockAgent} />);
    
    const statusIndicator = screen.getByTestId('status-indicator');
    expect(statusIndicator).toHaveClass('status-healthy');
  });
  
  it('displays capabilities', () => {
    render(<AgentCard agent={mockAgent} />);
    
    expect(screen.getByText('pattern-detection')).toBeInTheDocument();
    expect(screen.getByText('anomaly-detection')).toBeInTheDocument();
  });
  
  it('shows metrics when provided', () => {
    render(<AgentCard agent={mockAgent} metrics={mockMetrics} />);
    
    expect(screen.getByText('42')).toBeInTheDocument();
    expect(screen.getByText('92%')).toBeInTheDocument();
    expect(screen.getByText('145.5ms')).toBeInTheDocument();
  });
  
  it('shows loading state for metrics', () => {
    render(<AgentCard agent={mockAgent} metricsLoading={true} />);
    
    expect(screen.getByText('Loading metrics...')).toBeInTheDocument();
  });
  
  it('handles click events', async () => {
    const handleClick = vi.fn();
    const user = userEvent.setup();
    
    render(<AgentCard agent={mockAgent} onClick={handleClick} />);
    
    await user.click(screen.getByRole('article'));
    
    expect(handleClick).toHaveBeenCalledWith(mockAgent);
  });
  
  it('shows different status colors', () => {
    const terminatedAgent = { ...mockAgent, status: 'terminated' as const };
    const { rerender } = render(<AgentCard agent={terminatedAgent} />);
    
    expect(screen.getByTestId('status-indicator')).toHaveClass('status-critical');
    
    const errorAgent = { ...mockAgent, status: 'error' as const };
    rerender(<AgentCard agent={errorAgent} />);
    
    expect(screen.getByTestId('status-indicator')).toHaveClass('status-critical');
  });
  
  it('formats last activity time', () => {
    render(<AgentCard agent={mockAgent} />);
    
    expect(screen.getByText(/Last active:/)).toBeInTheDocument();
    expect(screen.getByText(/10:45:00/)).toBeInTheDocument();
  });
});