// Test file to debug Discovery import issue
import { Discovery } from './types/discovery';

const testDiscovery: Discovery = {
  id: 'test',
  type: 'pattern',
  content: 'test content',
  confidence: 0.8,
  agentId: 'test-agent',
  timestamp: new Date(),
};

console.log('Discovery test:', testDiscovery);