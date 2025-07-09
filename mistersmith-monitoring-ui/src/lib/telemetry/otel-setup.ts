import { metrics, trace } from '@opentelemetry/api';
import { 
  MeterProvider, 
  PeriodicExportingMetricReader,
  ConsoleMetricExporter,
  View,
  Aggregation,
  InstrumentType
} from '@opentelemetry/sdk-metrics';
import { WebTracerProvider, BatchSpanProcessor } from '@opentelemetry/sdk-trace-web';
import { Resource } from '@opentelemetry/resources';
import { ATTR_SERVICE_NAME, ATTR_SERVICE_VERSION } from '@opentelemetry/semantic-conventions';
import { registerInstrumentations } from '@opentelemetry/instrumentation';
import { DocumentLoadInstrumentation } from '@opentelemetry/instrumentation-document-load';
import { UserInteractionInstrumentation } from '@opentelemetry/instrumentation-user-interaction';
import { FetchInstrumentation } from '@opentelemetry/instrumentation-fetch';
import { XMLHttpRequestInstrumentation } from '@opentelemetry/instrumentation-xml-http-request';
import { LongTaskInstrumentation } from '@opentelemetry/instrumentation-long-task';
import { OTLPMetricExporter } from '@opentelemetry/exporter-metrics-otlp-http';
import { OTLPTraceExporter } from '@opentelemetry/exporter-trace-otlp-http';
import { SSEMetricExporter } from './sse-metric-exporter';
import { registerSystemMetricCallbacks } from './custom-metrics';
import { monitoringService } from '../monitoring-service';

export function initializeOpenTelemetry() {
  console.log('🚀 Initializing OpenTelemetry...');
  
  // Determine if we're in Docker environment
  const isDocker = window.location.hostname === 'mistersmith-ui' || import.meta.env.VITE_DOCKER_ENV === 'true';
  
  // Configure endpoints based on environment
  const metricsEndpoint = import.meta.env.VITE_METRICS_ENDPOINT || 
    (isDocker ? 'http://otel-collector:4318/v1/metrics' : 'http://localhost:4318/v1/metrics');
  
  const tracesEndpoint = import.meta.env.VITE_TRACES_ENDPOINT || 
    (isDocker ? 'http://otel-collector:4318/v1/traces' : 'http://localhost:4318/v1/traces');
  
  console.log('📡 Telemetry endpoints:', { metricsEndpoint, tracesEndpoint });
  
  // Create resource
  const resource = new Resource({
    [ATTR_SERVICE_NAME]: 'mistersmith-monitoring-ui',
    [ATTR_SERVICE_VERSION]: '1.0.0',
    'deployment.environment': import.meta.env.MODE || 'development',
    'deployment.docker': isDocker.toString(),
    'telemetry.sdk.language': 'javascript',
    'telemetry.sdk.name': '@opentelemetry/sdk-metrics',
    'host.name': window.location.hostname,
    'browser.user_agent': navigator.userAgent,
    'browser.language': navigator.language,
    'service.instance.id': `${window.location.hostname}-${Date.now()}`,
    'service.namespace': 'mistersmith'
  });

  // Initialize Tracer Provider
  const tracerProvider = new WebTracerProvider({
    resource,
  });

  // Add trace exporters
  const readers = [];
  
  // OTLP HTTP Exporter for traces (primary)
  try {
    const traceExporter = new OTLPTraceExporter({
      url: tracesEndpoint,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    tracerProvider.addSpanProcessor(new BatchSpanProcessor(traceExporter, {
      maxQueueSize: 1024,
      maxExportBatchSize: 512,
      scheduledDelayMillis: 5000,
      exportTimeoutMillis: 30000,
    }));
    
    console.log('✅ Trace exporter configured:', tracesEndpoint);
    
    // Debug: log when export happens
    console.log('Trace exporter created:', traceExporter);
  } catch (error) {
    console.warn('❌ Failed to configure trace exporter:', error);
  }

  // Register tracer provider
  trace.setGlobalTracerProvider(tracerProvider);

  // OTLP HTTP Exporter for metrics (primary)
  try {
    const otlpMetricExporter = new OTLPMetricExporter({
      url: metricsEndpoint,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    
    readers.push(new PeriodicExportingMetricReader({
      exporter: otlpMetricExporter,
      exportIntervalMillis: 10000
    }));
    
    console.log('OTLP metric exporter configured:', metricsEndpoint);
  } catch (error) {
    console.warn('Failed to configure OTLP metric exporter:', error);
  }

  // SSE Exporter for fallback (non-Docker only)
  if (!isDocker) {
    try {
      readers.push(new PeriodicExportingMetricReader({
        exporter: new SSEMetricExporter({
          endpoint: import.meta.env.VITE_SSE_METRICS_ENDPOINT || 'http://localhost:8080/api/v1/metrics',
          exportIntervalMillis: 10000,
          batchSize: 50
        }),
        exportIntervalMillis: 10000
      }));
      
      console.log('SSE metric exporter configured');
    } catch (error) {
      console.warn('Failed to configure SSE metric exporter:', error);
    }
  }

  // Console exporter for debugging (dev only)
  if (import.meta.env.DEV) {
    readers.push(new PeriodicExportingMetricReader({
      exporter: new ConsoleMetricExporter(),
      exportIntervalMillis: 60000 // Less frequent for console
    }));
  }

  // Initialize Meter Provider with multiple exporters
  const meterProvider = new MeterProvider({
    resource,
    readers,
    views: [
      // Custom view for HTTP request duration histogram
      new View({
        aggregation: Aggregation.ExplicitBucketHistogram([
          0, 5, 10, 25, 50, 75, 100, 250, 500, 750, 1000, 2500, 5000
        ]),
        instrumentName: 'http.client.duration',
        instrumentType: InstrumentType.HISTOGRAM
      }),
      // Custom view for render time histogram
      new View({
        aggregation: Aggregation.ExplicitBucketHistogram([
          0, 1, 5, 10, 20, 50, 100, 200, 500
        ]),
        instrumentName: 'mistersmith.ui.render_time',
        instrumentType: InstrumentType.HISTOGRAM
      }),
      // Drop noisy internal metrics
      new View({
        aggregation: Aggregation.Drop(),
        instrumentName: 'react.component.render.count',
        meterName: '*'
      })
    ]
  });

  // Set as global meter provider
  metrics.setGlobalMeterProvider(meterProvider);

  // Register instrumentations
  registerInstrumentations({
    instrumentations: [
      new DocumentLoadInstrumentation({
        // Measure initial page load performance
      }),
      new UserInteractionInstrumentation({
        eventNames: ['click', 'submit'],
        // Don't prevent any spans from being created
        shouldPreventSpanCreation: () => false
      }),
      new FetchInstrumentation({
        // Track API calls
        propagateTraceHeaderCorsUrls: [
          /^http:\/\/localhost:\d+/,
          /^http:\/\/otel-collector:\d+/,
          /^http:\/\/jaeger:\d+/,
          /^http:\/\/prometheus:\d+/,
          /^http:\/\/mistersmith-ui:\d+/,
          /^https?:\/\/.*\.mistersmith\.ai/
        ],
        clearTimingResources: true,
        // Add custom attributes
        applyCustomAttributesOnSpan: (span, request, response) => {
          span.setAttribute('http.request.body.size', request.headers.get('content-length') || 0);
          span.setAttribute('deployment.environment', import.meta.env.MODE || 'development');
          span.setAttribute('deployment.docker', isDocker.toString());
          if (response) {
            span.setAttribute('http.response.body.size', response.headers.get('content-length') || 0);
          }
        }
      }),
      new XMLHttpRequestInstrumentation({
        // Track XHR requests (if any legacy code uses them)
        propagateTraceHeaderCorsUrls: [
          /^http:\/\/localhost:\d+/,
          /^http:\/\/otel-collector:\d+/,
          /^http:\/\/jaeger:\d+/,
          /^http:\/\/prometheus:\d+/,
          /^http:\/\/mistersmith-ui:\d+/
        ]
      }),
      new LongTaskInstrumentation({
        // Monitor tasks blocking the main thread
        observerCallback: (span, longtaskEvent) => {
          span.setAttribute('longtask.duration', longtaskEvent.duration);
          span.setAttribute('longtask.name', longtaskEvent.name);
          span.setAttribute('longtask.entry_type', longtaskEvent.entryType);
          span.setAttribute('location.pathname', window.location.pathname);
          
          // Warn if task is too long
          if (longtaskEvent.duration > 100) {
            console.warn(`Long task detected: ${longtaskEvent.duration}ms`, longtaskEvent);
          }
        }
      })
    ]
  });

  // Register system metric callbacks
  registerSystemMetricCallbacks(() => {
    const metrics = monitoringService['systemMetrics$'].value;
    return {
      cpu: metrics?.cpu_usage || 0,
      memory: metrics?.memory_usage || 0
    };
  });

  // Log initialization
  console.log('OpenTelemetry initialized', {
    service: ATTR_SERVICE_NAME,
    environment: import.meta.env.MODE || 'development',
    isDocker,
    metricsEndpoint,
    tracesEndpoint,
    jaegerUI: import.meta.env.VITE_JAEGER_UI_URL || (isDocker ? 'http://localhost:16686' : 'http://localhost:16686'),
    prometheusUI: import.meta.env.VITE_PROMETHEUS_URL || (isDocker ? 'http://localhost:9090' : 'http://localhost:9090')
  });

  return { meterProvider, tracerProvider };
}

// Error handler for unhandled promise rejections
window.addEventListener('unhandledrejection', (event) => {
  import('./custom-metrics').then(({ recordError }) => {
    recordError(
      new Error(event.reason?.message || 'Unhandled promise rejection'),
      'window.unhandledrejection',
      'high'
    );
  });
});

// Error handler for global errors
window.addEventListener('error', (event) => {
  import('./custom-metrics').then(({ recordError }) => {
    recordError(
      event.error || new Error(event.message),
      'window.error',
      'high'
    );
  });
});