import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePerformanceMetrics, useAPIMetrics, useInteractionMetrics, useLifecycleMetrics } from './usePerformanceMetrics';

// Mock the custom metrics module
vi.mock('../lib/telemetry/custom-metrics', () => ({
  recordRenderTime: vi.fn(),
  recordError: vi.fn(),
  recordAPICall: vi.fn(),
  meter: {
    createCounter: vi.fn(() => ({
      add: vi.fn()
    })),
    createHistogram: vi.fn(() => ({
      record: vi.fn()
    }))
  }
}));

// Mock performance.now()
const mockPerformanceNow = vi.fn();
global.performance.now = mockPerformanceNow;

describe('usePerformanceMetrics', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockPerformanceNow.mockReturnValue(100);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should track render performance', async () => {
    const { recordRenderTime } = await import('../lib/telemetry/custom-metrics');
    
    // First render
    mockPerformanceNow.mockReturnValueOnce(100);
    const { result, rerender } = renderHook(() => 
      usePerformanceMetrics('TestComponent')
    );

    // Second render
    mockPerformanceNow.mockReturnValueOnce(200);
    mockPerformanceNow.mockReturnValueOnce(210);
    rerender();

    // Should record render time
    expect(recordRenderTime).toHaveBeenCalledWith('TestComponent', 10);
    expect(result.current.renderCount).toBe(1);
  });

  it('should warn about slow renders in dev mode', async () => {
    const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const originalEnv = import.meta.env.DEV;
    Object.defineProperty(import.meta.env, 'DEV', { value: true, writable: true });

    // Mock a slow render (> 16.67ms)
    mockPerformanceNow.mockReturnValueOnce(100);
    renderHook(() => usePerformanceMetrics('SlowComponent'));

    mockPerformanceNow.mockReturnValueOnce(200);
    mockPerformanceNow.mockReturnValueOnce(220); // 20ms render
    
    // Force a re-render
    const { rerender } = renderHook(() => usePerformanceMetrics('SlowComponent'));
    rerender();

    expect(consoleSpy).toHaveBeenCalledWith(
      expect.stringContaining('Slow render detected in SlowComponent: 20.00ms')
    );

    Object.defineProperty(import.meta.env, 'DEV', { value: originalEnv, writable: true });
    consoleSpy.mockRestore();
  });
});

describe('useAPIMetrics', () => {
  it('should measure successful API calls', async () => {
    const { recordAPICall } = await import('../lib/telemetry/custom-metrics');
    
    mockPerformanceNow
      .mockReturnValueOnce(100) // Start time
      .mockReturnValueOnce(300); // End time

    const { result } = renderHook(() => useAPIMetrics());

    const mockApiCall = vi.fn().mockResolvedValue({ data: 'test' });
    const response = await result.current.measureAPICall('/api/test', mockApiCall);

    expect(response).toEqual({ data: 'test' });
    expect(mockApiCall).toHaveBeenCalled();
    
    // Wait for dynamic import
    await new Promise(resolve => setTimeout(resolve, 0));
    
    expect(recordAPICall).toHaveBeenCalledWith('/api/test', 200, true);
  });

  it('should measure failed API calls', async () => {
    const { recordAPICall } = await import('../lib/telemetry/custom-metrics');
    
    mockPerformanceNow
      .mockReturnValueOnce(100) // Start time
      .mockReturnValueOnce(150); // End time

    const { result } = renderHook(() => useAPIMetrics());

    const mockApiCall = vi.fn().mockRejectedValue(new Error('API Error'));
    
    await expect(
      result.current.measureAPICall('/api/test', mockApiCall)
    ).rejects.toThrow('API Error');

    // Wait for dynamic import
    await new Promise(resolve => setTimeout(resolve, 0));
    
    expect(recordAPICall).toHaveBeenCalledWith('/api/test', 50, false);
  });
});

describe('useInteractionMetrics', () => {
  it('should track user interactions', async () => {
    const { meter } = await import('../lib/telemetry/custom-metrics');
    const mockCounter = { add: vi.fn() };
    (meter.createCounter as any).mockReturnValue(mockCounter);

    const { result } = renderHook(() => useInteractionMetrics());

    act(() => {
      result.current.trackInteraction('button_click', { component: 'Header' });
    });

    // Wait for dynamic import
    await new Promise(resolve => setTimeout(resolve, 0));

    expect(meter.createCounter).toHaveBeenCalledWith('mistersmith.ui.interactions', {
      description: 'User interactions'
    });
    
    expect(mockCounter.add).toHaveBeenCalledWith(1, {
      interaction_type: 'button_click',
      component: 'Header'
    });
  });
});

describe('useLifecycleMetrics', () => {
  it('should track component lifecycle', async () => {
    const { meter } = await import('../lib/telemetry/custom-metrics');
    const mockHistogram = { record: vi.fn() };
    (meter.createHistogram as any).mockReturnValue(mockHistogram);

    mockPerformanceNow
      .mockReturnValueOnce(100) // Mount time
      .mockReturnValueOnce(1100); // Unmount time

    const { unmount } = renderHook(() => 
      useLifecycleMetrics('TestComponent')
    );

    // Unmount the component
    unmount();

    // Wait for dynamic import
    await new Promise(resolve => setTimeout(resolve, 0));

    expect(meter.createHistogram).toHaveBeenCalledWith('mistersmith.ui.component_lifetime', {
      description: 'Component lifetime in milliseconds',
      unit: 'ms'
    });
    
    expect(mockHistogram.record).toHaveBeenCalledWith(1000, {
      component: 'TestComponent'
    });
  });

  it('should handle component that never unmounts', () => {
    const { meter } = await import('../lib/telemetry/custom-metrics');
    const mockHistogram = { record: vi.fn() };
    (meter.createHistogram as any).mockReturnValue(mockHistogram);

    mockPerformanceNow.mockReturnValue(100);

    renderHook(() => useLifecycleMetrics('PersistentComponent'));

    // Don't unmount - just let it exist
    
    // Should not record anything
    expect(mockHistogram.record).not.toHaveBeenCalled();
  });
});