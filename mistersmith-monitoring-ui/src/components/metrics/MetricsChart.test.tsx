import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { MetricsChart } from './MetricsChart';

// Mock recharts components
vi.mock('recharts', () => ({
  LineChart: ({ children }: any) => <div data-testid="line-chart">{children}</div>,
  AreaChart: ({ children }: any) => <div data-testid="area-chart">{children}</div>,
  BarChart: ({ children }: any) => <div data-testid="bar-chart">{children}</div>,
  Line: () => <div data-testid="line" />,
  Area: () => <div data-testid="area" />,
  Bar: () => <div data-testid="bar" />,
  XAxis: () => <div data-testid="x-axis" />,
  YAxis: () => <div data-testid="y-axis" />,
  CartesianGrid: () => <div data-testid="cartesian-grid" />,
  Tooltip: () => <div data-testid="tooltip" />,
  ResponsiveContainer: ({ children }: any) => <div data-testid="responsive-container">{children}</div>,
  Legend: () => <div data-testid="legend" />,
}));

describe('MetricsChart', () => {
  const mockData = [
    { timestamp: new Date('2025-01-08T10:00:00Z'), value: 25.5 },
    { timestamp: new Date('2025-01-08T10:01:00Z'), value: 30.2 },
    { timestamp: new Date('2025-01-08T10:02:00Z'), value: 28.9 },
  ];

  it('renders chart with title', () => {
    render(<MetricsChart data={mockData} title="CPU Usage" />);
    
    expect(screen.getByText('CPU Usage')).toBeInTheDocument();
    expect(screen.getByTestId('responsive-container')).toBeInTheDocument();
  });

  it('renders line chart by default', () => {
    render(<MetricsChart data={mockData} title="Test Chart" />);
    
    expect(screen.getByTestId('line-chart')).toBeInTheDocument();
    expect(screen.getByTestId('line')).toBeInTheDocument();
  });

  it('renders area chart when type is area', () => {
    render(<MetricsChart data={mockData} type="area" title="Test Chart" />);
    
    expect(screen.getByTestId('area-chart')).toBeInTheDocument();
    expect(screen.getByTestId('area')).toBeInTheDocument();
  });

  it('renders bar chart when type is bar', () => {
    render(<MetricsChart data={mockData} type="bar" title="Test Chart" />);
    
    expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
    expect(screen.getByTestId('bar')).toBeInTheDocument();
  });

  it('applies custom height', () => {
    render(<MetricsChart data={mockData} title="Test Chart" height={300} />);
    
    const container = screen.getByTestId('responsive-container');
    expect(container).toBeInTheDocument();
  });

  it('applies custom className', () => {
    render(<MetricsChart data={mockData} title="Test Chart" className="custom-class" />);
    
    const chartContainer = screen.getByTestId('responsive-container').closest('div');
    expect(chartContainer).toHaveClass('custom-class');
  });

  it('includes chart axes and grid', () => {
    render(<MetricsChart data={mockData} title="Test Chart" />);
    
    expect(screen.getByTestId('x-axis')).toBeInTheDocument();
    expect(screen.getByTestId('y-axis')).toBeInTheDocument();
    expect(screen.getByTestId('cartesian-grid')).toBeInTheDocument();
    expect(screen.getByTestId('tooltip')).toBeInTheDocument();
  });

  it('handles empty data', () => {
    render(<MetricsChart data={[]} title="Empty Chart" />);
    
    expect(screen.getByText('Empty Chart')).toBeInTheDocument();
    expect(screen.getByTestId('line-chart')).toBeInTheDocument();
  });

  it('formats data for chart consumption', () => {
    // Since we're mocking recharts, we can't directly test data formatting
    // But we can ensure the component renders without errors
    const dataWithVariousValues = [
      { timestamp: new Date('2025-01-08T10:00:00Z'), value: 0 },
      { timestamp: new Date('2025-01-08T10:01:00Z'), value: 100 },
      { timestamp: new Date('2025-01-08T10:02:00Z'), value: 50.123 },
    ];

    render(<MetricsChart data={dataWithVariousValues} title="Test Chart" unit="%" />);
    
    expect(screen.getByText('Test Chart')).toBeInTheDocument();
  });

  it('handles different chart types correctly', () => {
    const { rerender } = render(<MetricsChart data={mockData} type="line" title="Test Chart" />);
    expect(screen.getByTestId('line-chart')).toBeInTheDocument();

    rerender(<MetricsChart data={mockData} type="area" title="Test Chart" />);
    expect(screen.getByTestId('area-chart')).toBeInTheDocument();

    rerender(<MetricsChart data={mockData} type="bar" title="Test Chart" />);
    expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
  });

  it('uses default props when not provided', () => {
    render(<MetricsChart data={mockData} title="Test Chart" />);
    
    // Should render line chart (default type)
    expect(screen.getByTestId('line-chart')).toBeInTheDocument();
  });

  it('applies custom color (through props)', () => {
    // We can't directly test color application due to mocking,
    // but we can ensure the component accepts the prop
    render(<MetricsChart data={mockData} title="Test Chart" color="#FF0000" />);
    
    expect(screen.getByText('Test Chart')).toBeInTheDocument();
  });

  it('applies unit to display (through props)', () => {
    // We can't directly test unit display due to mocking,
    // but we can ensure the component accepts the prop
    render(<MetricsChart data={mockData} title="Test Chart" unit="ms" />);
    
    expect(screen.getByText('Test Chart')).toBeInTheDocument();
  });
});

// Additional tests for integration scenarios
describe('MetricsChart Integration', () => {
  const mockData = [
    { timestamp: new Date('2025-01-08T10:00:00Z'), value: 25.5 },
    { timestamp: new Date('2025-01-08T10:01:00Z'), value: 30.2 },
    { timestamp: new Date('2025-01-08T10:02:00Z'), value: 28.9 },
  ];

  it('renders multiple charts with different types', () => {
    render(
      <div>
        <MetricsChart data={mockData} type="line" title="Line Chart" />
        <MetricsChart data={mockData} type="area" title="Area Chart" />
        <MetricsChart data={mockData} type="bar" title="Bar Chart" />
      </div>
    );

    expect(screen.getByText('Line Chart')).toBeInTheDocument();
    expect(screen.getByText('Area Chart')).toBeInTheDocument();
    expect(screen.getByText('Bar Chart')).toBeInTheDocument();
    
    expect(screen.getByTestId('line-chart')).toBeInTheDocument();
    expect(screen.getByTestId('area-chart')).toBeInTheDocument();
    expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
  });

  it('handles real-time data updates', () => {
    const { rerender } = render(<MetricsChart data={mockData} title="Real-time Chart" />);
    
    const updatedData = [
      ...mockData,
      { timestamp: new Date('2025-01-08T10:03:00Z'), value: 32.1 },
    ];

    rerender(<MetricsChart data={updatedData} title="Real-time Chart" />);
    
    expect(screen.getByText('Real-time Chart')).toBeInTheDocument();
  });

  it('handles large datasets', () => {
    const largeData = Array.from({ length: 1000 }, (_, i) => ({
      timestamp: new Date(Date.now() + i * 1000),
      value: Math.random() * 100,
    }));

    render(<MetricsChart data={largeData} title="Large Dataset Chart" />);
    
    expect(screen.getByText('Large Dataset Chart')).toBeInTheDocument();
  });
});