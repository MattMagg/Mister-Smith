import { trace } from '@opentelemetry/api';
import { WebTracerProvider, BatchSpanProcessor, ConsoleSpanExporter } from '@opentelemetry/sdk-trace-web';
import { Resource } from '@opentelemetry/resources';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-http';

export function initializeSimpleOtel() {
  console.log('Starting simple OpenTelemetry initialization...');
  
  try {
    // 1. Create resource
    const resource = new Resource({
      'service.name': 'mistersmith-ui-simple',
      'service.version': '1.0.0'
    });
    console.log('✅ Resource created');
    
    // 2. Create provider
    const provider = new WebTracerProvider({
      resource
    });
    console.log('✅ Provider created');
    
    // 3. Add console exporter for debugging
    const consoleExporter = new ConsoleSpanExporter();
    provider.addSpanProcessor(new BatchSpanProcessor(consoleExporter));
    console.log('✅ Console exporter added');
    
    // 4. Try to add OTLP exporter
    try {
      const otlpExporter = new OTLPTraceExporter({
        url: 'http://localhost:4318/v1/traces',
        headers: {},
      });
      
      const processor = new BatchSpanProcessor(otlpExporter, {
        scheduledDelayMillis: 1000, // Export every 1 second for testing
      });
      
      provider.addSpanProcessor(processor);
      console.log('✅ OTLP exporter added');
      
      // Force flush after 2 seconds to test
      setTimeout(() => {
        console.log('Forcing flush...');
        provider.forceFlush().then(() => {
          console.log('✅ Force flush completed');
        }).catch(err => {
          console.error('❌ Force flush failed:', err);
        });
      }, 2000);
      
    } catch (error) {
      console.error('❌ Failed to create OTLP exporter:', error);
    }
    
    // 5. Register provider
    trace.setGlobalTracerProvider(provider);
    console.log('✅ Provider registered globally');
    
    // 6. Create test span immediately
    const tracer = trace.getTracer('simple-test', '1.0.0');
    const span = tracer.startSpan('initialization-test-span');
    span.setAttribute('test', 'simple-setup');
    span.end();
    console.log('✅ Test span created');
    
    return { provider, success: true };
    
  } catch (error) {
    console.error('❌ Simple OTel initialization failed:', error);
    return { provider: null, success: false, error };
  }
}