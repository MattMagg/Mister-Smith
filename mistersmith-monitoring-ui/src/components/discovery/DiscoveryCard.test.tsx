import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DiscoveryCard } from './DiscoveryCard';
import { Discovery } from '../../types/discovery';

const mockDiscovery: Discovery = {
  id: 'disc-123',
  type: 'pattern',
  content: 'Detected repeating API call pattern in user authentication flow',
  confidence: 0.85,
  agentId: 'agent-456',
  timestamp: new Date('2025-01-08T10:30:00Z'),
  relatedTo: 'disc-122',
};

describe('DiscoveryCard', () => {
  it('renders discovery content correctly', () => {
    render(<DiscoveryCard discovery={mockDiscovery} />);
    
    expect(screen.getByText(mockDiscovery.content)).toBeInTheDocument();
    expect(screen.getByText('pattern')).toBeInTheDocument();
    expect(screen.getByText('85%')).toBeInTheDocument();
    expect(screen.getByText('agent-456')).toBeInTheDocument();
  });
  
  it('applies correct color based on discovery type', () => {
    render(<DiscoveryCard discovery={mockDiscovery} />);
    
    const typeTag = screen.getByText('pattern');
    expect(typeTag).toHaveClass('bg-blue-500');
  });
  
  it('shows related discovery indicator when relatedTo is present', () => {
    render(<DiscoveryCard discovery={mockDiscovery} />);
    
    expect(screen.getByTitle('Related to disc-122')).toBeInTheDocument();
  });
  
  it('does not show related indicator when relatedTo is absent', () => {
    const discoveryWithoutRelation = { ...mockDiscovery, relatedTo: undefined };
    render(<DiscoveryCard discovery={discoveryWithoutRelation} />);
    
    expect(screen.queryByTitle(/Related to/)).not.toBeInTheDocument();
  });
  
  it('formats timestamp correctly', () => {
    render(<DiscoveryCard discovery={mockDiscovery} />);
    
    expect(screen.getByText(/10:30:00/)).toBeInTheDocument();
  });
  
  it('calls onClick handler when clicked', async () => {
    const handleClick = vi.fn();
    const user = userEvent.setup();
    
    render(<DiscoveryCard discovery={mockDiscovery} onClick={handleClick} />);
    
    await user.click(screen.getByRole('article'));
    
    expect(handleClick).toHaveBeenCalledWith(mockDiscovery);
  });
  
  it('shows hover state when interactive', async () => {
    const handleClick = vi.fn();
    const user = userEvent.setup();
    
    render(<DiscoveryCard discovery={mockDiscovery} onClick={handleClick} />);
    
    const card = screen.getByRole('article');
    await user.hover(card);
    
    expect(card).toHaveClass('hover:border-gray-600');
  });
  
  it('displays confidence indicator with correct opacity', () => {
    render(<DiscoveryCard discovery={mockDiscovery} />);
    
    const confidenceBar = screen.getByTestId('confidence-bar');
    expect(confidenceBar).toHaveStyle({ width: '85%' });
  });
});