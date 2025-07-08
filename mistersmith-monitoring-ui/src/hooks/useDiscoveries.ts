import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { apiClient } from '../lib/api-client';
import { Discovery, DiscoveryFilter } from '../types/discovery';
import { PaginationParams } from '../lib/api-client';

export function useDiscoveries(filter?: DiscoveryFilter, pagination?: PaginationParams) {
  return useQuery({
    queryKey: ['discoveries', filter, pagination],
    queryFn: () => apiClient.getDiscoveries(filter, pagination),
    keepPreviousData: true,
  });
}

export function useDiscovery(id: string) {
  return useQuery({
    queryKey: ['discovery', id],
    queryFn: () => apiClient.getDiscovery(id),
    enabled: !!id,
  });
}

export function useFilterDiscoveries() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (filter: DiscoveryFilter) => apiClient.filterDiscoveries(filter),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['discoveries'] });
    },
  });
}

export function useSubmitTestDiscovery() {
  const queryClient = useQueryClient();
  
  return useMutation({
    mutationFn: (discovery: Partial<Discovery>) => apiClient.submitTestDiscovery(discovery),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['discoveries'] });
    },
  });
}

export function useTraceDiscovery() {
  return useMutation({
    mutationFn: (id: string) => apiClient.traceDiscovery(id),
  });
}