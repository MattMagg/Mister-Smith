import React from 'react';
import {
  LineChart,
  Line,
  AreaChart,
  Area,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from 'recharts';
import { format } from 'date-fns';

export type ChartType = 'line' | 'area' | 'bar';

interface MetricDataPoint {
  timestamp: Date;
  value: number;
  label?: string;
}

interface MetricsChartProps {
  data: MetricDataPoint[];
  type?: ChartType;
  title: string;
  unit?: string;
  color?: string;
  height?: number;
  className?: string;
}

export const MetricsChart: React.FC<MetricsChartProps> = ({
  data,
  type = 'line',
  title,
  unit = '',
  color = '#3B82F6',
  height = 200,
  className,
}) => {
  const formattedData = data.map(point => ({
    ...point,
    time: format(point.timestamp, 'HH:mm:ss'),
    formattedValue: `${point.value}${unit}`,
  }));
  
  const renderChart = () => {
    const commonProps = {
      data: formattedData,
      margin: { top: 5, right: 5, left: 5, bottom: 5 },
    };
    
    const CustomTooltip = ({ active, payload, label }: any) => {
      if (active && payload && payload.length) {
        return (
          <div className="bg-gray-800 border border-gray-700 rounded p-2">
            <p className="text-xs text-gray-400">{label}</p>
            <p className="text-sm font-medium text-gray-200">
              {payload[0].value}{unit}
            </p>
          </div>
        );
      }
      return null;
    };
    
    switch (type) {
      case 'area':
        return (
          <AreaChart {...commonProps}>
            <defs>
              <linearGradient id={`gradient-${title}`} x1="0" y1="0" x2="0" y2="1">
                <stop offset="5%" stopColor={color} stopOpacity={0.8} />
                <stop offset="95%" stopColor={color} stopOpacity={0.1} />
              </linearGradient>
            </defs>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis 
              dataKey="time" 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <YAxis 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <Tooltip content={<CustomTooltip />} />
            <Area
              type="monotone"
              dataKey="value"
              stroke={color}
              fillOpacity={1}
              fill={`url(#gradient-${title})`}
            />
          </AreaChart>
        );
        
      case 'bar':
        return (
          <BarChart {...commonProps}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis 
              dataKey="time" 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <YAxis 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <Tooltip content={<CustomTooltip />} />
            <Bar dataKey="value" fill={color} />
          </BarChart>
        );
        
      case 'line':
      default:
        return (
          <LineChart {...commonProps}>
            <CartesianGrid strokeDasharray="3 3" stroke="#374151" />
            <XAxis 
              dataKey="time" 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <YAxis 
              stroke="#9CA3AF"
              fontSize={12}
            />
            <Tooltip content={<CustomTooltip />} />
            <Line
              type="monotone"
              dataKey="value"
              stroke={color}
              strokeWidth={2}
              dot={false}
            />
          </LineChart>
        );
    }
  };
  
  return (
    <div className={`bg-gray-800 rounded-lg p-4 border border-gray-700 ${className}`}>
      <h3 className="text-sm font-medium text-gray-200 mb-4">{title}</h3>
      <ResponsiveContainer width="100%" height={height}>
        {renderChart()}
      </ResponsiveContainer>
    </div>
  );
};