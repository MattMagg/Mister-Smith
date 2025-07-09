import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { monitoringService, SystemHealth, SystemMetrics, ErrorEvent, LogEntry, TraceEvent } from '../lib/monitoring-service';
import { Discovery } from '../types/discovery';
import { Agent } from '../types/agent';

export function useMonitoring() {
  const queryClient = useQueryClient();
  const [connectionStatus, setConnectionStatus] = useState<string>('disconnected');
  const [discoveries, setDiscoveries] = useState<Discovery[]>([]);
  const [agents, setAgents] = useState<Agent[]>([]);
  const [systemHealth, setSystemHealth] = useState<SystemHealth | null>(null);
  const [systemMetrics, setSystemMetrics] = useState<SystemMetrics | null>(null);
  const [errors, setErrors] = useState<ErrorEvent[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [traces, setTraces] = useState<TraceEvent[]>([]);
  
  // Subscribe to real-time updates
  useEffect(() => {
    // Connect to the monitoring service
    monitoringService.connect();
    
    const subscriptions = [
      monitoringService.getConnectionStatus().subscribe(setConnectionStatus),
      monitoringService.getDiscoveries().subscribe(setDiscoveries),
      monitoringService.getAgents().subscribe(setAgents),
      monitoringService.getSystemHealth().subscribe(setSystemHealth),
      monitoringService.getSystemMetrics().subscribe(setSystemMetrics),
      monitoringService.getErrors().subscribe(setErrors),
      monitoringService.getLogs().subscribe(setLogs),
      monitoringService.getTraces().subscribe(setTraces)
    ];
    
    return () => {
      subscriptions.forEach(sub => sub.unsubscribe());
    };
  }, []);
  
  // Invalidate queries when new data comes in
  useEffect(() => {
    if (discoveries.length > 0) {
      queryClient.invalidateQueries({ queryKey: ['discoveries'] });
    }
  }, [discoveries, queryClient]);
  
  useEffect(() => {
    if (agents.length > 0) {
      queryClient.invalidateQueries({ queryKey: ['agents'] });
    }
  }, [agents, queryClient]);
  
  return {
    connectionStatus,
    discoveries,
    agents,
    systemHealth,
    systemMetrics,
    errors,
    logs,
    traces,
    isConnected: connectionStatus === 'connected'
  };
}

export function useSystemHealth() {
  const { systemHealth } = useMonitoring();
  
  return useQuery({
    queryKey: ['system-health'],
    queryFn: () => systemHealth,
    enabled: !!systemHealth,
    refetchInterval: 30000, // Refetch every 30 seconds
    staleTime: 25000, // Data is fresh for 25 seconds
  });
}

export function useSystemMetrics() {
  const { systemMetrics } = useMonitoring();
  
  return useQuery({
    queryKey: ['system-metrics'],
    queryFn: () => systemMetrics,
    enabled: !!systemMetrics,
    refetchInterval: 10000, // Refetch every 10 seconds
    staleTime: 5000, // Data is fresh for 5 seconds
  });
}

export function useDiscoveries() {
  const { discoveries } = useMonitoring();
  
  return useQuery({
    queryKey: ['discoveries'],
    queryFn: () => discoveries,
    enabled: discoveries.length > 0,
    staleTime: 1000, // Data is fresh for 1 second
  });
}

export function useAgents() {
  const { agents } = useMonitoring();
  
  return useQuery({
    queryKey: ['agents'],
    queryFn: () => agents,
    enabled: agents.length > 0,
    staleTime: 5000, // Data is fresh for 5 seconds
  });
}

export function useErrors() {
  const { errors } = useMonitoring();
  
  return useQuery({
    queryKey: ['errors'],
    queryFn: () => errors,
    enabled: errors.length > 0,
    staleTime: 2000, // Data is fresh for 2 seconds
  });
}

export function useDashboardStatus() {
  const [dashboardStatus, setDashboardStatus] = useState<{
    isConnected: boolean;
    totalDiscoveries: number;
    activeAgents: number;
    systemHealth: SystemHealth | null;
    recentErrors: ErrorEvent[];
    recentLogs: LogEntry[];
  } | null>(null);
  
  useEffect(() => {
    const subscription = monitoringService.getDashboardStatus().subscribe(setDashboardStatus);
    return () => subscription.unsubscribe();
  }, []);
  
  return useQuery({
    queryKey: ['dashboard-status'],
    queryFn: () => dashboardStatus,
    enabled: !!dashboardStatus,
    staleTime: 1000, // Data is fresh for 1 second
  });
}

export function useLogs() {
  const { logs } = useMonitoring();
  
  return useQuery({
    queryKey: ['logs'],
    queryFn: () => logs,
    enabled: logs.length > 0,
    staleTime: 1000, // Data is fresh for 1 second
  });
}

export function useLogsByLevel(level: LogEntry['level']) {
  const [filteredLogs, setFilteredLogs] = useState<LogEntry[]>([]);
  
  useEffect(() => {
    const subscription = monitoringService.getLogsByLevel(level).subscribe(setFilteredLogs);
    return () => subscription.unsubscribe();
  }, [level]);
  
  return useQuery({
    queryKey: ['logs', level],
    queryFn: () => filteredLogs,
    enabled: filteredLogs.length > 0,
    staleTime: 1000, // Data is fresh for 1 second
  });
}

export function useTraces() {
  const { traces } = useMonitoring();
  
  return useQuery({
    queryKey: ['traces'],
    queryFn: () => traces,
    enabled: traces.length > 0,
    staleTime: 2000, // Data is fresh for 2 seconds
  });
}
