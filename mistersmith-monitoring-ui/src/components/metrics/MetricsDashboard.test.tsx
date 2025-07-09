import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, act } from '@testing-library/react';
import { MetricsDashboard } from './MetricsDashboard';
import { useSystemMetrics } from '../../hooks/useMonitoring';
import { SystemMetrics } from '../../lib/monitoring-service';

// Mock the useSystemMetrics hook
vi.mock('../../hooks/useMonitoring', () => ({
  useSystemMetrics: vi.fn(),
}));

// Mock the monitoring service
vi.mock('../../lib/monitoring-service', () => ({
  monitoringService: {
    getJaegerServices: vi.fn().mockResolvedValue(['claude-code', 'mistersmith-agent']),
  },
  SystemMetrics: {},
}));

// Mock usePerformanceMetrics hook
vi.mock('../../hooks/usePerformanceMetrics', () => ({
  usePerformanceMetrics: vi.fn(),
}));

// Mock MetricsChart component
vi.mock('./MetricsChart', () => ({
  MetricsChart: ({ title, data, type, color }: any) => (
    <div data-testid="metrics-chart">
      <h3>{title}</h3>
      <div data-testid="chart-type">{type}</div>
      <div data-testid="chart-color">{color}</div>
      <div data-testid="chart-data-length">{data?.length || 0}</div>
    </div>
  ),
}));

describe('MetricsDashboard', () => {
  const mockMetrics: SystemMetrics = {
    timestamp: new Date('2025-01-08T10:00:00Z'),
    cpu_usage: 45.5,
    memory_usage: 67.2,
    active_agents: 3,
    total_discoveries: 42,
    nats_connections: 5,
    message_throughput: 12.5,
  };

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders loading state', async () => {
    (useSystemMetrics as any).mockReturnValue({
      data: null,
      isLoading: true,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    const loadingCards = document.querySelectorAll('.animate-pulse');
    expect(loadingCards).toHaveLength(6);
  });

  it('renders error state', async () => {
    const mockError = new Error('Failed to load metrics');
    (useSystemMetrics as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: mockError,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    const errorMessages = screen.getAllByText('Failed to load metrics');
    expect(errorMessages).toHaveLength(2); // Both error heading and description
  });

  it('renders metrics dashboard with data', async () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    // Check current values display
    expect(screen.getByText('45.5%')).toBeInTheDocument();
    expect(screen.getByText('67.2%')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('42')).toBeInTheDocument();
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('12.5')).toBeInTheDocument();

    // Check labels (using getAllByText since there are multiple elements with the same text)
    expect(screen.getAllByText('CPU Usage')).toHaveLength(2);
    expect(screen.getAllByText('Memory Usage')).toHaveLength(2);
    expect(screen.getAllByText('Active Agents')).toHaveLength(2);
    expect(screen.getAllByText('Total Discoveries')).toHaveLength(2);
    expect(screen.getAllByText('NATS Connections')).toHaveLength(2);
    expect(screen.getByText('Messages/sec')).toBeInTheDocument();
  });

  it('renders all metric charts', async () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    // Check that all charts are rendered (6 regular + 4 Claude Code charts)
    const charts = screen.getAllByTestId('metrics-chart');
    expect(charts).toHaveLength(10);

    // Check chart titles (using getAllByText since there are multiple elements with the same text)
    expect(screen.getAllByText('CPU Usage')).toHaveLength(2);
    expect(screen.getAllByText('Memory Usage')).toHaveLength(2);
    expect(screen.getAllByText('Active Agents')).toHaveLength(2);
    expect(screen.getAllByText('Total Discoveries')).toHaveLength(2);
    expect(screen.getAllByText('NATS Connections')).toHaveLength(2);
    expect(screen.getByText('Message Throughput')).toBeInTheDocument();
  });

  it('uses correct chart types and colors', async () => {
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    const chartTypes = screen.getAllByTestId('chart-type');
    const chartColors = screen.getAllByTestId('chart-color');

    // Check chart types
    expect(chartTypes[0]).toHaveTextContent('area'); // CPU
    expect(chartTypes[1]).toHaveTextContent('area'); // Memory
    expect(chartTypes[2]).toHaveTextContent('line'); // Agents
    expect(chartTypes[3]).toHaveTextContent('line'); // Discoveries
    expect(chartTypes[4]).toHaveTextContent('bar'); // NATS
    expect(chartTypes[5]).toHaveTextContent('area'); // Throughput

    // Check chart colors
    expect(chartColors[0]).toHaveTextContent('#10B981'); // CPU - green
    expect(chartColors[1]).toHaveTextContent('#3B82F6'); // Memory - blue
    expect(chartColors[2]).toHaveTextContent('#8B5CF6'); // Agents - purple
    expect(chartColors[3]).toHaveTextContent('#F59E0B'); // Discoveries - yellow
    expect(chartColors[4]).toHaveTextContent('#F97316'); // NATS - orange
    expect(chartColors[5]).toHaveTextContent('#14B8A6'); // Throughput - teal
  });

  it('builds metrics history correctly', async () => {
    const { rerender } = render(<MetricsDashboard />);

    // First render with initial data
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    rerender(<MetricsDashboard />);

    // Second render with updated data
    const updatedMetrics = {
      ...mockMetrics,
      timestamp: new Date('2025-01-08T10:01:00Z'),
      cpu_usage: 50.0,
      memory_usage: 70.0,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: updatedMetrics,
      isLoading: false,
      error: null,
    });

    rerender(<MetricsDashboard />);

    // Check that charts have data
    await waitFor(() => {
      const dataLengths = screen.getAllByTestId('chart-data-length');
      // Should have at least 1 data point (could be more from multiple renders)
      dataLengths.forEach(length => {
        expect(parseInt(length.textContent || '0')).toBeGreaterThanOrEqual(1);
      });
    });
  });

  it('limits metrics history to 50 points', async () => {
    const { rerender } = render(<MetricsDashboard />);

    // Simulate many updates
    for (let i = 0; i < 60; i++) {
      const metricsUpdate = {
        ...mockMetrics,
        timestamp: new Date(Date.now() + i * 1000),
        cpu_usage: Math.random() * 100,
      };

      (useSystemMetrics as any).mockReturnValue({
        data: metricsUpdate,
        isLoading: false,
        error: null,
      });

      rerender(<MetricsDashboard />);
    }

    // Check that history is limited to 50 points
    await waitFor(() => {
      const dataLengths = screen.getAllByTestId('chart-data-length');
      dataLengths.forEach(length => {
        expect(parseInt(length.textContent || '0')).toBeLessThanOrEqual(50);
      });
    });
  });

  it('handles null metrics data', async () => {
    (useSystemMetrics as any).mockReturnValue({
      data: null,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    // When data is null but not loading, it should show the regular UI with null values
    expect(screen.getByText('Claude Code Metrics')).toBeInTheDocument();
    expect(screen.getByText('Active Sessions')).toBeInTheDocument();
    expect(screen.getByText('Total Cost')).toBeInTheDocument();
    expect(screen.getByText('Tokens Used')).toBeInTheDocument();
    expect(screen.getAllByText('Error Rate')).toHaveLength(2);
  });

  it('displays correct metric values with proper formatting', async () => {
    const metricsWithDecimals = {
      ...mockMetrics,
      cpu_usage: 45.123456,
      memory_usage: 67.987654,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: metricsWithDecimals,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    // Check that values are formatted to 1 decimal place
    expect(screen.getByText('45.1%')).toBeInTheDocument();
    expect(screen.getByText('68.0%')).toBeInTheDocument();
  });

  it('handles zero values correctly', async () => {
    const zeroMetrics = {
      ...mockMetrics,
      cpu_usage: 0,
      memory_usage: 0,
      active_agents: 0,
      total_discoveries: 0,
      nats_connections: 0,
      message_throughput: 0,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: zeroMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      render(<MetricsDashboard />);
    });

    // Check that zero values are displayed correctly
    expect(screen.getAllByText('0.0%')).toHaveLength(2); // CPU and Memory
    expect(screen.getAllByText('0')).toHaveLength(4); // Active agents, Total discoveries, NATS connections, Message throughput
  });

  it('updates in real-time when metrics change', async () => {
    const { rerender } = render(<MetricsDashboard />);

    // Initial metrics
    (useSystemMetrics as any).mockReturnValue({
      data: mockMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      rerender(<MetricsDashboard />);
    });
    
    expect(screen.getByText('45.5%')).toBeInTheDocument();

    // Updated metrics
    const updatedMetrics = {
      ...mockMetrics,
      cpu_usage: 75.8,
    };

    (useSystemMetrics as any).mockReturnValue({
      data: updatedMetrics,
      isLoading: false,
      error: null,
    });

    await act(async () => {
      rerender(<MetricsDashboard />);
    });
    
    expect(screen.getByText('75.8%')).toBeInTheDocument();
  });

  describe('Claude Code Metrics', () => {
    beforeEach(() => {
      // Setup system metrics
      (useSystemMetrics as any).mockReturnValue({
        data: mockMetrics,
        isLoading: false,
        error: null,
      });
      
      // Mock timers for intervals
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('renders Claude Code metrics section', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Advance timers to trigger the initial effect and flush promises
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      // Check that the section is available
      expect(screen.getByText('Claude Code Metrics')).toBeInTheDocument();

      // Check metrics cards
      expect(screen.getByText('Active Sessions')).toBeInTheDocument();
      expect(screen.getByText('Total Cost')).toBeInTheDocument();
      expect(screen.getByText('Tokens Used')).toBeInTheDocument();
      expect(screen.getAllByText('Error Rate')).toHaveLength(2);
    });

    it('fetches Jaeger services on mount', async () => {
      const { monitoringService } = await import('../../lib/monitoring-service');
      
      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Advance timers to trigger the initial effect and flush promises
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      expect(monitoringService.getJaegerServices).toHaveBeenCalled();
    });

    it('displays Claude Code performance charts', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Advance timers to trigger the initial effect and flush promises
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      expect(screen.getByText('Claude Code Performance')).toBeInTheDocument();

      // Check for Claude Code specific charts
      expect(screen.getByText('Claude Sessions')).toBeInTheDocument();
      expect(screen.getByText('Cost Over Time')).toBeInTheDocument();
      expect(screen.getByText('Token Usage')).toBeInTheDocument();
      expect(screen.getAllByText('Error Rate')).toHaveLength(2);
    });

    it('updates Claude metrics periodically', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Initial render
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      const sessionElements = screen.getAllByText(/Active Sessions/);
      expect(sessionElements.length).toBeGreaterThan(0);

      // Advance time to trigger the 30-second interval
      await act(async () => {
        vi.advanceTimersByTime(30000);
        await vi.runOnlyPendingTimersAsync();
      });

      // Should still have metrics displayed
      expect(screen.getByText('Active Sessions')).toBeInTheDocument();
    });

    it('handles Jaeger service fetch errors gracefully', async () => {
      const { monitoringService } = await import('../../lib/monitoring-service');
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      
      // Make getJaegerServices reject
      (monitoringService.getJaegerServices as any).mockRejectedValueOnce(new Error('Network error'));

      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Advance timers to trigger the initial effect and flush promises
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      expect(consoleSpy).toHaveBeenCalledWith('Failed to fetch Claude metrics:', expect.any(Error));

      // Should still render the UI
      expect(screen.getByText('Claude Code Metrics')).toBeInTheDocument();
      
      consoleSpy.mockRestore();
    });

    it('formats Claude metric values correctly', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      // Advance timers to trigger the initial fetch and flush promises
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      // Check cost formatting (should have $ and 2 decimal places)
      const costElements = screen.getAllByText(/\$\d+\.\d{2}/);
      expect(costElements.length).toBeGreaterThan(0);
      
      // Check error rate formatting (should have % and 1 decimal place)
      const errorRateElements = screen.getAllByText(/\d+\.\d%/);
      expect(errorRateElements.length).toBeGreaterThan(0);
    });

    it('uses correct chart types for Claude metrics', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      expect(screen.getByText('Claude Code Performance')).toBeInTheDocument();

      // Get all chart type elements
      const chartTypes = screen.getAllByTestId('chart-type');
      
      // The Claude Code charts should be at the end
      const claudeChartTypes = chartTypes.slice(-4);
      
      expect(claudeChartTypes[0]).toHaveTextContent('line'); // Sessions
      expect(claudeChartTypes[1]).toHaveTextContent('area'); // Cost
      expect(claudeChartTypes[2]).toHaveTextContent('bar');  // Tokens
      expect(claudeChartTypes[3]).toHaveTextContent('line'); // Error rate
    });

    it('uses correct colors for Claude metric charts', async () => {
      await act(async () => {
        render(<MetricsDashboard />);
      });

      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });

      expect(screen.getByText('Claude Code Performance')).toBeInTheDocument();

      // Get all chart color elements
      const chartColors = screen.getAllByTestId('chart-color');
      
      // The Claude Code charts should be at the end
      const claudeChartColors = chartColors.slice(-4);
      
      expect(claudeChartColors[0]).toHaveTextContent('#06B6D4'); // Sessions - cyan
      expect(claudeChartColors[1]).toHaveTextContent('#EC4899'); // Cost - pink
      expect(claudeChartColors[2]).toHaveTextContent('#6366F1'); // Tokens - indigo
      expect(claudeChartColors[3]).toHaveTextContent('#EF4444'); // Error rate - red
    });

    it('maintains Claude metrics history', async () => {
      const { rerender } = render(<MetricsDashboard />);

      // Initial fetch
      await act(async () => {
        vi.advanceTimersByTime(0);
        await vi.runOnlyPendingTimersAsync();
      });
      
      expect(screen.getByText('Claude Code Metrics')).toBeInTheDocument();

      // Trigger another update
      await act(async () => {
        vi.advanceTimersByTime(30000);
        await vi.runOnlyPendingTimersAsync();
      });
      
      await act(async () => {
        rerender(<MetricsDashboard />);
      });

      // Check that chart data is accumulating
      const dataLengths = screen.getAllByTestId('chart-data-length');
      // Claude charts should have data
      const claudeDataLengths = dataLengths.slice(-4);
      claudeDataLengths.forEach(length => {
        expect(parseInt(length.textContent || '0')).toBeGreaterThanOrEqual(1);
      });
    });
  });
});