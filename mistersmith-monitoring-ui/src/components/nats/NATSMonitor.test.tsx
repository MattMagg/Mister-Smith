import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { NATSMonitor } from './NATSMonitor';
import { useSystemMetrics, useSystemHealth } from '../../hooks/useMonitoring';
import { SystemMetrics, SystemHealth } from '../../lib/monitoring-service';

// Mock the hooks
vi.mock('../../hooks/useMonitoring', () => ({
  useSystemMetrics: vi.fn(),
  useSystemHealth: vi.fn(),
}));

// Mock MetricsChart component
vi.mock('../metrics/MetricsChart', () => ({
  MetricsChart: ({ title, data, type, color }: any) => (
    <div data-testid="metrics-chart">
      <h3>{title}</h3>
      <div data-testid="chart-type">{type}</div>
      <div data-testid="chart-color">{color}</div>
      <div data-testid="chart-data-length">{data.length}</div>
    </div>
  ),
}));

// Mock lucide-react icons
vi.mock('lucide-react', () => ({
  AlertCircle: () => <div data-testid="alert-circle" />,
  CheckCircle: () => <div data-testid="check-circle" />,
  Activity: () => <div data-testid="activity" />,
  Users: () => <div data-testid="users" />,
  MessageSquare: () => <div data-testid="message-square" />,
  Zap: () => <div data-testid="zap" />,
}));

describe('NATSMonitor', () => {
  const mockMetrics: SystemMetrics = {
    timestamp: new Date('2025-01-08T10:00:00Z'),
    cpu_usage: 45.5,
    memory_usage: 67.2,
    active_agents: 3,
    total_discoveries: 42,
    nats_connections: 5,
    message_throughput: 12.5,
  };

  const mockHealth: SystemHealth = {
    status: 'healthy',
    uptime: 3600,
    version: '1.0.0',
    timestamp: new Date('2025-01-08T10:00:00Z'),
    components: {
      nats_server: 'up',
      database: 'up',
      message_queue: 'up',
    },
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders NATS status with healthy state', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('NATS Server')).toBeInTheDocument();
    expect(screen.getByText('up')).toBeInTheDocument();
    expect(screen.getByTestId('check-circle')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument(); // Active connections
  });

  it('renders NATS status with down state', () => {
    const downHealth = {
      ...mockHealth,
      components: {
        ...mockHealth.components,
        nats_server: 'down',
      },
    };

    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: downHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('down')).toBeInTheDocument();
    expect(screen.getByTestId('alert-circle')).toBeInTheDocument();
  });

  it('renders NATS status with unknown state', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('unknown')).toBeInTheDocument();
    expect(screen.getByTestId('activity')).toBeInTheDocument();
  });

  it('displays correct metric values', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    // Check metrics grid
    expect(screen.getByText('5')).toBeInTheDocument(); // Connections
    expect(screen.getByText('12.5')).toBeInTheDocument(); // Messages/sec
    expect(screen.getByText('3')).toBeInTheDocument(); // Active agents
    expect(screen.getByText('60m')).toBeInTheDocument(); // Uptime in minutes
  });

  it('handles zero values correctly', () => {
    const zeroMetrics = {
      ...mockMetrics,
      nats_connections: 0,
      message_throughput: 0,
      active_agents: 0,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: zeroMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    // Check that zero values are displayed
    expect(screen.getByText('0')).toBeInTheDocument(); // Should appear multiple times
  });

  it('handles null metrics data', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    // Should display 0 for all metrics when data is null
    expect(screen.getByText('0')).toBeInTheDocument(); // Should appear multiple times
  });

  it('displays correct metric labels', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('Connections')).toBeInTheDocument();
    expect(screen.getByText('Messages/sec')).toBeInTheDocument();
    expect(screen.getByText('Active Agents')).toBeInTheDocument();
    expect(screen.getByText('Uptime')).toBeInTheDocument();
    expect(screen.getByText('Active Connections')).toBeInTheDocument();
  });

  it('displays correct icons for metrics', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByTestId('users')).toBeInTheDocument(); // Connections
    expect(screen.getByTestId('message-square')).toBeInTheDocument(); // Messages
    expect(screen.getByTestId('zap')).toBeInTheDocument(); // Active agents
    expect(screen.getByTestId('activity')).toBeInTheDocument(); // Uptime
  });

  it('renders performance charts', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    const charts = screen.getAllByTestId('metrics-chart');
    expect(charts).toHaveLength(2);

    // Check chart titles
    expect(screen.getByText('NATS Connections')).toBeInTheDocument();
    expect(screen.getByText('Message Throughput')).toBeInTheDocument();
  });

  it('uses correct chart types and colors', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    const chartTypes = screen.getAllByTestId('chart-type');
    const chartColors = screen.getAllByTestId('chart-color');

    // Check chart types
    expect(chartTypes[0]).toHaveTextContent('line'); // Connections
    expect(chartTypes[1]).toHaveTextContent('area'); // Throughput

    // Check chart colors
    expect(chartColors[0]).toHaveTextContent('#3B82F6'); // Blue
    expect(chartColors[1]).toHaveTextContent('#10B981'); // Green
  });

  it('builds NATS history correctly', async () => {
    const { rerender } = render(<NATSMonitor />);

    // First render with initial data
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    rerender(<NATSMonitor />);

    // Second render with updated data
    const updatedMetrics = {
      ...mockMetrics,
      timestamp: new Date('2025-01-08T10:01:00Z'),
      nats_connections: 7,
      message_throughput: 15.2,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: updatedMetrics,
      isLoading: false,
      error: null,
    });

    rerender(<NATSMonitor />);

    // Check that charts have data
    await waitFor(() => {
      const dataLengths = screen.getAllByTestId('chart-data-length');
      dataLengths.forEach(length => {
        expect(parseInt(length.textContent || '0')).toBeGreaterThanOrEqual(1);
      });
    });
  });

  it('limits NATS history to 50 points', async () => {
    const { rerender } = render(<NATSMonitor />);

    // Simulate many updates
    for (let i = 0; i < 60; i++) {
      const metricsUpdate = {
        ...mockMetrics,
        timestamp: new Date(Date.now() + i * 1000),
        nats_connections: Math.floor(Math.random() * 10),
        message_throughput: Math.random() * 100,
      };

      (useSystemMetrics as any).mockReturnValue({
        data: metricsUpdate,
        isLoading: false,
        error: null,
      });

      (useSystemHealth as any).mockReturnValue({
        data: mockHealth,
        isLoading: false,
        error: null,
      });

      rerender(<NATSMonitor />);
    }

    // Check that history is limited to 50 points
    await waitFor(() => {
      const dataLengths = screen.getAllByTestId('chart-data-length');
      dataLengths.forEach(length => {
        expect(parseInt(length.textContent || '0')).toBeLessThanOrEqual(50);
      });
    });
  });

  it('displays connection details', () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: mockHealth,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    // Check subjects
    expect(screen.getByText('• discoveries.> - Discovery sharing')).toBeInTheDocument();
    expect(screen.getByText('• agents.> - Agent communication')).toBeInTheDocument();
    expect(screen.getByText('• metrics.> - System metrics')).toBeInTheDocument();
    expect(screen.getByText('• logs.> - Log streaming')).toBeInTheDocument();

    // Check connection info
    expect(screen.getByText('Server: localhost:4222')).toBeInTheDocument();
    expect(screen.getByText('Version: 1.0.0')).toBeInTheDocument();
    expect(screen.getByText('Cluster: Single Node')).toBeInTheDocument();
    expect(screen.getByText('TLS: Disabled')).toBeInTheDocument();
  });

  it('handles unknown version gracefully', () => {
    const healthWithoutVersion = {
      ...mockHealth,
      version: undefined,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: healthWithoutVersion,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('Version: Unknown')).toBeInTheDocument();
  });

  it('calculates uptime correctly', () => {
    const healthWithUptime = {
      ...mockHealth,
      uptime: 7200, // 2 hours
    };

    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: healthWithUptime,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('120m')).toBeInTheDocument(); // 7200 seconds = 120 minutes
  });

  it('handles zero uptime', () => {
    const healthWithZeroUptime = {
      ...mockHealth,
      uptime: 0,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    (useSystemHealth as any).mockReturnValue({
      data: healthWithZeroUptime,
      isLoading: false,
      error: null,
    });

    render(<NATSMonitor />);

    expect(screen.getByText('0m')).toBeInTheDocument();
  });
});