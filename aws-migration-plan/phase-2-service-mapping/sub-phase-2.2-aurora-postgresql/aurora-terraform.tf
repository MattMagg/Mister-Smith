# Aurora Serverless v2 PostgreSQL Configuration for MisterSmith

terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

# VPC Configuration (if not existing)
resource "aws_vpc" "mistersmith" {
  cidr_block           = "10.0.0.0/16"
  enable_dns_hostnames = true
  enable_dns_support   = true

  tags = {
    Name        = "mistersmith-vpc"
    Environment = "production"
    Project     = "MisterSmith"
  }
}

# Subnets for Aurora (minimum 2 AZs required)
resource "aws_subnet" "db_subnet_1" {
  vpc_id            = aws_vpc.mistersmith.id
  cidr_block        = "10.0.1.0/24"
  availability_zone = data.aws_availability_zones.available.names[0]

  tags = {
    Name = "mistersmith-db-subnet-1"
  }
}

resource "aws_subnet" "db_subnet_2" {
  vpc_id            = aws_vpc.mistersmith.id
  cidr_block        = "10.0.2.0/24"
  availability_zone = data.aws_availability_zones.available.names[1]

  tags = {
    Name = "mistersmith-db-subnet-2"
  }
}

# DB Subnet Group
resource "aws_db_subnet_group" "mistersmith" {
  name       = "mistersmith-db-subnet-group"
  subnet_ids = [aws_subnet.db_subnet_1.id, aws_subnet.db_subnet_2.id]

  tags = {
    Name = "mistersmith-db-subnet-group"
  }
}

# Security Group for Aurora
resource "aws_security_group" "aurora" {
  name        = "mistersmith-aurora-sg"
  description = "Security group for MisterSmith Aurora cluster"
  vpc_id      = aws_vpc.mistersmith.id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
    description = "PostgreSQL access from VPC"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "mistersmith-aurora-sg"
  }
}

# Security Group for RDS Proxy
resource "aws_security_group" "rds_proxy" {
  name        = "mistersmith-rds-proxy-sg"
  description = "Security group for MisterSmith RDS Proxy"
  vpc_id      = aws_vpc.mistersmith.id

  ingress {
    from_port   = 5432
    to_port     = 5432
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/16"]
    description = "PostgreSQL proxy access from VPC"
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "mistersmith-rds-proxy-sg"
  }
}

# Database credentials in Secrets Manager
resource "aws_secretsmanager_secret" "db_credentials" {
  name = "mistersmith-aurora-credentials"

  tags = {
    Name = "mistersmith-aurora-credentials"
  }
}

resource "aws_secretsmanager_secret_version" "db_credentials" {
  secret_id = aws_secretsmanager_secret.db_credentials.id
  secret_string = jsonencode({
    username = "postgres"
    password = random_password.db_password.result
  })
}

resource "random_password" "db_password" {
  length  = 32
  special = true
  override_special = "!#$%&*()-_=+[]{}<>:?"
}

# Aurora Serverless v2 Cluster
resource "aws_rds_cluster" "supervision_db" {
  cluster_identifier = "mistersmith-supervision-db"
  engine             = "aurora-postgresql"
  engine_version     = "15.4"
  engine_mode        = "provisioned"
  database_name      = "supervision_db"
  master_username    = "postgres"
  master_password    = random_password.db_password.result

  db_subnet_group_name   = aws_db_subnet_group.mistersmith.name
  vpc_security_group_ids = [aws_security_group.aurora.id]

  serverlessv2_scaling_configuration {
    min_capacity = 2
    max_capacity = 16
  }

  backup_retention_period = 7
  preferred_backup_window = "03:00-04:00"
  preferred_maintenance_window = "sun:04:00-sun:05:00"

  enabled_cloudwatch_logs_exports = ["postgresql"]
  
  skip_final_snapshot = false
  final_snapshot_identifier = "mistersmith-supervision-db-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"

  tags = {
    Name        = "mistersmith-supervision-db"
    Environment = "production"
    Project     = "MisterSmith"
  }
}

# Aurora Serverless v2 Writer Instance
resource "aws_rds_cluster_instance" "writer" {
  identifier         = "mistersmith-aurora-writer"
  cluster_identifier = aws_rds_cluster.supervision_db.id
  instance_class     = "db.serverless"
  engine             = aws_rds_cluster.supervision_db.engine
  engine_version     = aws_rds_cluster.supervision_db.engine_version

  performance_insights_enabled = true
  monitoring_interval         = 60
  monitoring_role_arn        = aws_iam_role.monitoring.arn

  tags = {
    Name = "mistersmith-aurora-writer"
    Role = "writer"
  }
}

# Aurora Serverless v2 Reader Instance
resource "aws_rds_cluster_instance" "reader" {
  identifier         = "mistersmith-aurora-reader"
  cluster_identifier = aws_rds_cluster.supervision_db.id
  instance_class     = "db.serverless"
  engine             = aws_rds_cluster.supervision_db.engine
  engine_version     = aws_rds_cluster.supervision_db.engine_version

  performance_insights_enabled = true
  monitoring_interval         = 60
  monitoring_role_arn        = aws_iam_role.monitoring.arn

  tags = {
    Name = "mistersmith-aurora-reader"
    Role = "reader"
  }
}

# IAM Role for Enhanced Monitoring
resource "aws_iam_role" "monitoring" {
  name = "mistersmith-aurora-monitoring"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "monitoring.rds.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "monitoring" {
  role       = aws_iam_role.monitoring.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}

# RDS Proxy
resource "aws_db_proxy" "mistersmith" {
  name                   = "mistersmith-db-proxy"
  engine_family         = "POSTGRESQL"
  auth {
    auth_scheme = "SECRETS"
    secret_arn  = aws_secretsmanager_secret.db_credentials.arn
  }

  role_arn               = aws_iam_role.proxy.arn
  vpc_subnet_ids         = [aws_subnet.db_subnet_1.id, aws_subnet.db_subnet_2.id]
  vpc_security_group_ids = [aws_security_group.rds_proxy.id]

  max_connections_percent         = 100
  max_idle_connections_percent    = 50
  connection_borrow_timeout       = 120

  require_tls = true

  tags = {
    Name = "mistersmith-db-proxy"
  }
}

# IAM Role for RDS Proxy
resource "aws_iam_role" "proxy" {
  name = "mistersmith-rds-proxy-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "rds.amazonaws.com"
        }
      }
    ]
  })
}

resource "aws_iam_policy" "proxy_policy" {
  name = "mistersmith-rds-proxy-policy"

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = aws_secretsmanager_secret.db_credentials.arn
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "proxy_policy" {
  role       = aws_iam_role.proxy.name
  policy_arn = aws_iam_policy.proxy_policy.arn
}

# RDS Proxy Target
resource "aws_db_proxy_target" "cluster" {
  db_proxy_name          = aws_db_proxy.mistersmith.name
  target_arn             = aws_rds_cluster.supervision_db.arn
  db_cluster_identifier  = aws_rds_cluster.supervision_db.id
}

# CloudWatch Alarms
resource "aws_cloudwatch_metric_alarm" "database_connections" {
  alarm_name          = "mistersmith-aurora-high-connections"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "DatabaseConnections"
  namespace          = "AWS/RDS"
  period             = "300"
  statistic          = "Average"
  threshold          = "25"
  alarm_description  = "This metric monitors database connections"

  dimensions = {
    DBClusterIdentifier = aws_rds_cluster.supervision_db.id
  }
}

resource "aws_cloudwatch_metric_alarm" "serverless_capacity" {
  alarm_name          = "mistersmith-aurora-capacity-scaling"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name        = "ServerlessDatabaseCapacity"
  namespace          = "AWS/RDS"
  period             = "300"
  statistic          = "Average"
  threshold          = "14"
  alarm_description  = "Alert when nearing max capacity"

  dimensions = {
    DBClusterIdentifier = aws_rds_cluster.supervision_db.id
  }
}

# Outputs
output "aurora_cluster_endpoint" {
  value       = aws_rds_cluster.supervision_db.endpoint
  description = "Aurora cluster writer endpoint"
}

output "aurora_reader_endpoint" {
  value       = aws_rds_cluster.supervision_db.reader_endpoint
  description = "Aurora cluster reader endpoint"
}

output "rds_proxy_endpoint" {
  value       = aws_db_proxy.mistersmith.endpoint
  description = "RDS Proxy endpoint for connection pooling"
}

output "db_secret_arn" {
  value       = aws_secretsmanager_secret.db_credentials.arn
  description = "ARN of the database credentials secret"
}

# Data sources
data "aws_availability_zones" "available" {
  state = "available"
}