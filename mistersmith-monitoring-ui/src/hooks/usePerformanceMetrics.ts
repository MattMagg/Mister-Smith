import { useEffect, useRef } from 'react';
import { recordRenderTime, recordError } from '../lib/telemetry/custom-metrics';

/**
 * Hook to measure and record component render performance
 * @param componentName - Name of the component being measured
 */
export function usePerformanceMetrics(componentName: string) {
  const renderStartTime = useRef<number>();
  const renderCount = useRef(0);
  
  useEffect(() => {
    // Record render time
    if (renderStartTime.current) {
      const renderTime = performance.now() - renderStartTime.current;
      recordRenderTime(componentName, renderTime);
      
      // Log slow renders in development
      if (import.meta.env.DEV && renderTime > 16.67) { // More than 1 frame at 60fps
        console.warn(`Slow render detected in ${componentName}: ${renderTime.toFixed(2)}ms`);
      }
    }
    
    // Increment render count
    renderCount.current++;
  });
  
  // Set render start time on each render
  renderStartTime.current = performance.now();
  
  // Return render count for debugging
  return { renderCount: renderCount.current };
}

/**
 * Hook to measure API call performance
 */
export function useAPIMetrics() {
  const measureAPICall = async <T,>(
    endpoint: string,
    apiCall: () => Promise<T>
  ): Promise<T> => {
    const startTime = performance.now();
    let success = false;
    
    try {
      const result = await apiCall();
      success = true;
      return result;
    } catch (error) {
      success = false;
      throw error;
    } finally {
      const duration = performance.now() - startTime;
      // Dynamically import to avoid circular dependencies
      import('../lib/telemetry/custom-metrics').then(({ recordAPICall }) => {
        recordAPICall(endpoint, duration, success);
      });
    }
  };
  
  return { measureAPICall };
}

/**
 * Hook to track user interaction metrics
 */
export function useInteractionMetrics() {
  const trackInteraction = (interactionType: string, metadata?: Record<string, any>) => {
    // This would integrate with OpenTelemetry spans
    // For now, we'll just record a custom metric
    import('../lib/telemetry/custom-metrics').then(({ meter }) => {
      const interactionCounter = meter.createCounter('mistersmith.ui.interactions', {
        description: 'User interactions'
      });
      
      interactionCounter.add(1, {
        interaction_type: interactionType,
        component: metadata?.component || 'unknown',
        ...metadata
      });
    });
  };
  
  return { trackInteraction };
}

/**
 * Hook to monitor component lifecycle
 */
export function useLifecycleMetrics(componentName: string) {
  const mountTime = useRef<number>();
  
  useEffect(() => {
    // Record mount time
    mountTime.current = performance.now();
    
    return () => {
      // Record component lifetime on unmount
      if (mountTime.current) {
        const lifetime = performance.now() - mountTime.current;
        
        import('../lib/telemetry/custom-metrics').then(({ meter }) => {
          const lifetimeHistogram = meter.createHistogram('mistersmith.ui.component_lifetime', {
            description: 'Component lifetime in milliseconds',
            unit: 'ms'
          });
          
          lifetimeHistogram.record(lifetime, {
            component: componentName
          });
        });
      }
    };
  }, [componentName]);
}