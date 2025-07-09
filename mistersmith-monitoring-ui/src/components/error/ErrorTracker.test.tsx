import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ErrorTracker } from './ErrorTracker';
import { useErrors, useLogs } from '../../hooks/useMonitoring';
import { ErrorEvent, LogEntry } from '../../lib/monitoring-service';

// Mock the hooks
vi.mock('../../hooks/useMonitoring', () => ({
  useErrors: vi.fn(),
  useLogs: vi.fn(),
}));

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  AlertTriangle: () => <div data-testid="alert-triangle" />,
  XCircle: () => <div data-testid="x-circle" />,
  AlertCircle: () => <div data-testid="alert-circle" />,
  Info: () => <div data-testid="info" />,
  ChevronDown: () => <div data-testid="chevron-down" />,
  ChevronRight: () => <div data-testid="chevron-right" />,
}));

// Mock date-fns
vi.mock('date-fns', () => ({
  format: vi.fn((date, formatStr) => {
    if (formatStr === 'HH:mm:ss.SSS') {
      return '10:30:45.123';
    }
    if (formatStr === 'yyyy-MM-dd HH:mm:ss.SSS') {
      return '2025-01-08 10:30:45.123';
    }
    return date.toISOString();
  }),
}));

describe('ErrorTracker', () => {
  const mockErrors: ErrorEvent[] = [
    {
      id: 'error-1',
      message: 'Critical system error',
      source: 'system-component',
      timestamp: new Date('2025-01-08T10:30:45.123Z'),
      severity: 'critical',
      stack: 'Error: Critical system error\n    at system-component:123',
    },
    {
      id: 'error-2',
      message: 'High priority warning',
      source: 'app-component',
      timestamp: new Date('2025-01-08T10:29:30.456Z'),
      severity: 'high',
      stack: 'Error: High priority warning\n    at app-component:456',
    },
  ];

  const mockLogs: LogEntry[] = [
    {
      id: 'log-1',
      message: 'Error log message',
      level: 'error',
      source: 'log-component',
      timestamp: new Date('2025-01-08T10:28:15.789Z'),
      metadata: {
        stack: 'Error: Error log message\n    at log-component:789',
      },
    },
    {
      id: 'log-2',
      message: 'Info log message',
      level: 'info',
      source: 'log-component',
      timestamp: new Date('2025-01-08T10:27:00.000Z'),
      metadata: {},
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders error summary with correct counts', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    expect(screen.getByText('Error Tracking')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument(); // Critical count
    expect(screen.getByText('1')).toBeInTheDocument(); // High count
    expect(screen.getByText('1')).toBeInTheDocument(); // Medium count (from log)
    expect(screen.getByText('0')).toBeInTheDocument(); // Low count
  });

  it('renders all filter buttons', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    expect(screen.getByText('All')).toBeInTheDocument();
    expect(screen.getByText('Critical')).toBeInTheDocument();
    expect(screen.getByText('High')).toBeInTheDocument();
    expect(screen.getByText('Medium')).toBeInTheDocument();
    expect(screen.getByText('Low')).toBeInTheDocument();
  });

  it('filters errors by severity', async () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    const user = userEvent.setup();
    render(<ErrorTracker />);

    // Click on Critical filter
    await user.click(screen.getByText('Critical'));

    // Should only show critical errors
    expect(screen.getByText('Critical system error')).toBeInTheDocument();
    expect(screen.queryByText('High priority warning')).not.toBeInTheDocument();
  });

  it('displays error details correctly', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    // Check error details
    expect(screen.getByText('Critical system error')).toBeInTheDocument();
    expect(screen.getByText('system-component')).toBeInTheDocument();
    expect(screen.getByText('10:30:45.123')).toBeInTheDocument();
    expect(screen.getByText('critical')).toBeInTheDocument();
  });

  it('expands error details when clicked', async () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    const user = userEvent.setup();
    render(<ErrorTracker />);

    // Initially should show chevron right
    expect(screen.getByTestId('chevron-right')).toBeInTheDocument();

    // Click on first error
    const firstError = screen.getByText('Critical system error').closest('div');
    if (firstError) {
      await user.click(firstError);
    }

    // Should now show chevron down and expanded details
    await waitFor(() => {
      expect(screen.getByTestId('chevron-down')).toBeInTheDocument();
      expect(screen.getByText('2025-01-08 10:30:45.123')).toBeInTheDocument();
    });
  });

  it('displays stack traces when expanded', async () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    const user = userEvent.setup();
    render(<ErrorTracker />);

    // Click on first error to expand
    const firstError = screen.getByText('Critical system error').closest('div');
    if (firstError) {
      await user.click(firstError);
    }

    // Should show stack trace
    await waitFor(() => {
      expect(screen.getByText('Stack Trace:')).toBeInTheDocument();
      expect(screen.getByText('Error: Critical system error\n    at system-component:123')).toBeInTheDocument();
    });
  });

  it('hides stack traces when showStackTraces is false', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker showStackTraces={false} />);

    // Stack trace should not be shown even if present
    expect(screen.queryByText('Stack Trace:')).not.toBeInTheDocument();
  });

  it('combines errors from both sources', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    // Should show errors from both sources
    expect(screen.getByText('Critical system error')).toBeInTheDocument(); // From errors
    expect(screen.getByText('High priority warning')).toBeInTheDocument(); // From errors
    expect(screen.getByText('Error log message')).toBeInTheDocument(); // From logs
  });

  it('only includes error-level logs', () => {
    (useErrors as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    // Should show error log but not info log
    expect(screen.getByText('Error log message')).toBeInTheDocument();
    expect(screen.queryByText('Info log message')).not.toBeInTheDocument();
  });

  it('respects maxErrors prop', () => {
    const manyErrors = Array.from({ length: 10 }, (_, i) => ({
      id: `error-${i}`,
      message: `Error ${i}`,
      source: 'test-component',
      timestamp: new Date(),
      severity: 'medium' as const,
      stack: `Error ${i} stack`,
    }));

    (useErrors as any).mockReturnValue({
      data: manyErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker maxErrors={5} />);

    // Should only show first 5 errors
    expect(screen.getByText('Error 0')).toBeInTheDocument();
    expect(screen.getByText('Error 4')).toBeInTheDocument();
    expect(screen.queryByText('Error 5')).not.toBeInTheDocument();
  });

  it('sorts errors by timestamp (newest first)', () => {
    const unsortedErrors = [
      {
        id: 'error-old',
        message: 'Old error',
        source: 'test',
        timestamp: new Date('2025-01-08T10:00:00Z'),
        severity: 'medium' as const,
        stack: 'Old error stack',
      },
      {
        id: 'error-new',
        message: 'New error',
        source: 'test',
        timestamp: new Date('2025-01-08T10:30:00Z'),
        severity: 'medium' as const,
        stack: 'New error stack',
      },
    ];

    (useErrors as any).mockReturnValue({
      data: unsortedErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    const errorElements = screen.getAllByText(/error/i);
    // New error should appear first
    expect(errorElements[0]).toHaveTextContent('New error');
  });

  it('displays correct severity icons', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    // Should show different icons for different severities
    expect(screen.getByTestId('x-circle')).toBeInTheDocument(); // Critical
    expect(screen.getByTestId('alert-triangle')).toBeInTheDocument(); // High
    expect(screen.getByTestId('alert-circle')).toBeInTheDocument(); // Medium (from log)
  });

  it('shows no errors message when filter has no results', async () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    const user = userEvent.setup();
    render(<ErrorTracker />);

    // Filter by low severity (which has no errors)
    await user.click(screen.getByText('Low'));

    expect(screen.getByText('No low severity errors found')).toBeInTheDocument();
  });

  it('shows no errors message when no errors exist', () => {
    (useErrors as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    expect(screen.getByText('No errors found')).toBeInTheDocument();
  });

  it('handles null/undefined data gracefully', () => {
    (useErrors as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    expect(screen.getByText('No errors found')).toBeInTheDocument();
    expect(screen.getByText('0')).toBeInTheDocument(); // Should show 0 counts
  });

  it('toggles multiple errors independently', async () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: [],
      isLoading: false,
      error: null,
    });

    const user = userEvent.setup();
    render(<ErrorTracker />);

    // Click on first error
    const firstError = screen.getByText('Critical system error').closest('div');
    if (firstError) {
      await user.click(firstError);
    }

    // Click on second error
    const secondError = screen.getByText('High priority warning').closest('div');
    if (secondError) {
      await user.click(secondError);
    }

    // Both should be expanded
    await waitFor(() => {
      const chevronDowns = screen.getAllByTestId('chevron-down');
      expect(chevronDowns).toHaveLength(2);
    });
  });

  it('shows filter counts correctly', () => {
    (useErrors as any).mockReturnValue({
      data: mockErrors,
      isLoading: false,
      error: null,
    });

    (useLogs as any).mockReturnValue({
      data: mockLogs,
      isLoading: false,
      error: null,
    });

    render(<ErrorTracker />);

    // Check filter buttons show correct counts
    expect(screen.getByText('Critical')).toBeInTheDocument();
    expect(screen.getByText('(1)')).toBeInTheDocument(); // Critical count
    expect(screen.getByText('High')).toBeInTheDocument();
    expect(screen.getByText('(1)')).toBeInTheDocument(); // High count
    expect(screen.getByText('Medium')).toBeInTheDocument();
    expect(screen.getByText('(1)')).toBeInTheDocument(); // Medium count
    expect(screen.getByText('Low')).toBeInTheDocument();
    expect(screen.getByText('(0)')).toBeInTheDocument(); // Low count
  });
});