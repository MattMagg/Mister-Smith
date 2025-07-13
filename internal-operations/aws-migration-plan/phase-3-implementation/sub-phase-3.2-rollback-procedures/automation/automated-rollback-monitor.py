#!/usr/bin/env python3
"""
automated-rollback-monitor.py
Monitors system health and triggers rollbacks based on configured thresholds
"""

import json
import yaml
import time
import logging
import subprocess
import requests
import psutil
import threading
from datetime import datetime, timedelta
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum
import grpc
from concurrent import futures

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.FileHandler('/var/log/mistersmith/rollback-monitor.log'),
        logging.StreamHandler()
    ]
)
logger = logging.getLogger('RollbackMonitor')


class HealthStatus(Enum):
    HEALTHY = "healthy"
    DEGRADED = "degraded"
    UNHEALTHY = "unhealthy"
    CRITICAL = "critical"


class ActionType(Enum):
    COMPONENT_ROLLBACK = "component_rollback"
    IMMEDIATE_ROLLBACK = "immediate_rollback"
    EVALUATE_ROLLBACK = "evaluate_rollback"
    ALERT_ONLY = "alert_only"
    COMPONENT_RESTART = "component_restart"


@dataclass
class HealthCheckResult:
    service: str
    status: HealthStatus
    response_time: float
    error_message: Optional[str] = None
    timestamp: datetime = None

    def __post_init__(self):
        if self.timestamp is None:
            self.timestamp = datetime.utcnow()


class RollbackMonitor:
    def __init__(self, config_path: str):
        self.config = self._load_config(config_path)
        self.health_history: Dict[str, List[HealthCheckResult]] = {}
        self.rollback_in_progress = False
        self.monitoring = True
        self.lock = threading.Lock()
        
    def _load_config(self, config_path: str) -> dict:
        """Load rollback trigger configuration"""
        with open(config_path, 'r') as f:
            return yaml.safe_load(f)
    
    def check_orchestrator_health(self) -> HealthCheckResult:
        """Check Orchestrator service health via gRPC"""
        service = "orchestrator"
        start_time = time.time()
        
        try:
            # Use grpc_health_probe command
            result = subprocess.run(
                ['grpc_health_probe', '-addr=localhost:50051'],
                capture_output=True,
                timeout=10
            )
            
            response_time = (time.time() - start_time) * 1000  # ms
            
            if result.returncode == 0:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.HEALTHY,
                    response_time=response_time
                )
            else:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.UNHEALTHY,
                    response_time=response_time,
                    error_message="gRPC health check failed"
                )
                
        except subprocess.TimeoutExpired:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message="Health check timeout"
            )
        except Exception as e:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message=str(e)
            )
    
    def check_webui_health(self) -> HealthCheckResult:
        """Check Web UI service health via HTTP"""
        service = "webui"
        start_time = time.time()
        
        try:
            response = requests.get(
                'http://localhost:3000/health',
                timeout=5
            )
            
            response_time = (time.time() - start_time) * 1000
            
            if response.status_code == 200:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.HEALTHY,
                    response_time=response_time
                )
            else:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.UNHEALTHY,
                    response_time=response_time,
                    error_message=f"HTTP {response.status_code}"
                )
                
        except requests.exceptions.Timeout:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message="Request timeout"
            )
        except Exception as e:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message=str(e)
            )
    
    def check_database_health(self) -> HealthCheckResult:
        """Check PostgreSQL database health"""
        service = "database"
        start_time = time.time()
        
        try:
            result = subprocess.run(
                ['sudo', '-u', 'postgres', 'pg_isready', '-d', 'mistersmith'],
                capture_output=True,
                timeout=30
            )
            
            response_time = (time.time() - start_time) * 1000
            
            if result.returncode == 0:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.HEALTHY,
                    response_time=response_time
                )
            else:
                return HealthCheckResult(
                    service=service,
                    status=HealthStatus.UNHEALTHY,
                    response_time=response_time,
                    error_message="Database not ready"
                )
                
        except subprocess.TimeoutExpired:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message="Database check timeout"
            )
        except Exception as e:
            return HealthCheckResult(
                service=service,
                status=HealthStatus.CRITICAL,
                response_time=(time.time() - start_time) * 1000,
                error_message=str(e)
            )
    
    def check_system_resources(self) -> Dict[str, float]:
        """Check system resource usage"""
        return {
            'cpu_percent': psutil.cpu_percent(interval=1),
            'memory_percent': psutil.virtual_memory().percent,
            'disk_io_percent': self._get_disk_io_usage()
        }
    
    def _get_disk_io_usage(self) -> float:
        """Calculate disk I/O usage percentage"""
        try:
            disk_io = psutil.disk_io_counters()
            # Simplified calculation - in production, track deltas
            return min((disk_io.read_bytes + disk_io.write_bytes) / (1024**3) * 10, 100)
        except:
            return 0.0
    
    def evaluate_health_history(self, service: str, threshold: int) -> bool:
        """Evaluate if service has failed threshold number of checks"""
        with self.lock:
            if service not in self.health_history:
                return False
            
            recent_checks = self.health_history[service][-threshold:]
            
            if len(recent_checks) < threshold:
                return False
            
            failed_count = sum(
                1 for check in recent_checks 
                if check.status in [HealthStatus.UNHEALTHY, HealthStatus.CRITICAL]
            )
            
            return failed_count >= threshold
    
    def trigger_rollback(self, action: Dict, component: Optional[str] = None):
        """Trigger rollback based on action configuration"""
        if self.rollback_in_progress:
            logger.warning("Rollback already in progress, skipping trigger")
            return
        
        with self.lock:
            self.rollback_in_progress = True
        
        try:
            action_type = ActionType(action['type'])
            
            if action_type == ActionType.ALERT_ONLY:
                self._send_alert(f"Alert: {component or 'System'} requires attention", action)
                
            elif action_type == ActionType.COMPONENT_RESTART:
                self._restart_component(component)
                
            elif action_type == ActionType.COMPONENT_ROLLBACK:
                self._rollback_component(component or action.get('target'))
                
            elif action_type == ActionType.IMMEDIATE_ROLLBACK:
                self._immediate_system_rollback()
                
            elif action_type == ActionType.EVALUATE_ROLLBACK:
                self._evaluate_and_rollback(component, action)
                
        finally:
            with self.lock:
                self.rollback_in_progress = False
    
    def _restart_component(self, component: str):
        """Restart a specific component"""
        logger.info(f"Restarting component: {component}")
        
        try:
            # Stop container
            subprocess.run(['docker', 'restart', f'mistersmith-{component}'], check=True)
            
            # Wait for healthy state
            time.sleep(10)
            
            # Verify health
            if component == "orchestrator":
                result = self.check_orchestrator_health()
            elif component == "webui":
                result = self.check_webui_health()
            else:
                result = HealthCheckResult(component, HealthStatus.UNKNOWN, 0)
            
            if result.status == HealthStatus.HEALTHY:
                logger.info(f"Component {component} restarted successfully")
            else:
                logger.error(f"Component {component} still unhealthy after restart")
                self._rollback_component(component)
                
        except Exception as e:
            logger.error(f"Failed to restart {component}: {e}")
            self._rollback_component(component)
    
    def _rollback_component(self, component: str):
        """Execute component-specific rollback"""
        logger.warning(f"Initiating rollback for component: {component}")
        
        script_map = {
            'orchestrator': 'rollback-orchestrator.sh',
            'webui': 'rollback-webui.sh',
            'database': 'rollback-database.sh',
            'nats': 'rollback-nats.sh'
        }
        
        script = script_map.get(component)
        if not script:
            logger.error(f"No rollback script found for {component}")
            return
        
        try:
            result = subprocess.run(
                [f'/path/to/scripts/{script}'],
                capture_output=True,
                text=True
            )
            
            if result.returncode == 0:
                logger.info(f"Rollback completed successfully for {component}")
                self._send_alert(f"Rollback completed for {component}", {'severity': 'info'})
            else:
                logger.error(f"Rollback failed for {component}: {result.stderr}")
                self._send_alert(f"Rollback failed for {component}", {'severity': 'critical'})
                
        except Exception as e:
            logger.error(f"Exception during rollback: {e}")
            self._send_alert(f"Rollback exception for {component}: {e}", {'severity': 'critical'})
    
    def _immediate_system_rollback(self):
        """Execute immediate full system rollback"""
        logger.critical("Initiating immediate system rollback")
        
        try:
            result = subprocess.run(
                ['/path/to/scripts/rollback-all.sh', 'production', '--confirm'],
                capture_output=True,
                text=True
            )
            
            if result.returncode == 0:
                logger.info("System rollback completed successfully")
                self._send_alert("System rollback completed", {'severity': 'warning'})
            else:
                logger.error(f"System rollback failed: {result.stderr}")
                self._send_alert("System rollback failed", {'severity': 'critical'})
                
        except Exception as e:
            logger.error(f"Exception during system rollback: {e}")
            self._send_alert(f"System rollback exception: {e}", {'severity': 'critical'})
    
    def _evaluate_and_rollback(self, component: str, action: Dict):
        """Evaluate conditions and decide on rollback"""
        logger.info(f"Evaluating rollback for {component}")
        
        # Implement evaluation logic based on severity and other conditions
        severity = action.get('severity', 'warning')
        
        if severity == 'critical':
            self._rollback_component(component)
        else:
            self._send_alert(f"Evaluation: {component} may need rollback", action)
    
    def _send_alert(self, message: str, context: Dict):
        """Send alert via configured channels"""
        logger.info(f"Alert: {message}")
        
        # Webhook notification
        webhook_url = self.config.get('notifications', {}).get('webhook_url')
        if webhook_url:
            try:
                requests.post(webhook_url, json={
                    'message': message,
                    'context': context,
                    'timestamp': datetime.utcnow().isoformat()
                })
            except Exception as e:
                logger.error(f"Failed to send webhook: {e}")
    
    def monitor_loop(self):
        """Main monitoring loop"""
        logger.info("Starting rollback monitor")
        
        while self.monitoring:
            try:
                # Health checks
                checks = [
                    self.check_orchestrator_health(),
                    self.check_webui_health(),
                    self.check_database_health()
                ]
                
                # Store results
                for check in checks:
                    with self.lock:
                        if check.service not in self.health_history:
                            self.health_history[check.service] = []
                        self.health_history[check.service].append(check)
                        # Keep only last 100 checks
                        self.health_history[check.service] = self.health_history[check.service][-100:]
                
                # Evaluate triggers
                for service_name, service_config in self.config.get('health_checks', {}).items():
                    if self.evaluate_health_history(service_name, service_config.get('failure_threshold', 3)):
                        logger.warning(f"Service {service_name} exceeded failure threshold")
                        self.trigger_rollback(service_config.get('action', {}), service_name)
                
                # Check resource usage
                resources = self.check_system_resources()
                
                # Evaluate resource triggers
                resource_config = self.config.get('performance', {}).get('resource_usage', {})
                
                if resources['cpu_percent'] > resource_config.get('cpu', {}).get('threshold', 90):
                    logger.warning(f"CPU usage critical: {resources['cpu_percent']}%")
                    
                if resources['memory_percent'] > resource_config.get('memory', {}).get('threshold', 95):
                    logger.warning(f"Memory usage critical: {resources['memory_percent']}%")
                    self.trigger_rollback(resource_config.get('memory', {}).get('action', {}))
                
            except Exception as e:
                logger.error(f"Error in monitor loop: {e}")
            
            # Sleep for interval
            time.sleep(30)  # Default 30 second check interval
    
    def start(self):
        """Start the monitoring service"""
        monitor_thread = threading.Thread(target=self.monitor_loop)
        monitor_thread.start()
        
        try:
            monitor_thread.join()
        except KeyboardInterrupt:
            logger.info("Shutting down rollback monitor")
            self.monitoring = False
            monitor_thread.join()


if __name__ == "__main__":
    import sys
    
    config_path = sys.argv[1] if len(sys.argv) > 1 else "/path/to/rollback-triggers.yaml"
    
    monitor = RollbackMonitor(config_path)
    monitor.start()