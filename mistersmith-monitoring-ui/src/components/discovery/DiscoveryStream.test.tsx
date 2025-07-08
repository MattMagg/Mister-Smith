import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DiscoveryStream } from './DiscoveryStream';
import { Discovery } from '../../types/discovery';

const mockDiscoveries: Discovery[] = [
  {
    id: 'disc-1',
    type: 'pattern',
    content: 'Pattern detected in API calls',
    confidence: 0.9,
    agentId: 'agent-1',
    timestamp: new Date('2025-01-08T10:30:00Z'),
  },
  {
    id: 'disc-2',
    type: 'anomaly',
    content: 'Unusual spike in error rates',
    confidence: 0.95,
    agentId: 'agent-2',
    timestamp: new Date('2025-01-08T10:31:00Z'),
  },
  {
    id: 'disc-3',
    type: 'insight',
    content: 'System performance degraded during peak hours',
    confidence: 0.8,
    agentId: 'agent-1',
    timestamp: new Date('2025-01-08T10:32:00Z'),
  },
];

describe('DiscoveryStream', () => {
  it('renders all discoveries', () => {
    render(<DiscoveryStream discoveries={mockDiscoveries} />);
    
    expect(screen.getByText('Pattern detected in API calls')).toBeInTheDocument();
    expect(screen.getByText('Unusual spike in error rates')).toBeInTheDocument();
    expect(screen.getByText('System performance degraded during peak hours')).toBeInTheDocument();
  });
  
  it('shows empty state when no discoveries', () => {
    render(<DiscoveryStream discoveries={[]} />);
    
    expect(screen.getByText('No discoveries yet')).toBeInTheDocument();
    expect(screen.getByText('Discoveries will appear here in real-time')).toBeInTheDocument();
  });
  
  it('shows loading state when isLoading is true', () => {
    render(<DiscoveryStream discoveries={[]} isLoading={true} />);
    
    expect(screen.getByText('Loading discoveries...')).toBeInTheDocument();
  });
  
  it('handles discovery click events', async () => {
    const handleClick = vi.fn();
    const user = userEvent.setup();
    
    render(<DiscoveryStream discoveries={mockDiscoveries} onDiscoveryClick={handleClick} />);
    
    await user.click(screen.getByText('Pattern detected in API calls'));
    
    expect(handleClick).toHaveBeenCalledWith(mockDiscoveries[0]);
  });
  
  it('auto-scrolls to bottom when autoScroll is true', () => {
    const { container } = render(
      <DiscoveryStream discoveries={mockDiscoveries} autoScroll={true} />
    );
    
    const scrollContainer = container.querySelector('[data-testid="stream-container"]');
    expect(scrollContainer).toHaveClass('overflow-y-auto');
  });
  
  it('applies custom className', () => {
    const { container } = render(
      <DiscoveryStream discoveries={mockDiscoveries} className="custom-class" />
    );
    
    expect(container.firstChild).toHaveClass('custom-class');
  });
  
  it('shows discovery count in header', () => {
    render(<DiscoveryStream discoveries={mockDiscoveries} />);
    
    expect(screen.getByText('3 discoveries')).toBeInTheDocument();
  });
  
  it('respects maxItems prop', () => {
    render(<DiscoveryStream discoveries={mockDiscoveries} maxItems={2} />);
    
    expect(screen.getByText('Pattern detected in API calls')).toBeInTheDocument();
    expect(screen.getByText('Unusual spike in error rates')).toBeInTheDocument();
    expect(screen.queryByText('System performance degraded during peak hours')).not.toBeInTheDocument();
  });
});