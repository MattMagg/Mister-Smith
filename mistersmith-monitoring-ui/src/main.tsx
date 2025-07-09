import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
// import App from './App.tsx'
import App from './AppMinimal.tsx'
// import App from './test-telemetry.tsx'
// import App from './AppBasic.tsx'
// import { initializeOpenTelemetry } from './lib/telemetry/otel-setup'
// import { initializeSimpleOtel } from './lib/telemetry/simple-otel-setup'

// Initialize OpenTelemetry before app starts
console.log('Starting app initialization...');

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
