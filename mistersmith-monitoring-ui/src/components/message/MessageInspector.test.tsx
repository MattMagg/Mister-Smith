import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MessageInspector } from './MessageInspector';

const mockMessage = {
  id: 'msg-123',
  subject: 'discoveries.agent-1.pattern',
  payload: {
    discoveryId: 'disc-456',
    type: 'pattern',
    content: 'Detected pattern in API calls',
    confidence: 0.85,
  },
  timestamp: new Date('2025-01-08T10:30:00Z'),
  direction: 'inbound' as const,
  size: 256,
};

describe('MessageInspector', () => {
  it('renders message subject and timestamp', () => {
    render(<MessageInspector message={mockMessage} />);
    
    expect(screen.getByText('discoveries.agent-1.pattern')).toBeInTheDocument();
    expect(screen.getByText(/10:30:00/)).toBeInTheDocument();
  });
  
  it('shows message direction indicator', () => {
    render(<MessageInspector message={mockMessage} />);
    
    expect(screen.getByText('→')).toBeInTheDocument();
    expect(screen.getByTitle('Inbound message')).toBeInTheDocument();
  });
  
  it('displays formatted JSON payload', () => {
    render(<MessageInspector message={mockMessage} />);
    
    expect(screen.getByText(/"discoveryId":/)).toBeInTheDocument();
    expect(screen.getByText(/"type": "pattern"/)).toBeInTheDocument();
    expect(screen.getByText(/"confidence": 0.85/)).toBeInTheDocument();
  });
  
  it('shows message size', () => {
    render(<MessageInspector message={mockMessage} />);
    
    expect(screen.getByText('256 bytes')).toBeInTheDocument();
  });
  
  it('handles copy to clipboard', async () => {
    const mockClipboard = {
      writeText: vi.fn().mockResolvedValue(undefined),
    };
    Object.assign(navigator, { clipboard: mockClipboard });
    
    const user = userEvent.setup();
    render(<MessageInspector message={mockMessage} />);
    
    await user.click(screen.getByTitle('Copy payload'));
    
    expect(mockClipboard.writeText).toHaveBeenCalledWith(
      JSON.stringify(mockMessage.payload, null, 2)
    );
  });
  
  it('shows outbound message differently', () => {
    const outboundMessage = { ...mockMessage, direction: 'outbound' as const };
    render(<MessageInspector message={outboundMessage} />);
    
    expect(screen.getByText('←')).toBeInTheDocument();
    expect(screen.getByTitle('Outbound message')).toBeInTheDocument();
  });
  
  it('handles collapsed/expanded state', async () => {
    const user = userEvent.setup();
    render(<MessageInspector message={mockMessage} />);
    
    const toggleButton = screen.getByTitle('Toggle payload');
    const payload = screen.getByTestId('message-payload');
    
    expect(payload).toBeVisible();
    
    await user.click(toggleButton);
    expect(payload).not.toBeVisible();
    
    await user.click(toggleButton);
    expect(payload).toBeVisible();
  });
  
  it('highlights search terms if provided', () => {
    render(<MessageInspector message={mockMessage} searchTerm="pattern" />);
    
    const highlighted = screen.getAllByText('pattern');
    expect(highlighted[0]).toHaveClass('bg-yellow-500');
  });
});