# Message Schema Definitions

## Agent Framework Message Type System

> **Integration Foundation**: This document defines comprehensive message schemas that integrate with transport layer specifications and agent orchestration patterns

## Executive Summary

This document provides complete JSON Schema definitions for all message types in the Mister Smith AI Agent Framework. It establishes message versioning, validation rules, serialization formats, routing schemes, and transformation patterns to enable type-safe, performant agent communication.

## 1. Foundation Schemas

### 1.1 Base Message Envelope

All messages in the framework inherit from this base schema to ensure consistent structure and metadata handling.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/base-message.json",
  "title": "Base Message Envelope",
  "description": "Universal message envelope for all framework communications",
  "type": "object",
  "required": ["message_id", "timestamp", "schema_version", "message_type"],
  "properties": {
    "message_id": {
      "type": "string",
      "format": "uuid",
      "description": "Unique message identifier (UUID v4)"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time",
      "description": "Message creation timestamp (ISO 8601 RFC 3339)"
    },
    "schema_version": {
      "type": "string",
      "pattern": "^\\d+\\.\\d+\\.\\d+$",
      "description": "Schema version (semantic versioning)",
      "default": "1.0.0"
    },
    "message_type": {
      "type": "string",
      "description": "Discriminator for message type routing and validation"
    },
    "correlation_id": {
      "type": "string",
      "format": "uuid",
      "description": "Optional correlation identifier for request-response patterns"
    },
    "trace_id": {
      "type": "string",
      "format": "uuid",
      "description": "Distributed tracing identifier"
    },
    "source_agent_id": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+$",
      "description": "Identifier of the sending agent"
    },
    "target_agent_id": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+$",
      "description": "Identifier of the intended recipient agent"
    },
    "priority": {
      "type": "integer",
      "minimum": 1,
      "maximum": 10,
      "default": 5,
      "description": "Message priority (1=highest, 10=lowest)"
    },
    "reply_to": {
      "type": "string",
      "description": "NATS subject for reply messages"
    },
    "timeout_ms": {
      "type": "integer",
      "minimum": 1000,
      "description": "Message processing timeout in milliseconds"
    },
    "metadata": {
      "type": "object",
      "additionalProperties": {
        "type": "string"
      },
      "description": "Extensible metadata container"
    }
  },
  "additionalProperties": false
}
```

### 1.2 Common Type Definitions

Reusable schema components used across multiple message types.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/common-types.json",
  "title": "Common Type Definitions",
  "$defs": {
    "agent_id": {
      "type": "string",
      "pattern": "^[a-zA-Z0-9_-]+$",
      "minLength": 1,
      "maxLength": 64,
      "description": "Agent identifier following naming conventions"
    },
    "task_id": {
      "type": "string",
      "format": "uuid",
      "description": "Task identifier (UUID v4)"
    },
    "agent_status": {
      "type": "string",
      "enum": ["initializing", "idle", "busy", "paused", "error", "stopping", "terminated"],
      "description": "Agent operational status"
    },
    "task_status": {
      "type": "string",
      "enum": ["pending", "assigned", "running", "completed", "failed", "cancelled", "timeout"],
      "description": "Task execution status"
    },
    "capability": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9-]*[a-z0-9]$",
      "description": "Agent capability identifier (kebab-case)"
    },
    "resource_usage": {
      "type": "object",
      "properties": {
        "cpu_percent": {
          "type": "number",
          "minimum": 0,
          "maximum": 100,
          "description": "CPU utilization percentage"
        },
        "memory_mb": {
          "type": "number",
          "minimum": 0,
          "description": "Memory usage in megabytes"
        },
        "disk_mb": {
          "type": "number",
          "minimum": 0,
          "description": "Disk usage in megabytes"
        },
        "network_connections": {
          "type": "integer",
          "minimum": 0,
          "description": "Active network connections"
        }
      },
      "additionalProperties": false
    },
    "error_details": {
      "type": "object",
      "required": ["error_code", "error_message"],
      "properties": {
        "error_code": {
          "type": "string",
          "pattern": "^E\\d{4}$",
          "description": "Standardized error code (E####)"
        },
        "error_message": {
          "type": "string",
          "maxLength": 2048,
          "description": "Human-readable error description"
        },
        "stack_trace": {
          "type": "string",
          "description": "Technical stack trace for debugging"
        },
        "retry_count": {
          "type": "integer",
          "minimum": 0,
          "description": "Number of retry attempts"
        },
        "recoverable": {
          "type": "boolean",
          "description": "Whether the error is potentially recoverable"
        },
        "context": {
          "type": "object",
          "description": "Additional error context"
        }
      },
      "additionalProperties": false
    },
    "execution_metrics": {
      "type": "object",
      "properties": {
        "execution_time_ms": {
          "type": "integer",
          "minimum": 0,
          "description": "Total execution time in milliseconds"
        },
        "cpu_time_ms": {
          "type": "integer",
          "minimum": 0,
          "description": "CPU time consumed in milliseconds"
        },
        "memory_peak_mb": {
          "type": "number",
          "minimum": 0,
          "description": "Peak memory usage during execution"
        },
        "io_operations": {
          "type": "integer",
          "minimum": 0,
          "description": "Number of I/O operations performed"
        },
        "network_bytes_sent": {
          "type": "integer",
          "minimum": 0,
          "description": "Network bytes transmitted"
        },
        "network_bytes_received": {
          "type": "integer",
          "minimum": 0,
          "description": "Network bytes received"
        }
      },
      "additionalProperties": false
    }
  }
}
```

## 2. Agent Communication Messages

### 2.1 Agent Command Message

Schema for commanding agents to perform operations.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/agent-command.json",
  "title": "Agent Command Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "agent_command" },
    "payload": {
      "type": "object",
      "required": ["command_type"],
      "properties": {
        "command_type": {
          "type": "string",
          "enum": ["execute", "stop", "pause", "resume", "configure", "shutdown", "restart"],
          "description": "Type of command to execute"
        },
        "command_data": {
          "type": "object",
          "description": "Command-specific data payload"
        },
        "parameters": {
          "type": "object",
          "additionalProperties": true,
          "description": "Command execution parameters"
        },
        "require_confirmation": {
          "type": "boolean",
          "default": false,
          "description": "Whether command requires explicit confirmation"
        },
        "idempotency_key": {
          "type": "string",
          "description": "Key for idempotent command execution"
        }
      },
      "additionalProperties": false,
      "if": {
        "properties": {
          "command_type": { "const": "execute" }
        }
      },
      "then": {
        "properties": {
          "command_data": {
            "type": "object",
            "required": ["task"],
            "properties": {
              "task": { "$ref": "common-types.json#/$defs/task_id" },
              "context": { "type": "object" },
              "deadline": {
                "type": "string",
                "format": "date-time"
              }
            }
          }
        }
      }
    }
  },
  "additionalProperties": false
}
```

### 2.2 Agent Status Update Message

Schema for agent status reporting and health monitoring.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/agent-status.json",
  "title": "Agent Status Update Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "agent_status" },
    "payload": {
      "type": "object",
      "required": ["agent_id", "status", "uptime_seconds"],
      "properties": {
        "agent_id": { "$ref": "common-types.json#/$defs/agent_id" },
        "status": { "$ref": "common-types.json#/$defs/agent_status" },
        "previous_status": { "$ref": "common-types.json#/$defs/agent_status" },
        "uptime_seconds": {
          "type": "integer",
          "minimum": 0,
          "description": "Agent uptime in seconds"
        },
        "capacity": {
          "type": "number",
          "minimum": 0.0,
          "maximum": 1.0,
          "description": "Current capacity utilization (0.0-1.0)"
        },
        "current_tasks": {
          "type": "integer",
          "minimum": 0,
          "description": "Number of currently executing tasks"
        },
        "max_tasks": {
          "type": "integer",
          "minimum": 1,
          "description": "Maximum concurrent task capacity"
        },
        "capabilities": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/capability" },
          "uniqueItems": true,
          "description": "Agent capabilities list"
        },
        "resource_usage": { "$ref": "common-types.json#/$defs/resource_usage" },
        "last_heartbeat": {
          "type": "string",
          "format": "date-time",
          "description": "Last heartbeat timestamp"
        },
        "health_check_results": {
          "type": "object",
          "properties": {
            "overall_health": {
              "type": "string",
              "enum": ["healthy", "degraded", "unhealthy"],
              "description": "Overall health status"
            },
            "component_health": {
              "type": "object",
              "patternProperties": {
                "^[a-zA-Z0-9_-]+$": {
                  "type": "string",
                  "enum": ["healthy", "degraded", "unhealthy"]
                }
              },
              "description": "Per-component health status"
            }
          }
        },
        "error_details": { "$ref": "common-types.json#/$defs/error_details" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 2.3 Agent Registration Message

Schema for agent registration and capability announcement.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/agent-registration.json",
  "title": "Agent Registration Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "agent_registration" },
    "payload": {
      "type": "object",
      "required": ["agent_id", "agent_type", "capabilities", "max_tasks"],
      "properties": {
        "agent_id": { "$ref": "common-types.json#/$defs/agent_id" },
        "agent_type": {
          "type": "string",
          "enum": ["supervisor", "worker", "coordinator", "monitor", "planner", "executor", "critic", "router", "memory"],
          "description": "Agent type classification"
        },
        "agent_version": {
          "type": "string",
          "pattern": "^\\d+\\.\\d+\\.\\d+$",
          "description": "Agent implementation version"
        },
        "capabilities": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/capability" },
          "minItems": 1,
          "uniqueItems": true,
          "description": "Agent capabilities list"
        },
        "max_tasks": {
          "type": "integer",
          "minimum": 1,
          "maximum": 1000,
          "description": "Maximum concurrent task capacity"
        },
        "resource_requirements": {
          "type": "object",
          "properties": {
            "min_memory_mb": {
              "type": "integer",
              "minimum": 256,
              "description": "Minimum memory requirement"
            },
            "max_memory_mb": {
              "type": "integer",
              "description": "Maximum memory limit"
            },
            "cpu_cores": {
              "type": "number",
              "minimum": 0.1,
              "description": "CPU core requirement"
            }
          }
        },
        "configuration": {
          "type": "object",
          "description": "Agent-specific configuration parameters"
        },
        "endpoints": {
          "type": "object",
          "properties": {
            "nats_subjects": {
              "type": "array",
              "items": { "type": "string" },
              "description": "NATS subjects the agent subscribes to"
            },
            "grpc_address": {
              "type": "string",
              "description": "gRPC service address"
            },
            "http_address": {
              "type": "string",
              "description": "HTTP API address"
            }
          }
        },
        "supervision": {
          "type": "object",
          "properties": {
            "supervisor_id": { "$ref": "common-types.json#/$defs/agent_id" },
            "restart_policy": {
              "type": "string",
              "enum": ["always", "on-failure", "never"],
              "default": "on-failure"
            },
            "health_check_interval_ms": {
              "type": "integer",
              "minimum": 1000,
              "default": 30000
            }
          }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## 3. Task Management Messages

### 3.1 Task Assignment Message

Schema for assigning tasks to agents.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/task-assignment.json",
  "title": "Task Assignment Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "task_assignment" },
    "payload": {
      "type": "object",
      "required": ["task_id", "task_type", "assigned_agent", "created_at"],
      "properties": {
        "task_id": { "$ref": "common-types.json#/$defs/task_id" },
        "task_type": {
          "type": "string",
          "enum": ["analysis", "synthesis", "execution", "validation", "monitoring", "planning", "coordination"],
          "description": "Type of task to be executed"
        },
        "assigned_agent": { "$ref": "common-types.json#/$defs/agent_id" },
        "created_at": {
          "type": "string",
          "format": "date-time",
          "description": "Task creation timestamp"
        },
        "deadline": {
          "type": "string",
          "format": "date-time",
          "description": "Task completion deadline"
        },
        "priority": {
          "type": "integer",
          "minimum": 1,
          "maximum": 10,
          "default": 5,
          "description": "Task priority (1=highest, 10=lowest)"
        },
        "requirements": {
          "type": "object",
          "properties": {
            "capabilities": {
              "type": "array",
              "items": { "$ref": "common-types.json#/$defs/capability" },
              "description": "Required agent capabilities"
            },
            "resources": {
              "type": "object",
              "properties": {
                "cpu_cores": {
                  "type": "number",
                  "minimum": 0.1,
                  "description": "Required CPU cores"
                },
                "memory_mb": {
                  "type": "integer",
                  "minimum": 256,
                  "description": "Required memory in MB"
                },
                "disk_mb": {
                  "type": "integer",
                  "minimum": 100,
                  "description": "Required disk space in MB"
                },
                "network_bandwidth_mbps": {
                  "type": "number",
                  "minimum": 0,
                  "description": "Required network bandwidth"
                }
              }
            },
            "security": {
              "type": "object",
              "properties": {
                "security_level": {
                  "type": "string",
                  "enum": ["public", "internal", "confidential", "restricted"],
                  "default": "internal"
                },
                "required_permissions": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "Required security permissions"
                },
                "requires_encryption": {
                  "type": "boolean",
                  "default": false,
                  "description": "Whether task requires encrypted communication"
                }
              }
            }
          }
        },
        "task_data": {
          "type": "object",
          "description": "Task-specific input data and parameters"
        },
        "dependencies": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/task_id" },
          "uniqueItems": true,
          "description": "Task dependencies that must complete first"
        },
        "callback_subjects": {
          "type": "object",
          "properties": {
            "progress": { "type": "string" },
            "completion": { "type": "string" },
            "error": { "type": "string" }
          },
          "description": "NATS subjects for task lifecycle callbacks"
        },
        "retry_policy": {
          "type": "object",
          "properties": {
            "max_retries": {
              "type": "integer",
              "minimum": 0,
              "maximum": 10,
              "default": 3
            },
            "backoff_strategy": {
              "type": "string",
              "enum": ["fixed", "linear", "exponential"],
              "default": "exponential"
            },
            "initial_delay_ms": {
              "type": "integer",
              "minimum": 100,
              "default": 1000
            },
            "max_delay_ms": {
              "type": "integer",
              "minimum": 1000,
              "default": 30000
            }
          }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 3.2 Task Result Message

Schema for reporting task execution results.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/task-result.json",
  "title": "Task Result Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "task_result" },
    "payload": {
      "type": "object",
      "required": ["task_id", "agent_id", "status", "completion_time"],
      "properties": {
        "task_id": { "$ref": "common-types.json#/$defs/task_id" },
        "agent_id": { "$ref": "common-types.json#/$defs/agent_id" },
        "status": { "$ref": "common-types.json#/$defs/task_status" },
        "completion_time": {
          "type": "string",
          "format": "date-time",
          "description": "Task completion timestamp"
        },
        "result_data": {
          "type": "object",
          "description": "Task execution results and output data"
        },
        "partial_results": {
          "type": "array",
          "items": {
            "type": "object",
            "properties": {
              "timestamp": {
                "type": "string",
                "format": "date-time"
              },
              "data": { "type": "object" },
              "percentage_complete": {
                "type": "number",
                "minimum": 0,
                "maximum": 100
              }
            }
          },
          "description": "Intermediate results during task execution"
        },
        "error_details": { "$ref": "common-types.json#/$defs/error_details" },
        "metrics": { "$ref": "common-types.json#/$defs/execution_metrics" },
        "artifacts": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["artifact_id", "artifact_type"],
            "properties": {
              "artifact_id": {
                "type": "string",
                "format": "uuid"
              },
              "artifact_type": {
                "type": "string",
                "enum": ["file", "data", "log", "report", "model"]
              },
              "location": {
                "type": "string",
                "description": "Artifact storage location"
              },
              "size_bytes": {
                "type": "integer",
                "minimum": 0
              },
              "checksum": {
                "type": "string",
                "description": "SHA-256 checksum"
              },
              "metadata": {
                "type": "object",
                "description": "Artifact-specific metadata"
              }
            }
          },
          "description": "Generated artifacts and outputs"
        },
        "follow_up_tasks": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/task_id" },
          "description": "Tasks generated as a result of this task"
        },
        "quality_assessment": {
          "type": "object",
          "properties": {
            "score": {
              "type": "number",
              "minimum": 0,
              "maximum": 1,
              "description": "Quality score (0-1)"
            },
            "criteria": {
              "type": "object",
              "patternProperties": {
                "^[a-zA-Z0-9_-]+$": {
                  "type": "number",
                  "minimum": 0,
                  "maximum": 1
                }
              },
              "description": "Individual quality criteria scores"
            },
            "feedback": {
              "type": "string",
              "description": "Qualitative feedback on results"
            }
          }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 3.3 Task Progress Update Message

Schema for reporting task execution progress.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/task-progress.json",
  "title": "Task Progress Update Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "task_progress" },
    "payload": {
      "type": "object",
      "required": ["task_id", "agent_id", "progress_percentage", "update_time"],
      "properties": {
        "task_id": { "$ref": "common-types.json#/$defs/task_id" },
        "agent_id": { "$ref": "common-types.json#/$defs/agent_id" },
        "progress_percentage": {
          "type": "number",
          "minimum": 0,
          "maximum": 100,
          "description": "Task completion percentage"
        },
        "update_time": {
          "type": "string",
          "format": "date-time",
          "description": "Progress update timestamp"
        },
        "current_phase": {
          "type": "string",
          "description": "Current execution phase or step"
        },
        "phase_description": {
          "type": "string",
          "maxLength": 512,
          "description": "Human-readable phase description"
        },
        "estimated_completion": {
          "type": "string",
          "format": "date-time",
          "description": "Estimated completion time"
        },
        "interim_results": {
          "type": "object",
          "description": "Preliminary results available at this point"
        },
        "resource_usage": { "$ref": "common-types.json#/$defs/resource_usage" },
        "blockers": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["blocker_type", "description"],
            "properties": {
              "blocker_type": {
                "type": "string",
                "enum": ["dependency", "resource", "permission", "data", "external"],
                "description": "Type of blocking issue"
              },
              "description": {
                "type": "string",
                "maxLength": 1024,
                "description": "Blocker description"
              },
              "severity": {
                "type": "string",
                "enum": ["low", "medium", "high", "critical"],
                "description": "Blocker severity level"
              },
              "estimated_resolution": {
                "type": "string",
                "format": "date-time",
                "description": "Estimated blocker resolution time"
              }
            }
          },
          "description": "Current execution blockers"
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## 4. Workflow Orchestration Messages

### 4.1 Workflow Coordination Message

Schema for coordinating complex multi-agent workflows.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/workflow-coordination.json",
  "title": "Workflow Coordination Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "workflow_coordination" },
    "payload": {
      "type": "object",
      "required": ["workflow_id", "coordination_type", "participants"],
      "properties": {
        "workflow_id": {
          "type": "string",
          "format": "uuid",
          "description": "Workflow instance identifier"
        },
        "coordination_type": {
          "type": "string",
          "enum": ["sync", "barrier", "checkpoint", "rollback", "commit", "abort"],
          "description": "Type of coordination action"
        },
        "participants": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/agent_id" },
          "minItems": 1,
          "uniqueItems": true,
          "description": "Participating agents in coordination"
        },
        "coordination_data": {
          "type": "object",
          "description": "Coordination-specific data"
        },
        "timeout_ms": {
          "type": "integer",
          "minimum": 1000,
          "description": "Coordination timeout in milliseconds"
        },
        "required_confirmations": {
          "type": "integer",
          "minimum": 1,
          "description": "Number of required confirmations"
        },
        "compensation_actions": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["action_type", "target_agent"],
            "properties": {
              "action_type": {
                "type": "string",
                "enum": ["undo", "rollback", "cleanup", "notify"]
              },
              "target_agent": { "$ref": "common-types.json#/$defs/agent_id" },
              "action_data": { "type": "object" }
            }
          },
          "description": "Compensation actions for failure scenarios"
        },
        "dependency_graph": {
          "type": "object",
          "patternProperties": {
            "^[a-zA-Z0-9_-]+$": {
              "type": "array",
              "items": { "$ref": "common-types.json#/$defs/agent_id" }
            }
          },
          "description": "Agent dependency relationships"
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 4.2 Workflow State Synchronization Message

Schema for synchronizing workflow state across agents.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/workflow-state-sync.json",
  "title": "Workflow State Synchronization Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "workflow_state_sync" },
    "payload": {
      "type": "object",
      "required": ["workflow_id", "state_version", "state_data"],
      "properties": {
        "workflow_id": {
          "type": "string",
          "format": "uuid",
          "description": "Workflow instance identifier"
        },
        "state_version": {
          "type": "integer",
          "minimum": 1,
          "description": "State version number for conflict resolution"
        },
        "state_data": {
          "type": "object",
          "required": ["workflow_status", "agent_states"],
          "properties": {
            "workflow_status": {
              "type": "string",
              "enum": ["initializing", "running", "suspended", "completed", "failed", "aborted"],
              "description": "Overall workflow status"
            },
            "agent_states": {
              "type": "object",
              "patternProperties": {
                "^[a-zA-Z0-9_-]+$": {
                  "type": "object",
                  "properties": {
                    "status": { "$ref": "common-types.json#/$defs/agent_status" },
                    "current_task": { "$ref": "common-types.json#/$defs/task_id" },
                    "completed_tasks": {
                      "type": "array",
                      "items": { "$ref": "common-types.json#/$defs/task_id" }
                    },
                    "local_state": { "type": "object" }
                  }
                }
              },
              "description": "Per-agent state information"
            },
            "shared_state": {
              "type": "object",
              "description": "Shared workflow state accessible to all agents"
            },
            "execution_context": {
              "type": "object",
              "properties": {
                "started_at": {
                  "type": "string",
                  "format": "date-time"
                },
                "last_checkpoint": {
                  "type": "string",
                  "format": "date-time"
                },
                "estimated_completion": {
                  "type": "string",
                  "format": "date-time"
                }
              }
            }
          }
        },
        "checksum": {
          "type": "string",
          "description": "State data integrity checksum"
        },
        "diff": {
          "type": "object",
          "description": "State changes since last synchronization"
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## 5. Claude CLI Integration Messages

### 5.1 Hook Event Message

Schema for Claude CLI hook events and integration points.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/hook-event.json",
  "title": "Claude CLI Hook Event Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "hook_event" },
    "payload": {
      "type": "object",
      "required": ["hook_type", "session_info"],
      "properties": {
        "hook_type": {
          "type": "string",
          "enum": ["startup", "pre_task", "post_task", "on_error", "on_file_change", "shutdown"],
          "description": "Type of hook event"
        },
        "tool_name": {
          "type": "string",
          "description": "Name of the tool being executed (for task hooks)"
        },
        "tool_input": {
          "type": "object",
          "description": "Tool input parameters (for task hooks)"
        },
        "tool_response": {
          "type": "object",
          "description": "Tool execution response (for post-task hooks)"
        },
        "session_info": {
          "type": "object",
          "required": ["session_id", "model", "start_time"],
          "properties": {
            "session_id": {
              "type": "string",
              "format": "uuid",
              "description": "Claude CLI session identifier"
            },
            "model": {
              "type": "string",
              "description": "Claude model being used"
            },
            "start_time": {
              "type": "string",
              "format": "date-time",
              "description": "Session start time"
            },
            "user_id": {
              "type": "string",
              "description": "User identifier"
            },
            "workspace_path": {
              "type": "string",
              "description": "Working directory path"
            }
          },
          "additionalProperties": false
        },
        "context_id": {
          "type": "string",
          "format": "uuid",
          "description": "Contextual grouping identifier"
        },
        "file_changes": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["file_path", "change_type"],
            "properties": {
              "file_path": {
                "type": "string",
                "description": "Path to changed file"
              },
              "change_type": {
                "type": "string",
                "enum": ["created", "modified", "deleted", "renamed"],
                "description": "Type of file change"
              },
              "old_path": {
                "type": "string",
                "description": "Previous path (for renamed files)"
              },
              "content_preview": {
                "type": "string",
                "maxLength": 2048,
                "description": "Preview of file content changes"
              }
            }
          },
          "description": "File changes (for file change hooks)"
        },
        "error_context": {
          "type": "object",
          "properties": {
            "error_message": {
              "type": "string",
              "description": "Error message"
            },
            "stack_trace": {
              "type": "string",
              "description": "Error stack trace"
            },
            "recovery_suggestions": {
              "type": "array",
              "items": { "type": "string" },
              "description": "Suggested recovery actions"
            }
          },
          "description": "Error context (for error hooks)"
        },
        "agent_spawn_request": {
          "type": "object",
          "properties": {
            "requested_agents": {
              "type": "array",
              "items": {
                "type": "object",
                "required": ["agent_type", "capabilities"],
                "properties": {
                  "agent_type": {
                    "type": "string",
                    "enum": ["planner", "executor", "critic", "router", "memory", "coordinator"]
                  },
                  "capabilities": {
                    "type": "array",
                    "items": { "$ref": "common-types.json#/$defs/capability" }
                  },
                  "configuration": { "type": "object" },
                  "priority": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 5
                  }
                }
              },
              "description": "Agents requested for parallel execution"
            },
            "coordination_strategy": {
              "type": "string",
              "enum": ["parallel", "sequential", "pipeline", "hierarchical"],
              "default": "parallel",
              "description": "How agents should be coordinated"
            }
          },
          "description": "Request for parallel agent spawning"
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 5.2 Hook Response Message

Schema for responding to Claude CLI hook events.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/hook-response.json",
  "title": "Claude CLI Hook Response Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "hook_response" },
    "payload": {
      "type": "object",
      "required": ["hook_event_id", "decision"],
      "properties": {
        "hook_event_id": {
          "type": "string",
          "format": "uuid",
          "description": "ID of the hook event being responded to"
        },
        "decision": {
          "type": "string",
          "enum": ["approve", "block", "continue", "modify"],
          "description": "Hook processing decision"
        },
        "reason": {
          "type": "string",
          "maxLength": 1024,
          "description": "Reason for the decision"
        },
        "continue": {
          "type": "boolean",
          "default": true,
          "description": "Whether to continue normal execution"
        },
        "stop_reason": {
          "type": "string",
          "description": "Reason for stopping execution (if continue=false)"
        },
        "modifications": {
          "type": "object",
          "properties": {
            "tool_input_changes": {
              "type": "object",
              "description": "Modifications to tool input parameters"
            },
            "additional_context": {
              "type": "object",
              "description": "Additional context to inject"
            },
            "environment_changes": {
              "type": "object",
              "description": "Environment variable modifications"
            },
            "configuration_overrides": {
              "type": "object",
              "description": "Configuration parameter overrides"
            }
          },
          "description": "Modifications to apply (for modify decision)"
        },
        "spawned_agents": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["agent_id", "agent_type", "status"],
            "properties": {
              "agent_id": { "$ref": "common-types.json#/$defs/agent_id" },
              "agent_type": {
                "type": "string",
                "enum": ["planner", "executor", "critic", "router", "memory", "coordinator"]
              },
              "status": {
                "type": "string",
                "enum": ["spawning", "ready", "failed"]
              },
              "endpoints": {
                "type": "object",
                "properties": {
                  "nats_subject": { "type": "string" },
                  "grpc_address": { "type": "string" },
                  "http_address": { "type": "string" }
                }
              },
              "error_details": { "$ref": "common-types.json#/$defs/error_details" }
            }
          },
          "description": "Information about spawned agents"
        },
        "execution_context": {
          "type": "object",
          "properties": {
            "parallel_task_results": {
              "type": "object",
              "patternProperties": {
                "^[a-zA-Z0-9_-]+$": {
                  "type": "object",
                  "properties": {
                    "status": { "$ref": "common-types.json#/$defs/task_status" },
                    "result": { "type": "object" },
                    "metrics": { "$ref": "common-types.json#/$defs/execution_metrics" }
                  }
                }
              },
              "description": "Results from parallel agent execution"
            },
            "coordination_summary": {
              "type": "object",
              "properties": {
                "total_agents": { "type": "integer" },
                "successful_agents": { "type": "integer" },
                "failed_agents": { "type": "integer" },
                "total_execution_time_ms": { "type": "integer" }
              }
            }
          },
          "description": "Context from parallel execution"
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## 6. System Operation Messages

### 6.1 System Alert Message

Schema for system alerts and notifications.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/system-alert.json",
  "title": "System Alert Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "system_alert" },
    "payload": {
      "type": "object",
      "required": ["alert_id", "severity", "component", "alert_type", "alert_message"],
      "properties": {
        "alert_id": {
          "type": "string",
          "format": "uuid",
          "description": "Unique alert identifier"
        },
        "severity": {
          "type": "string",
          "enum": ["info", "warning", "error", "critical", "emergency"],
          "description": "Alert severity level"
        },
        "component": {
          "type": "string",
          "description": "System component that generated the alert"
        },
        "alert_type": {
          "type": "string",
          "enum": ["health", "performance", "security", "resource", "operational", "business"],
          "description": "Category of alert"
        },
        "alert_message": {
          "type": "string",
          "maxLength": 2048,
          "description": "Human-readable alert message"
        },
        "details": {
          "type": "object",
          "description": "Additional alert details and context"
        },
        "affected_agents": {
          "type": "array",
          "items": { "$ref": "common-types.json#/$defs/agent_id" },
          "description": "Agents affected by this alert"
        },
        "metrics": {
          "type": "object",
          "patternProperties": {
            "^[a-zA-Z0-9_.-]+$": {
              "type": "number"
            }
          },
          "description": "Relevant metrics at time of alert"
        },
        "resolution_steps": {
          "type": "array",
          "items": {
            "type": "object",
            "required": ["step", "description"],
            "properties": {
              "step": { "type": "integer" },
              "description": { "type": "string" },
              "automated": { "type": "boolean" },
              "estimated_time_minutes": { "type": "integer" }
            }
          },
          "description": "Suggested resolution steps"
        },
        "auto_resolution": {
          "type": "object",
          "properties": {
            "enabled": { "type": "boolean" },
            "action": {
              "type": "string",
              "enum": ["restart", "scale", "failover", "throttle", "ignore"]
            },
            "parameters": { "type": "object" }
          },
          "description": "Automatic resolution configuration"
        },
        "escalation": {
          "type": "object",
          "properties": {
            "escalate_after_minutes": { "type": "integer" },
            "escalation_targets": {
              "type": "array",
              "items": { "type": "string" }
            },
            "notification_channels": {
              "type": "array",
              "items": {
                "type": "string",
                "enum": ["email", "slack", "webhook", "sms", "pager"]
              }
            }
          }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

### 6.2 System Health Check Message

Schema for system health monitoring and reporting.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/system-health-check.json",
  "title": "System Health Check Message",
  "allOf": [
    { "$ref": "base-message.json" }
  ],
  "properties": {
    "message_type": { "const": "system_health_check" },
    "payload": {
      "type": "object",
      "required": ["check_id", "check_type", "overall_status", "components"],
      "properties": {
        "check_id": {
          "type": "string",
          "format": "uuid",
          "description": "Health check instance identifier"
        },
        "check_type": {
          "type": "string",
          "enum": ["scheduled", "on_demand", "triggered", "startup", "shutdown"],
          "description": "Type of health check"
        },
        "overall_status": {
          "type": "string",
          "enum": ["healthy", "degraded", "unhealthy", "unknown"],
          "description": "Overall system health status"
        },
        "components": {
          "type": "object",
          "patternProperties": {
            "^[a-zA-Z0-9_-]+$": {
              "type": "object",
              "required": ["status"],
              "properties": {
                "status": {
                  "type": "string",
                  "enum": ["healthy", "degraded", "unhealthy", "unknown"]
                },
                "response_time_ms": {
                  "type": "number",
                  "minimum": 0,
                  "description": "Component response time"
                },
                "last_check": {
                  "type": "string",
                  "format": "date-time"
                },
                "error_details": { "$ref": "common-types.json#/$defs/error_details" },
                "metrics": {
                  "type": "object",
                  "properties": {
                    "uptime_seconds": { "type": "integer" },
                    "request_rate": { "type": "number" },
                    "error_rate": { "type": "number" },
                    "latency_p95_ms": { "type": "number" }
                  }
                },
                "dependencies": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "Component dependencies"
                }
              }
            }
          },
          "description": "Per-component health status"
        },
        "infrastructure": {
          "type": "object",
          "properties": {
            "nats_cluster": {
              "type": "object",
              "properties": {
                "cluster_size": { "type": "integer" },
                "healthy_nodes": { "type": "integer" },
                "leader_node": { "type": "string" },
                "message_rate": { "type": "number" },
                "storage_usage_percent": { "type": "number" }
              }
            },
            "database": {
              "type": "object",
              "properties": {
                "connection_pool_status": { "type": "string" },
                "active_connections": { "type": "integer" },
                "query_latency_ms": { "type": "number" },
                "storage_usage_percent": { "type": "number" }
              }
            },
            "resources": {
              "type": "object",
              "properties": {
                "cpu_usage_percent": { "type": "number" },
                "memory_usage_percent": { "type": "number" },
                "disk_usage_percent": { "type": "number" },
                "network_utilization_percent": { "type": "number" }
              }
            }
          }
        },
        "sla_compliance": {
          "type": "object",
          "properties": {
            "availability_percent": {
              "type": "number",
              "minimum": 0,
              "maximum": 100
            },
            "response_time_sla_ms": { "type": "number" },
            "error_rate_threshold_percent": { "type": "number" },
            "compliance_status": {
              "type": "string",
              "enum": ["compliant", "at_risk", "breached"]
            }
          }
        }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false
}
```

## 7. Validation Framework

### 7.1 Validation Rules and Levels

The framework supports multiple validation levels to balance performance and safety:

```yaml
validation_levels:
  strict:
    description: "Full schema validation with all constraints"
    performance: "Slower but comprehensive"
    use_cases: ["development", "testing", "critical_operations"]
    
  standard:
    description: "Essential validation with performance optimization"
    performance: "Balanced performance and safety"
    use_cases: ["production", "normal_operations"]
    
  permissive:
    description: "Minimal validation for high-throughput scenarios"
    performance: "Fastest validation"
    use_cases: ["high_frequency_messages", "monitoring_data"]

validation_configuration:
  schema_cache_size: 1000
  schema_cache_ttl_seconds: 3600
  max_validation_errors: 100
  fail_fast: false
  error_aggregation: true
  format_validation: true
  content_encoding_validation: true
```

### 7.2 Error Code Classification

Standardized error codes for message validation failures:

```json
{
  "validation_errors": {
    "V1001": {
      "description": "Schema validation failed",
      "severity": "error",
      "retryable": false
    },
    "V1002": {
      "description": "Required field missing",
      "severity": "error",
      "retryable": false
    },
    "V1003": {
      "description": "Invalid field format",
      "severity": "error", 
      "retryable": false
    },
    "V1004": {
      "description": "Field value out of range",
      "severity": "error",
      "retryable": false
    },
    "V1005": {
      "description": "Invalid enum value",
      "severity": "error",
      "retryable": false
    },
    "V2001": {
      "description": "Message too large",
      "severity": "warning",
      "retryable": false
    },
    "V2002": {
      "description": "Deprecated field used",
      "severity": "warning", 
      "retryable": true
    },
    "V3001": {
      "description": "Schema version incompatible",
      "severity": "error",
      "retryable": false
    },
    "V3002": {
      "description": "Unknown message type",
      "severity": "error",
      "retryable": false
    }
  }
}
```

### 7.3 Performance Optimization

Validation performance optimizations and caching strategies:

```rust
// Example validation configuration in Rust
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub level: ValidationLevel,
    pub schema_cache_size: usize,
    pub enable_fast_path: bool,
    pub max_validation_errors: usize,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub enum ValidationLevel {
    Strict,    // Full validation
    Standard,  // Balanced validation
    Permissive // Minimal validation
}

// Fast-path validation for high-frequency message types
pub struct FastPathValidator {
    compiled_schemas: HashMap<String, CompiledSchema>,
    performance_cache: LruCache<String, ValidationResult>,
}
```

## 8. Serialization Specifications

### 8.1 JSON Serialization Standards

```json
{
  "json_serialization": {
    "encoding": "UTF-8",
    "date_format": "ISO 8601 (RFC 3339)",
    "number_precision": "IEEE 754 double precision",
    "string_max_length": 1048576,
    "object_max_depth": 32,
    "array_max_length": 10000,
    "null_handling": "explicit",
    "pretty_print": false,
    "escape_unicode": false
  },
  "compression": {
    "algorithm": "gzip",
    "level": 6,
    "threshold_bytes": 1024,
    "content_types": ["application/json", "text/plain"]
  },
  "security": {
    "max_string_length": 1048576,
    "max_array_length": 10000,
    "max_object_properties": 1000,
    "max_nesting_depth": 32,
    "sanitization": {
      "html_entities": true,
      "sql_injection": true,
      "xss_prevention": true
    }
  }
}
```

### 8.2 Binary Serialization Alternatives

Support for Protocol Buffers and MessagePack for performance-critical scenarios:

```protobuf
// Protocol Buffers message envelope
syntax = "proto3";
package mister_smith.messages;

message BaseMessage {
  string message_id = 1;
  int64 timestamp_unix_nano = 2;
  string schema_version = 3;
  string message_type = 4;
  string correlation_id = 5;
  string trace_id = 6;
  string source_agent_id = 7;
  string target_agent_id = 8;
  int32 priority = 9;
  string reply_to = 10;
  int64 timeout_ms = 11;
  map<string, string> metadata = 12;
  bytes payload = 13;  // Serialized message-specific payload
}
```

MessagePack configuration for space-efficient serialization:

```yaml
messagepack_config:
  use_compact_integers: true
  use_string_keys: true
  preserve_order: false
  binary_threshold_bytes: 512
  compression_after_pack: true
```

## 9. Message Routing and Addressing

### 9.1 NATS Subject Pattern Schemas

Schema definitions for NATS subject patterns and routing rules:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.mister-smith.dev/nats-subject-patterns.json",
  "title": "NATS Subject Pattern Definitions",
  "type": "object",
  "properties": {
    "agent_subjects": {
      "type": "object",
      "properties": {
        "commands": {
          "type": "string",
          "pattern": "^agents\\.[a-zA-Z0-9_-]+\\.commands\\.[a-z_]+$",
          "example": "agents.analyzer-001.commands.execute"
        },
        "status": {
          "type": "string", 
          "pattern": "^agents\\.[a-zA-Z0-9_-]+\\.status$",
          "example": "agents.analyzer-001.status"
        },
        "heartbeat": {
          "type": "string",
          "pattern": "^agents\\.[a-zA-Z0-9_-]+\\.heartbeat$",
          "example": "agents.analyzer-001.heartbeat"
        }
      }
    },
    "task_subjects": {
      "type": "object",
      "properties": {
        "assignment": {
          "type": "string",
          "pattern": "^tasks\\.[a-z_]+\\.assignment$",
          "example": "tasks.analysis.assignment"
        },
        "progress": {
          "type": "string",
          "pattern": "^tasks\\.[0-9a-f-]{36}\\.progress$",
          "example": "tasks.550e8400-e29b-41d4-a716-446655440000.progress"
        },
        "result": {
          "type": "string", 
          "pattern": "^tasks\\.[0-9a-f-]{36}\\.result$",
          "example": "tasks.550e8400-e29b-41d4-a716-446655440000.result"
        }
      }
    },
    "system_subjects": {
      "type": "object",
      "properties": {
        "alerts": {
          "type": "string",
          "pattern": "^system\\.alerts\\.(info|warning|error|critical|emergency)$",
          "example": "system.alerts.critical"
        },
        "health": {
          "type": "string",
          "pattern": "^system\\.health\\.[a-zA-Z0-9_-]+$",
          "example": "system.health.nats_cluster"
        }
      }
    },
    "cli_subjects": {
      "type": "object",
      "properties": {
        "hooks": {
          "type": "string",
          "pattern": "^cli\\.hooks\\.(startup|pre_task|post_task|on_error|on_file_change)\\.[a-zA-Z0-9_-]+$",
          "example": "cli.hooks.pre_task.analyzer-001"
        },
        "responses": {
          "type": "string",
          "pattern": "^cli\\.responses\\.[a-zA-Z0-9_-]+$",
          "example": "cli.responses.analyzer-001"
        }
      }
    }
  }
}
```

### 9.2 Message Correlation Strategies

```json
{
  "correlation_strategies": {
    "request_response": {
      "pattern": "Include correlation_id in request, match in response",
      "timeout_handling": "Exponential backoff with max attempts",
      "correlation_storage": "In-memory cache with TTL"
    },
    "workflow_coordination": {
      "pattern": "Use workflow_id for all related messages",
      "state_tracking": "Persistent storage with checkpointing",
      "recovery": "Replay from last checkpoint"
    },
    "distributed_tracing": {
      "pattern": "Propagate trace_id across all service boundaries",
      "sampling": "Configurable sampling rate",
      "span_creation": "Automatic span creation per message hop"
    }
  }
}
```

## 10. Message Transformation Patterns

### 10.1 Protocol Adaptation

Transformation rules for adapting messages between different transport protocols:

```yaml
transformation_rules:
  json_to_protobuf:
    - map_timestamps: "ISO string to Unix nanoseconds"
    - compress_payload: "Gzip compression for large payloads"
    - validate_required: "Ensure all required protobuf fields present"
    
  protobuf_to_json:
    - expand_timestamps: "Unix nanoseconds to ISO string"
    - decompress_payload: "Decompress gzipped payloads"
    - add_schema_version: "Include JSON schema version"
    
  nats_to_grpc:
    - extract_metadata: "Move NATS headers to gRPC metadata"
    - preserve_correlation: "Maintain correlation_id across protocols"
    - map_subjects: "Convert NATS subjects to gRPC method names"
    
  grpc_to_http:
    - convert_streaming: "Buffer streaming responses for HTTP"
    - map_status_codes: "gRPC status to HTTP status codes"
    - flatten_nested: "Flatten nested objects for query parameters"
```

### 10.2 Message Enrichment

Automatic message enhancement and metadata injection:

```json
{
  "enrichment_rules": {
    "timestamp_injection": {
      "condition": "timestamp field missing",
      "action": "Add current UTC timestamp",
      "format": "ISO 8601 RFC 3339"
    },
    "correlation_generation": {
      "condition": "correlation_id field missing and reply_to present",
      "action": "Generate UUID v4 correlation_id",
      "propagation": "Include in all related messages"
    },
    "trace_propagation": {
      "condition": "Always for cross-service calls",
      "action": "Propagate or generate trace_id",
      "sampling": "Configurable rate based on service"
    },
    "agent_metadata": {
      "condition": "Message from known agent",
      "action": "Inject agent version and capabilities",
      "source": "Agent registry lookup"
    },
    "security_context": {
      "condition": "Authenticated session",
      "action": "Add security context metadata",
      "fields": ["user_id", "permissions", "security_level"]
    }
  }
}
```

### 10.3 Content Transformation

Field mapping and data type conversion rules:

```yaml
field_mappings:
  version_compatibility:
    v1_0_0_to_v1_1_0:
      - add_field: "schema_version" 
        default_value: "1.1.0"
      - rename_field: 
          from: "agent_type"
          to: "agent_classification"
      - split_field:
          from: "full_name"
          to: ["first_name", "last_name"]
          
  data_type_conversions:
    timestamp_normalization:
      - from: "Unix timestamp (seconds)"
        to: "ISO 8601 string"
        conversion: "multiply by 1000, convert to datetime, format ISO"
      - from: "Unix timestamp (milliseconds)"
        to: "ISO 8601 string" 
        conversion: "convert to datetime, format ISO"
        
  nested_object_handling:
    flattening_rules:
      - condition: "target protocol is HTTP query params"
        action: "Flatten nested objects with dot notation"
        example: "metadata.source -> metadata.source"
    expansion_rules:
      - condition: "source is flattened format"
        action: "Rebuild nested object structure"
        delimiter: "."
```

## 11. Schema Version Management

### 11.1 Versioning Strategy

```json
{
  "versioning_strategy": {
    "semantic_versioning": {
      "format": "MAJOR.MINOR.PATCH",
      "major_change": "Breaking changes requiring code updates",
      "minor_change": "Backward-compatible additions",
      "patch_change": "Bug fixes and documentation"
    },
    "compatibility_policy": {
      "backward_compatibility": "Maintain for 2 major versions",
      "forward_compatibility": "Best effort with graceful degradation",
      "deprecation_period": "6 months minimum for major changes"
    },
    "schema_evolution": {
      "additive_changes": "Always allowed in minor versions",
      "field_removal": "Deprecated in minor, removed in major",
      "type_changes": "Only allowed in major versions",
      "constraint_tightening": "Only allowed in major versions"
    }
  }
}
```

### 11.2 Compatibility Matrix

Version compatibility and migration support:

```yaml
compatibility_matrix:
  v1.0.0:
    compatible_with: ["1.0.x"]
    migration_from: []
    deprecation_date: "2025-12-31"
    
  v1.1.0:
    compatible_with: ["1.0.x", "1.1.x"] 
    migration_from: ["1.0.0"]
    new_features:
      - "Enhanced error details"
      - "Agent health monitoring"
      - "Workflow coordination"
      
  v2.0.0:
    compatible_with: ["2.0.x"]
    migration_from: ["1.0.0", "1.1.0"]
    breaking_changes:
      - "Renamed agent_type to agent_classification"
      - "Required correlation_id for all requests"
      - "Changed timestamp format to nanosecond precision"

migration_tools:
  schema_converter:
    supports: ["1.0.0 -> 1.1.0", "1.1.0 -> 2.0.0"]
    validation: "Pre and post conversion validation"
    rollback: "Automatic rollback on conversion failure"
    
  version_negotiation:
    strategy: "Highest common version"
    fallback: "Graceful degradation to v1.0.0"
    discovery: "Schema registry lookup"
```

### 11.3 Schema Registry

Central schema registry for version management and discovery:

```json
{
  "schema_registry": {
    "base_url": "https://schemas.mister-smith.dev",
    "endpoints": {
      "list_schemas": "/schemas",
      "get_schema": "/schemas/{schema_id}",
      "get_version": "/schemas/{schema_id}/versions/{version}",
      "validate": "/validate/{schema_id}/{version}"
    },
    "caching": {
      "cache_duration": 3600,
      "cache_size": 1000,
      "cache_key_format": "{schema_id}:{version}"
    },
    "versioning": {
      "latest_alias": "Support for 'latest' version alias",
      "version_negotiation": "Automatic version compatibility checking",
      "schema_evolution": "Track schema evolution and migrations"
    }
  }
}
```

## 12. Implementation Guidelines

### 12.1 Code Generation

Framework for generating type-safe code from schemas:

```rust
// Example Rust code generation
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseMessage {
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub schema_version: String,
    pub message_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

fn default_priority() -> u8 { 5 }

// Message type discriminated union
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "message_type")]
pub enum Message {
    #[serde(rename = "agent_command")]
    AgentCommand(AgentCommandMessage),
    #[serde(rename = "agent_status")]
    AgentStatus(AgentStatusMessage),
    #[serde(rename = "task_assignment")]
    TaskAssignment(TaskAssignmentMessage),
    // ... other message types
}
```

### 12.2 Validation Integration

Runtime validation with performance optimization:

```rust
use jsonschema::{JSONSchema, ValidationError};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub struct MessageValidator {
    schemas: HashMap<String, Arc<JSONSchema>>,
    validation_level: ValidationLevel,
}

impl MessageValidator {
    pub fn validate_message(&self, message: &Value) -> Result<(), Vec<ValidationError>> {
        let message_type = message.get("message_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| vec![ValidationError::custom("missing message_type")])?;
            
        let schema = self.schemas.get(message_type)
            .ok_or_else(|| vec![ValidationError::custom("unknown message_type")])?;
            
        match self.validation_level {
            ValidationLevel::Strict => schema.validate(message).collect(),
            ValidationLevel::Standard => self.validate_essential_fields(message),
            ValidationLevel::Permissive => self.validate_minimal(message),
        }
    }
    
    fn validate_essential_fields(&self, message: &Value) -> Result<(), Vec<ValidationError>> {
        // Optimized validation for production use
        // Only validate critical fields for performance
        Ok(())
    }
}
```

### 12.3 Testing Strategies

Comprehensive testing approach for message schemas:

```yaml
testing_strategy:
  unit_tests:
    - schema_validation_tests: "Test all valid and invalid message examples"
    - serialization_tests: "Round-trip testing for all formats"
    - compatibility_tests: "Cross-version compatibility validation"
    
  integration_tests:
    - end_to_end_flow: "Complete message flow through all components"
    - protocol_conversion: "Message transformation between protocols"
    - error_handling: "Proper error propagation and handling"
    
  performance_tests:
    - validation_benchmarks: "Measure validation performance"
    - serialization_benchmarks: "Compare serialization formats"
    - memory_usage: "Monitor memory consumption patterns"
    
  property_based_tests:
    - message_generation: "Generate random valid messages"
    - invariant_testing: "Verify schema invariants hold"
    - fuzzing: "Test with malformed message data"

test_data_generation:
  valid_examples:
    - minimal_messages: "Messages with only required fields"
    - complete_messages: "Messages with all optional fields"
    - edge_cases: "Boundary value testing"
    
  invalid_examples:
    - missing_required: "Test required field validation"
    - invalid_formats: "Test format validation"
    - constraint_violations: "Test constraint enforcement"
```

### 12.4 Monitoring and Observability

Schema validation and message processing metrics:

```yaml
monitoring_metrics:
  validation_metrics:
    - validation_success_rate: "Percentage of messages passing validation"
    - validation_latency: "Time spent in validation (p50, p95, p99)"
    - validation_errors_by_type: "Breakdown of validation error types"
    
  message_metrics:
    - message_throughput: "Messages processed per second by type"
    - message_size_distribution: "Size distribution by message type"
    - serialization_format_usage: "Usage breakdown of JSON/protobuf/msgpack"
    
  schema_metrics:
    - schema_cache_hit_rate: "Schema cache effectiveness"
    - schema_version_distribution: "Usage of different schema versions"
    - schema_evolution_events: "Schema update and migration events"

alerting_rules:
  validation_failures:
    threshold: "> 5% error rate over 5 minutes"
    severity: "warning"
    action: "Investigate schema compatibility issues"
    
  performance_degradation:
    threshold: "p95 validation latency > 100ms"
    severity: "warning" 
    action: "Review validation optimization"
    
  schema_compatibility:
    threshold: "Schema version drift detected"
    severity: "info"
    action: "Plan schema migration"
```

## 13. Security Considerations

### 13.1 Input Validation and Sanitization

Security-focused validation rules:

```json
{
  "security_validation": {
    "input_sanitization": {
      "max_string_length": 1048576,
      "max_array_length": 10000,
      "max_object_properties": 1000,
      "max_nesting_depth": 32,
      "prohibited_patterns": [
        "javascript:",
        "data:",
        "vbscript:",
        "<script",
        "</script>",
        "{{.*}}",
        "${.*}"
      ]
    },
    "injection_prevention": {
      "sql_injection": "Escape SQL metacharacters",
      "xss_prevention": "HTML entity encoding",
      "command_injection": "Validate against allowed characters",
      "path_traversal": "Normalize and validate file paths"
    },
    "content_validation": {
      "validate_urls": "Check URL format and allowed schemes",
      "validate_emails": "RFC 5322 email format validation",
      "validate_file_types": "Whitelist allowed file extensions",
      "validate_encoding": "Ensure UTF-8 encoding compliance"
    }
  }
}
```

### 13.2 Message Encryption and Signing

Support for end-to-end encryption and message integrity:

```yaml
encryption_support:
  encryption_algorithms:
    - "AES-256-GCM" # For payload encryption
    - "ChaCha20-Poly1305" # Alternative encryption
    - "RSA-OAEP-256" # For key exchange
    
  signing_algorithms:
    - "Ed25519" # Digital signatures
    - "ECDSA-P256" # Alternative signing
    - "HMAC-SHA256" # Message authentication
    
  key_management:
    rotation_policy: "Monthly automatic rotation"
    key_derivation: "PBKDF2 with 100,000 iterations"
    secure_storage: "HSM or secure enclave"
    
  message_envelope:
    encrypted_payload: "Base64-encoded encrypted content"
    signature: "Digital signature of entire message"
    key_id: "Reference to encryption key"
    algorithm: "Encryption algorithm identifier"
```

## 14. Performance Optimization

### 14.1 Message Size Optimization

Strategies for reducing message overhead:

```yaml
size_optimization:
  field_optimization:
    - use_short_field_names: "For high-frequency messages"
    - optional_field_omission: "Skip null/empty fields"
    - enum_value_compression: "Use integers instead of strings"
    
  payload_compression:
    - compression_threshold: 1024 # bytes
    - algorithms: ["gzip", "lz4", "snappy"]
    - adaptive_compression: "Choose algorithm based on payload type"
    
  schema_optimization:
    - minimize_nesting: "Flatten nested structures where possible"
    - reuse_definitions: "Extensive use of $ref for common types"
    - efficient_validation: "Optimize schema for fast validation"

binary_formats:
  protobuf_optimization:
    - field_numbering: "Optimal field number assignment"
    - packed_encoding: "Use packed encoding for arrays"
    - oneof_fields: "Use oneof for discriminated unions"
    
  messagepack_optimization:
    - compact_integers: "Use smallest integer representation"
    - string_interning: "Reuse common string values"
    - binary_data: "Use binary type for large data"
```

### 14.2 Validation Performance

High-performance validation strategies:

```rust
// Fast-path validation for critical message types
pub struct FastPathValidator {
    // Pre-compiled validation rules for common cases
    essential_validators: HashMap<String, Box<dyn Fn(&Value) -> bool>>,
    
    // LRU cache for validation results
    validation_cache: LruCache<u64, ValidationResult>,
    
    // Statistics for optimization
    validation_stats: ValidationStats,
}

impl FastPathValidator {
    pub fn validate_fast(&mut self, message: &Value) -> ValidationResult {
        // Fast hash-based cache lookup
        let cache_key = self.hash_message(message);
        if let Some(cached_result) = self.validation_cache.get(&cache_key) {
            self.validation_stats.cache_hits += 1;
            return cached_result.clone();
        }
        
        // Fast-path validation for known message types
        if let Some(validator) = self.essential_validators.get(message_type) {
            let result = if validator(message) {
                ValidationResult::Valid
            } else {
                ValidationResult::Invalid(vec!["fast validation failed".into()])
            };
            
            self.validation_cache.put(cache_key, result.clone());
            return result;
        }
        
        // Fall back to full schema validation
        self.validate_full_schema(message)
    }
}
```

## 15. Future Extensions

### 15.1 Planned Schema Enhancements

Future message types and capabilities:

```yaml
planned_extensions:
  v1.2.0:
    - distributed_consensus_messages: "Raft/PBFT consensus protocol support"
    - streaming_data_messages: "Large dataset streaming protocols"
    - machine_learning_pipeline: "ML model training and inference coordination"
    
  v1.3.0:
    - multi_tenant_support: "Tenant isolation and resource quotas"
    - advanced_security: "Zero-trust security model integration"
    - edge_computing: "Edge node coordination and synchronization"
    
  v2.0.0:
    - quantum_resistance: "Post-quantum cryptography support"
    - semantic_messaging: "AI-powered message understanding"
    - self_healing_protocols: "Automatic error recovery and adaptation"
```

### 15.2 Integration Roadmap

Planned integrations with emerging technologies:

```yaml
integration_roadmap:
  blockchain_integration:
    - immutable_audit_logs: "Blockchain-based message audit trails"
    - smart_contracts: "Automated contract execution based on messages"
    - decentralized_identity: "Web3 identity and authentication"
    
  ai_enhancements:
    - intelligent_routing: "AI-powered message routing optimization"
    - anomaly_detection: "ML-based message pattern analysis"
    - auto_schema_generation: "AI-assisted schema evolution"
    
  cloud_native:
    - serverless_functions: "Event-driven serverless message processing"
    - service_mesh: "Istio/Linkerd integration for advanced routing"
    - gitops_deployment: "GitOps-based schema and configuration management"
```

---

## Summary

This comprehensive message schema specification provides:

1. **Complete JSON Schema definitions** for all framework message types
2. **Robust validation framework** with multiple performance levels
3. **Flexible serialization support** (JSON, Protocol Buffers, MessagePack)
4. **Advanced routing and addressing** schemes for NATS and other transports
5. **Message transformation patterns** for protocol adaptation
6. **Version management strategy** with backward compatibility
7. **Security considerations** including encryption and validation
8. **Performance optimization** techniques for high-throughput scenarios
9. **Implementation guidelines** with code generation and testing strategies
10. **Future-ready architecture** for emerging technology integration

The schema system integrates seamlessly with the existing transport layer specifications and agent orchestration patterns, providing a comprehensive foundation for type-safe, performant, and evolvable agent communication throughout the Mister Smith AI Agent Framework.

*Message Schema Definitions v1.0.0 - Agent Framework Communication Protocol*
*Integration with Transport Layer Specifications and Agent Orchestration Patterns*