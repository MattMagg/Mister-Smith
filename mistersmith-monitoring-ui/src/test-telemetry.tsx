import React, { useEffect, useState } from 'react';
import { trace, metrics } from '@opentelemetry/api';

function TestTelemetry() {
  const [status, setStatus] = useState('Initializing...');
  const [spanCount, setSpanCount] = useState(0);
  const [metricCount, setMetricCount] = useState(0);
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (msg: string) => {
    console.log(msg);
    setLogs(prev => [...prev, `${new Date().toLocaleTimeString()}: ${msg}`]);
  };

  useEffect(() => {
    addLog('TestTelemetry component mounted');
    
    // Check if OTel is initialized
    const tracerProvider = trace.getTracerProvider();
    addLog(`TracerProvider: ${tracerProvider ? 'Available' : 'Not available'}`);
    
    // Test trace generation
    const tracer = trace.getTracer('test-telemetry', '1.0.0');
    addLog('Got tracer instance');
    
    // Create a test span
    const span = tracer.startSpan('test-manual-span');
    addLog(`Created span: ${span ? 'Success' : 'Failed'}`);
    
    span.setAttribute('test.type', 'manual');
    span.setAttribute('test.timestamp', new Date().toISOString());
    span.addEvent('test-event', {
      'event.detail': 'Manual test event'
    });
    
    // Simulate some work
    setTimeout(() => {
      span.end();
      setSpanCount(prev => prev + 1);
      setStatus('Test span created and ended');
      addLog('Test span ended');
    }, 100);

    // Test metric generation
    const meter = metrics.getMeter('test-telemetry', '1.0.0');
    const counter = meter.createCounter('test_counter', {
      description: 'Test counter for telemetry validation'
    });
    
    counter.add(1, { 'test.type': 'manual' });
    setMetricCount(prev => prev + 1);
    addLog('Test metric recorded');
  }, []);

  const handleTestClick = () => {
    const tracer = trace.getTracer('test-telemetry', '1.0.0');
    const span = tracer.startSpan('test-click-span');
    span.setAttribute('interaction.type', 'button-click');
    span.setAttribute('button.id', 'test-button');
    
    // Simulate API call
    fetch('http://localhost:5173/test-endpoint')
      .then(() => {
        span.setStatus({ code: 1 });
        span.end();
        setSpanCount(prev => prev + 1);
        setStatus('Click span sent');
      })
      .catch((error) => {
        span.recordException(error);
        span.setStatus({ code: 2, message: error.message });
        span.end();
        setSpanCount(prev => prev + 1);
        setStatus('Click span sent (with error)');
      });
  };

  return (
    <div style={{ padding: '20px', backgroundColor: '#f0f0f0', margin: '20px' }}>
      <h2>OpenTelemetry Test Component</h2>
      <p>Status: {status}</p>
      <p>Spans created: {spanCount}</p>
      <p>Metrics recorded: {metricCount}</p>
      <button 
        id="test-button"
        onClick={handleTestClick}
        style={{ 
          padding: '10px 20px', 
          fontSize: '16px',
          backgroundColor: '#007bff',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer'
        }}
      >
        Generate Test Span
      </button>
      <div style={{ marginTop: '20px' }}>
        <p>Check browser console for telemetry logs</p>
        <p>Check Network tab for POST requests to localhost:4318</p>
      </div>
      <div style={{ marginTop: '20px', padding: '10px', backgroundColor: '#f9f9f9', border: '1px solid #ddd' }}>
        <h3>Debug Logs:</h3>
        <pre style={{ fontSize: '12px', maxHeight: '200px', overflow: 'auto' }}>
          {logs.join('\n')}
        </pre>
      </div>
    </div>
  );
}

export default TestTelemetry;