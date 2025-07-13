# Terraform Module Structure for MisterSmith AWS Migration
# This file demonstrates how to translate the API specifications into Terraform

## Module: VPC
module "vpc" {
  source = "./modules/vpc"

  cidr_block              = "10.0.0.0/16"
  enable_ipv6             = true
  enable_dns_hostnames    = true
  enable_dns_support      = true
  
  availability_zones = ["us-east-1a", "us-east-1b"]
  
  public_subnet_cidrs  = ["10.0.1.0/24", "10.0.2.0/24"]
  private_subnet_cidrs = ["10.0.11.0/24", "10.0.12.0/24"]
  
  enable_nat_gateway = true
  single_nat_gateway = false
  
  tags = {
    Name        = "mistersmith-vpc"
    Environment = "production"
    ManagedBy   = "terraform"
  }
}

## Module: Security Groups
module "security_groups" {
  source = "./modules/security"
  
  vpc_id = module.vpc.vpc_id
  
  alb_ingress_rules = [
    {
      protocol    = "tcp"
      from_port   = 80
      to_port     = 80
      cidr_blocks = ["0.0.0.0/0"]
      description = "HTTP from anywhere"
    },
    {
      protocol    = "tcp"
      from_port   = 443
      to_port     = 443
      cidr_blocks = ["0.0.0.0/0"]
      description = "HTTPS from anywhere"
    }
  ]
  
  tags = local.common_tags
}

## Module: ECS Cluster
module "ecs_cluster" {
  source = "./modules/ecs"
  
  cluster_name = "mistersmith-cluster"
  
  capacity_providers = ["FARGATE", "FARGATE_SPOT"]
  
  default_capacity_provider_strategy = [
    {
      capacity_provider = "FARGATE_SPOT"
      weight           = 2
      base             = 0
    },
    {
      capacity_provider = "FARGATE"
      weight           = 1
      base             = 1
    }
  ]
  
  enable_container_insights = true
  
  tags = local.common_tags
}

## Module: Aurora Serverless v2
module "aurora" {
  source = "./modules/rds"
  
  cluster_identifier = "mistersmith-aurora-cluster"
  engine            = "aurora-postgresql"
  engine_version    = "15.4"
  
  serverless_v2_scaling_configuration = {
    min_capacity = 0.5
    max_capacity = 4
  }
  
  database_name   = "mistersmith"
  master_username = "mistersmith_admin"
  
  vpc_id                  = module.vpc.vpc_id
  db_subnet_group_name    = module.vpc.database_subnet_group
  vpc_security_group_ids  = [module.security_groups.rds_security_group_id]
  
  backup_retention_period = 7
  preferred_backup_window = "03:00-04:00"
  
  enabled_cloudwatch_logs_exports = ["postgresql"]
  
  deletion_protection = true
  storage_encrypted  = true
  kms_key_id        = module.kms.key_id
  
  enable_http_endpoint = true
  
  create_db_proxy = true
  proxy_configuration = {
    max_connections_percent      = 100
    max_idle_connections_percent = 50
    connection_borrow_timeout    = 120
    require_tls                 = true
  }
  
  tags = local.common_tags
}

## Module: Application Load Balancer
module "alb" {
  source = "./modules/alb"
  
  name               = "mistersmith-alb"
  load_balancer_type = "application"
  
  vpc_id          = module.vpc.vpc_id
  subnets         = module.vpc.public_subnets
  security_groups = [module.security_groups.alb_security_group_id]
  
  enable_deletion_protection = true
  enable_http2              = true
  
  target_groups = [
    {
      name             = "mistersmith-api-tg"
      backend_protocol = "HTTP"
      backend_port     = 3000
      target_type      = "ip"
      
      health_check = {
        enabled             = true
        interval            = 30
        path                = "/health"
        port                = "traffic-port"
        healthy_threshold   = 2
        unhealthy_threshold = 3
        timeout             = 5
        protocol            = "HTTP"
        matcher             = "200"
      }
      
      stickiness = {
        type            = "lb_cookie"
        cookie_duration = 86400
        enabled         = true
      }
    }
  ]
  
  https_listeners = [
    {
      port               = 443
      protocol           = "HTTPS"
      certificate_arn    = module.acm.certificate_arn
      ssl_policy         = "ELBSecurityPolicy-TLS13-1-2-2021-06"
      target_group_index = 0
    }
  ]
  
  http_tcp_listeners = [
    {
      port        = 80
      protocol    = "HTTP"
      action_type = "redirect"
      redirect = {
        port        = "443"
        protocol    = "HTTPS"
        status_code = "HTTP_301"
      }
    }
  ]
  
  tags = local.common_tags
}

## Module: ECS Services
module "ecs_services" {
  source = "./modules/ecs-services"
  
  cluster_id = module.ecs_cluster.cluster_id
  
  services = {
    api = {
      name                              = "mistersmith-api-service"
      task_definition                   = module.task_definitions.api_task_definition_arn
      desired_count                     = 2
      launch_type                       = "FARGATE"
      platform_version                  = "LATEST"
      health_check_grace_period_seconds = 60
      
      network_configuration = {
        subnets          = module.vpc.private_subnets
        security_groups  = [module.security_groups.ecs_service_security_group_id]
        assign_public_ip = false
      }
      
      load_balancer = {
        target_group_arn = module.alb.target_group_arns[0]
        container_name   = "api"
        container_port   = 3000
      }
      
      deployment_circuit_breaker = {
        enable   = true
        rollback = true
      }
      
      enable_execute_command = true
      
      auto_scaling = {
        min_capacity = 2
        max_capacity = 10
        
        policies = [
          {
            name        = "cpu-scaling"
            metric_type = "ECSServiceAverageCPUUtilization"
            target_value = 70.0
          },
          {
            name        = "memory-scaling"
            metric_type = "ECSServiceAverageMemoryUtilization"
            target_value = 80.0
          }
        ]
      }
    }
  }
  
  tags = local.common_tags
}

## Module: EventBridge
module "eventbridge" {
  source = "./modules/eventbridge"
  
  bus_name = "mistersmith-event-bus"
  
  rules = [
    {
      name         = "agent-task-events"
      description  = "Routes agent task events to appropriate handlers"
      event_pattern = jsonencode({
        source      = ["mistersmith.agents"]
        detail-type = ["Agent Task Started", "Agent Task Completed", "Agent Task Failed"]
      })
    },
    {
      name         = "discovery-events"
      description  = "Routes discovery sharing events"
      event_pattern = jsonencode({
        source      = ["mistersmith.discovery"]
        detail-type = ["Discovery Shared", "Discovery Updated"]
      })
    }
  ]
  
  targets = {
    "agent-task-events" = [
      {
        arn = module.sqs.queue_arns["task-queue"]
      }
    ]
  }
  
  tags = local.common_tags
}

## Module: SQS
module "sqs" {
  source = "./modules/sqs"
  
  queues = {
    "task-queue" = {
      name                      = "mistersmith-task-queue.fifo"
      fifo_queue               = true
      content_based_deduplication = true
      message_retention_seconds = 1209600
      visibility_timeout_seconds = 300
      receive_wait_time_seconds = 20
      
      redrive_policy = {
        maxReceiveCount = 3
      }
      
      kms_master_key_id = module.kms.key_id
    }
    
    "discovery-queue" = {
      name                      = "mistersmith-discovery-queue"
      message_retention_seconds = 345600
      visibility_timeout_seconds = 60
    }
  }
  
  tags = local.common_tags
}

## Module: DynamoDB
module "dynamodb" {
  source = "./modules/dynamodb"
  
  tables = {
    discoveries = {
      name           = "mistersmith-discoveries"
      billing_mode   = "PAY_PER_REQUEST"
      hash_key       = "discovery_id"
      range_key      = "timestamp"
      
      attributes = [
        {
          name = "discovery_id"
          type = "S"
        },
        {
          name = "timestamp"
          type = "N"
        },
        {
          name = "agent_id"
          type = "S"
        },
        {
          name = "discovery_type"
          type = "S"
        }
      ]
      
      global_secondary_indexes = [
        {
          name            = "agent-index"
          hash_key        = "agent_id"
          range_key       = "timestamp"
          projection_type = "ALL"
        },
        {
          name            = "type-index"
          hash_key        = "discovery_type"
          range_key       = "timestamp"
          projection_type = "ALL"
        }
      ]
      
      stream_enabled   = true
      stream_view_type = "NEW_AND_OLD_IMAGES"
      
      point_in_time_recovery_enabled = true
      
      server_side_encryption_enabled = true
      kms_key_arn = module.kms.key_arn
    }
  }
  
  tags = local.common_tags
}

## Module: Route 53
module "route53" {
  source = "./modules/route53"
  
  zones = {
    "mistersmith.ai" = {
      comment = "Managed by Terraform"
      
      records = [
        {
          name = ""
          type = "A"
          alias = {
            name                   = module.alb.lb_dns_name
            zone_id                = module.alb.lb_zone_id
            evaluate_target_health = true
          }
        },
        {
          name = "www"
          type = "CNAME"
          ttl  = 300
          records = ["mistersmith.ai"]
        }
      ]
    }
  }
  
  tags = local.common_tags
}

## Module: CloudWatch
module "cloudwatch" {
  source = "./modules/cloudwatch"
  
  log_groups = {
    "/ecs/mistersmith-api" = {
      retention_in_days = 30
      kms_key_id       = module.kms.key_id
    }
    "/ecs/mistersmith-mcp" = {
      retention_in_days = 30
    }
    "/aws/rds/cluster/mistersmith-aurora-cluster/postgresql" = {
      retention_in_days = 7
    }
  }
  
  alarms = {
    "mistersmith-api-high-cpu" = {
      comparison_operator = "GreaterThanThreshold"
      evaluation_periods  = 2
      metric_name        = "CPUUtilization"
      namespace          = "AWS/ECS"
      period             = 300
      statistic          = "Average"
      threshold          = 80.0
      alarm_description  = "Triggers when API service CPU exceeds 80%"
      
      dimensions = {
        ServiceName = "mistersmith-api-service"
        ClusterName = "mistersmith-cluster"
      }
      
      alarm_actions = [module.sns.topic_arn]
    }
  }
  
  dashboard = {
    name = "mistersmith-dashboard"
    
    body = jsonencode({
      widgets = [
        {
          type   = "metric"
          width  = 12
          height = 6
          
          properties = {
            metrics = [
              ["AWS/ECS", "CPUUtilization", "ServiceName", "mistersmith-api-service", "ClusterName", "mistersmith-cluster"],
              [".", "MemoryUtilization", ".", ".", ".", "."]
            ]
            period = 300
            stat   = "Average"
            region = "us-east-1"
            title  = "ECS Service Metrics"
          }
        }
      ]
    })
  }
  
  tags = local.common_tags
}

## Common Variables
locals {
  common_tags = {
    Application = "mistersmith"
    Environment = "production"
    ManagedBy   = "terraform"
    CostCenter  = "engineering"
  }
}