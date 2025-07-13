# MCP Tools Visual Map

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Claude-Flow MCP Tools (87 Total)                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐ │
│  │ Swarm Coord (12) │  │ Neural Net (15)  │  │ Memory (12)      │ │
│  ├──────────────────┤  ├──────────────────┤  ├──────────────────┤ │
│  │ • swarm_init     │  │ • neural_train   │  │ • memory_usage   │ │
│  │ • agent_spawn    │  │ • neural_predict │  │ • memory_search  │ │
│  │ • task_orchestr. │  │ • model_save     │  │ • state_snapshot │ │
│  │ • topology_opt.  │  │ • ensemble_create│  │ • cache_manage   │ │
│  │ • load_balance   │  │ • transfer_learn │  │ • memory_sync    │ │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘ │
│                                                                      │
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐ │
│  │ Analysis (13)    │  │ GitHub (8)       │  │ DAA (8)          │ │
│  ├──────────────────┤  ├──────────────────┤  ├──────────────────┤ │
│  │ • perf_report    │  │ • repo_analyze   │  │ • agent_create   │ │
│  │ • bottleneck     │  │ • pr_manage      │  │ • capability     │ │
│  │ • token_usage    │  │ • issue_track    │  │ • consensus      │ │
│  │ • benchmark_run  │  │ • code_review    │  │ • fault_tolerant │ │
│  │ • health_check   │  │ • sync_coord     │  │ • optimization   │ │
│  └──────────────────┘  └──────────────────┘  └──────────────────┘ │
│                                                                      │
│  ┌──────────────────┐  ┌──────────────────┐                        │
│  │ Workflow (11)    │  │ System (8)       │                        │
│  ├──────────────────┤  ├──────────────────┤                        │
│  │ • workflow_create│  │ • terminal_exec  │                        │
│  │ • sparc_mode     │  │ • config_manage  │                        │
│  │ • automation     │  │ • security_scan  │                        │
│  │ • pipeline       │  │ • backup_create  │                        │
│  │ • parallel_exec  │  │ • diagnostic_run │                        │
│  └──────────────────┘  └──────────────────┘                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘

## Integration Flow

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Claude Code   │────▶│   MCP Server    │────▶│  Tool Registry  │
│   (Executor)    │     │ (Coordinator)   │     │   (87 Tools)    │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        │                        │                        │
        ▼                        ▼                        ▼
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│ Actual Work:    │     │ Context Inject: │     │ Tool Categories:│
│ • File ops      │     │ • Orchestrator  │     │ • Swarm (12)    │
│ • Code gen      │     │ • Swarm Coord   │     │ • Neural (15)   │
│ • Bash cmds     │     │ • Resource Mgr  │     │ • Memory (12)   │
│ • Git ops       │     │ • Message Bus   │     │ • Analysis (13) │
│ • Testing       │     │ • Monitor       │     │ • GitHub (8)    │
└─────────────────┘     └─────────────────┘     │ • DAA (8)       │
                                                 │ • Workflow (11) │
                                                 │ • System (8)    │
                                                 └─────────────────┘
```

## Key Pattern: Separation of Concerns

```
MCP Tools                          Claude Code
─────────                          ───────────
Coordination ─────────────────────▶ Execution
Planning     ─────────────────────▶ Implementation  
Memory       ─────────────────────▶ File Operations
Intelligence ─────────────────────▶ Code Generation
Monitoring   ─────────────────────▶ System Commands
```