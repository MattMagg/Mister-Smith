#!/bin/bash

# MS3 Infrastructure Verification
# Verifies VPC, subnets, security groups, load balancers, and networking

set -euo pipefail

# Source parent functions if available
if [[ -f "../master-verify.sh" ]]; then
    source <(grep -E '^(log|track_result|RED|GREEN|YELLOW|BLUE|NC)=' ../master-verify.sh)
fi

# Infrastructure configuration
VPC_NAME="ms3-vpc"
EXPECTED_SUBNETS=6  # 3 public, 3 private across 3 AZs
EXPECTED_SECURITY_GROUPS=5  # alb, ecs, rds, bastion, endpoints
EXPECTED_NAT_GATEWAYS=3
EXPECTED_ROUTE_TABLES=4  # 1 public, 3 private

# VPC Verification
verify_vpc() {
    local vpc_id=$(aws ec2 describe-vpcs \
        --filters "Name=tag:Name,Values=${VPC_NAME}" \
        --query 'Vpcs[0].VpcId' \
        --output text 2>/dev/null || echo "None")
    
    if [[ "$vpc_id" != "None" && "$vpc_id" != "null" ]]; then
        # Verify VPC CIDR
        local vpc_cidr=$(aws ec2 describe-vpcs \
            --vpc-ids "$vpc_id" \
            --query 'Vpcs[0].CidrBlock' \
            --output text)
        
        if [[ "$vpc_cidr" == "10.0.0.0/16" ]]; then
            track_result "VPC_EXISTS" "PASS" "VPC ${vpc_id} with CIDR ${vpc_cidr}"
        else
            track_result "VPC_CIDR" "FAIL" "Expected CIDR 10.0.0.0/16, got ${vpc_cidr}"
        fi
        
        # Verify DNS settings
        local dns_support=$(aws ec2 describe-vpc-attribute \
            --vpc-id "$vpc_id" \
            --attribute enableDnsSupport \
            --query 'EnableDnsSupport.Value' \
            --output text)
        
        local dns_hostnames=$(aws ec2 describe-vpc-attribute \
            --vpc-id "$vpc_id" \
            --attribute enableDnsHostnames \
            --query 'EnableDnsHostnames.Value' \
            --output text)
        
        if [[ "$dns_support" == "true" && "$dns_hostnames" == "true" ]]; then
            track_result "VPC_DNS" "PASS" "DNS support and hostnames enabled"
        else
            track_result "VPC_DNS" "FAIL" "DNS settings incorrect"
        fi
        
        echo "$vpc_id"
    else
        track_result "VPC_EXISTS" "FAIL" "VPC ${VPC_NAME} not found"
        echo "None"
    fi
}

# Subnet Verification
verify_subnets() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "SUBNETS" "FAIL" "Cannot verify subnets without VPC"
        return 1
    fi
    
    # Count subnets
    local subnet_count=$(aws ec2 describe-subnets \
        --filters "Name=vpc-id,Values=${vpc_id}" \
        --query 'length(Subnets)' \
        --output text)
    
    if [[ $subnet_count -eq $EXPECTED_SUBNETS ]]; then
        track_result "SUBNET_COUNT" "PASS" "Found ${subnet_count} subnets"
    else
        track_result "SUBNET_COUNT" "FAIL" "Expected ${EXPECTED_SUBNETS} subnets, found ${subnet_count}"
    fi
    
    # Verify public subnets
    local public_subnets=$(aws ec2 describe-subnets \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Type,Values=public" \
        --query 'length(Subnets)' \
        --output text)
    
    if [[ $public_subnets -eq 3 ]]; then
        track_result "PUBLIC_SUBNETS" "PASS" "Found 3 public subnets"
        
        # Verify auto-assign public IP
        local auto_assign=$(aws ec2 describe-subnets \
            --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Type,Values=public" \
            --query 'Subnets[?MapPublicIpOnLaunch==`true`] | length(@)' \
            --output text)
        
        if [[ $auto_assign -eq 3 ]]; then
            track_result "PUBLIC_IP_AUTO_ASSIGN" "PASS" "All public subnets auto-assign IPs"
        else
            track_result "PUBLIC_IP_AUTO_ASSIGN" "WARN" "Not all public subnets auto-assign IPs"
        fi
    else
        track_result "PUBLIC_SUBNETS" "FAIL" "Expected 3 public subnets, found ${public_subnets}"
    fi
    
    # Verify private subnets
    local private_subnets=$(aws ec2 describe-subnets \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Type,Values=private" \
        --query 'length(Subnets)' \
        --output text)
    
    if [[ $private_subnets -eq 3 ]]; then
        track_result "PRIVATE_SUBNETS" "PASS" "Found 3 private subnets"
    else
        track_result "PRIVATE_SUBNETS" "FAIL" "Expected 3 private subnets, found ${private_subnets}"
    fi
    
    # Verify AZ distribution
    local azs=$(aws ec2 describe-subnets \
        --filters "Name=vpc-id,Values=${vpc_id}" \
        --query 'Subnets[].AvailabilityZone' \
        --output text | tr '\t' '\n' | sort -u | wc -l)
    
    if [[ $azs -eq 3 ]]; then
        track_result "AZ_DISTRIBUTION" "PASS" "Subnets distributed across 3 AZs"
    else
        track_result "AZ_DISTRIBUTION" "FAIL" "Expected 3 AZs, found ${azs}"
    fi
}

# Security Group Verification
verify_security_groups() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "SECURITY_GROUPS" "FAIL" "Cannot verify security groups without VPC"
        return 1
    fi
    
    # Count security groups
    local sg_count=$(aws ec2 describe-security-groups \
        --filters "Name=vpc-id,Values=${vpc_id}" \
        --query 'length(SecurityGroups[?GroupName!=`default`])' \
        --output text)
    
    if [[ $sg_count -ge $EXPECTED_SECURITY_GROUPS ]]; then
        track_result "SECURITY_GROUP_COUNT" "PASS" "Found ${sg_count} security groups"
    else
        track_result "SECURITY_GROUP_COUNT" "FAIL" "Expected at least ${EXPECTED_SECURITY_GROUPS} security groups, found ${sg_count}"
    fi
    
    # Verify ALB security group
    local alb_sg=$(aws ec2 describe-security-groups \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Name,Values=ms3-alb-sg" \
        --query 'SecurityGroups[0].GroupId' \
        --output text 2>/dev/null || echo "None")
    
    if [[ "$alb_sg" != "None" && "$alb_sg" != "null" ]]; then
        # Check ALB ingress rules
        local http_rule=$(aws ec2 describe-security-groups \
            --group-ids "$alb_sg" \
            --query 'SecurityGroups[0].IpPermissions[?FromPort==`80`] | length(@)' \
            --output text)
        
        local https_rule=$(aws ec2 describe-security-groups \
            --group-ids "$alb_sg" \
            --query 'SecurityGroups[0].IpPermissions[?FromPort==`443`] | length(@)' \
            --output text)
        
        if [[ $http_rule -gt 0 && $https_rule -gt 0 ]]; then
            track_result "ALB_SG_RULES" "PASS" "ALB allows HTTP/HTTPS traffic"
        else
            track_result "ALB_SG_RULES" "FAIL" "ALB security group missing HTTP/HTTPS rules"
        fi
    else
        track_result "ALB_SG" "FAIL" "ALB security group not found"
    fi
    
    # Verify ECS security group
    local ecs_sg=$(aws ec2 describe-security-groups \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Name,Values=ms3-ecs-sg" \
        --query 'SecurityGroups[0].GroupId' \
        --output text 2>/dev/null || echo "None")
    
    if [[ "$ecs_sg" != "None" && "$ecs_sg" != "null" ]]; then
        track_result "ECS_SG" "PASS" "ECS security group exists"
    else
        track_result "ECS_SG" "FAIL" "ECS security group not found"
    fi
    
    # Verify RDS security group
    local rds_sg=$(aws ec2 describe-security-groups \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Name,Values=ms3-rds-sg" \
        --query 'SecurityGroups[0].GroupId' \
        --output text 2>/dev/null || echo "None")
    
    if [[ "$rds_sg" != "None" && "$rds_sg" != "null" ]]; then
        # Check PostgreSQL port
        local pg_rule=$(aws ec2 describe-security-groups \
            --group-ids "$rds_sg" \
            --query 'SecurityGroups[0].IpPermissions[?FromPort==`5432`] | length(@)' \
            --output text)
        
        if [[ $pg_rule -gt 0 ]]; then
            track_result "RDS_SG_RULES" "PASS" "RDS allows PostgreSQL traffic"
        else
            track_result "RDS_SG_RULES" "FAIL" "RDS security group missing PostgreSQL rule"
        fi
    else
        track_result "RDS_SG" "FAIL" "RDS security group not found"
    fi
}

# NAT Gateway Verification
verify_nat_gateways() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "NAT_GATEWAYS" "FAIL" "Cannot verify NAT gateways without VPC"
        return 1
    fi
    
    # Get NAT gateways
    local nat_count=$(aws ec2 describe-nat-gateways \
        --filter "Name=vpc-id,Values=${vpc_id}" "Name=state,Values=available" \
        --query 'length(NatGateways)' \
        --output text)
    
    if [[ $nat_count -eq $EXPECTED_NAT_GATEWAYS ]]; then
        track_result "NAT_GATEWAY_COUNT" "PASS" "Found ${nat_count} NAT gateways"
        
        # Verify NAT gateway distribution
        local nat_azs=$(aws ec2 describe-nat-gateways \
            --filter "Name=vpc-id,Values=${vpc_id}" "Name=state,Values=available" \
            --query 'NatGateways[].SubnetId' \
            --output text | xargs -n1 -I{} aws ec2 describe-subnets \
            --subnet-ids {} \
            --query 'Subnets[0].AvailabilityZone' \
            --output text | sort -u | wc -l)
        
        if [[ $nat_azs -eq 3 ]]; then
            track_result "NAT_GATEWAY_AZ" "PASS" "NAT gateways distributed across 3 AZs"
        else
            track_result "NAT_GATEWAY_AZ" "WARN" "NAT gateways in ${nat_azs} AZs, expected 3"
        fi
    else
        track_result "NAT_GATEWAY_COUNT" "FAIL" "Expected ${EXPECTED_NAT_GATEWAYS} NAT gateways, found ${nat_count}"
    fi
}

# Route Table Verification
verify_route_tables() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "ROUTE_TABLES" "FAIL" "Cannot verify route tables without VPC"
        return 1
    fi
    
    # Count route tables (excluding main)
    local rt_count=$(aws ec2 describe-route-tables \
        --filters "Name=vpc-id,Values=${vpc_id}" \
        --query 'length(RouteTables[?Tags[?Key==`Name`]])' \
        --output text)
    
    if [[ $rt_count -ge $EXPECTED_ROUTE_TABLES ]]; then
        track_result "ROUTE_TABLE_COUNT" "PASS" "Found ${rt_count} route tables"
    else
        track_result "ROUTE_TABLE_COUNT" "FAIL" "Expected at least ${EXPECTED_ROUTE_TABLES} route tables, found ${rt_count}"
    fi
    
    # Verify public route table has IGW route
    local igw_route=$(aws ec2 describe-route-tables \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Type,Values=public" \
        --query 'RouteTables[0].Routes[?GatewayId!=`local` && starts_with(GatewayId, `igw-`)] | length(@)' \
        --output text)
    
    if [[ $igw_route -gt 0 ]]; then
        track_result "PUBLIC_ROUTES" "PASS" "Public route table has internet gateway route"
    else
        track_result "PUBLIC_ROUTES" "FAIL" "Public route table missing internet gateway route"
    fi
    
    # Verify private route tables have NAT routes
    local nat_routes=$(aws ec2 describe-route-tables \
        --filters "Name=vpc-id,Values=${vpc_id}" "Name=tag:Type,Values=private" \
        --query 'RouteTables[].Routes[?starts_with(NatGatewayId, `nat-`)] | length(@)' \
        --output text | xargs -n1 | grep -v '^0$' | wc -l)
    
    if [[ $nat_routes -ge 3 ]]; then
        track_result "PRIVATE_ROUTES" "PASS" "Private route tables have NAT gateway routes"
    else
        track_result "PRIVATE_ROUTES" "FAIL" "Some private route tables missing NAT gateway routes"
    fi
}

# Load Balancer Verification
verify_load_balancers() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "LOAD_BALANCERS" "FAIL" "Cannot verify load balancers without VPC"
        return 1
    fi
    
    # Check for ALB
    local alb_arn=$(aws elbv2 describe-load-balancers \
        --query "LoadBalancers[?VpcId=='${vpc_id}' && Type=='application'].LoadBalancerArn | [0]" \
        --output text 2>/dev/null || echo "None")
    
    if [[ "$alb_arn" != "None" && "$alb_arn" != "null" ]]; then
        track_result "ALB_EXISTS" "PASS" "Application Load Balancer found"
        
        # Verify ALB is active
        local alb_state=$(aws elbv2 describe-load-balancers \
            --load-balancer-arns "$alb_arn" \
            --query 'LoadBalancers[0].State.Code' \
            --output text)
        
        if [[ "$alb_state" == "active" ]]; then
            track_result "ALB_STATE" "PASS" "ALB is active"
        else
            track_result "ALB_STATE" "FAIL" "ALB state is ${alb_state}, expected active"
        fi
        
        # Verify listeners
        local listener_count=$(aws elbv2 describe-listeners \
            --load-balancer-arn "$alb_arn" \
            --query 'length(Listeners)' \
            --output text)
        
        if [[ $listener_count -ge 1 ]]; then
            track_result "ALB_LISTENERS" "PASS" "ALB has ${listener_count} listener(s)"
        else
            track_result "ALB_LISTENERS" "FAIL" "ALB has no listeners"
        fi
        
        # Verify target groups
        local tg_count=$(aws elbv2 describe-target-groups \
            --load-balancer-arn "$alb_arn" \
            --query 'length(TargetGroups)' \
            --output text 2>/dev/null || echo "0")
        
        if [[ $tg_count -ge 1 ]]; then
            track_result "ALB_TARGET_GROUPS" "PASS" "ALB has ${tg_count} target group(s)"
        else
            track_result "ALB_TARGET_GROUPS" "WARN" "ALB has no target groups attached"
        fi
    else
        track_result "ALB_EXISTS" "FAIL" "Application Load Balancer not found"
    fi
}

# VPC Endpoints Verification
verify_vpc_endpoints() {
    local vpc_id=$1
    
    if [[ "$vpc_id" == "None" ]]; then
        track_result "VPC_ENDPOINTS" "FAIL" "Cannot verify VPC endpoints without VPC"
        return 1
    fi
    
    # Expected endpoints
    local expected_endpoints=("s3" "ecr.api" "ecr.dkr" "logs" "secretsmanager")
    
    for endpoint in "${expected_endpoints[@]}"; do
        local endpoint_id=$(aws ec2 describe-vpc-endpoints \
            --filters "Name=vpc-id,Values=${vpc_id}" "Name=service-name,Values=com.amazonaws.${AWS_REGION}.${endpoint}" \
            --query 'VpcEndpoints[0].VpcEndpointId' \
            --output text 2>/dev/null || echo "None")
        
        if [[ "$endpoint_id" != "None" && "$endpoint_id" != "null" ]]; then
            # Check endpoint state
            local endpoint_state=$(aws ec2 describe-vpc-endpoints \
                --vpc-endpoint-ids "$endpoint_id" \
                --query 'VpcEndpoints[0].State' \
                --output text)
            
            if [[ "$endpoint_state" == "available" ]]; then
                track_result "VPC_ENDPOINT_${endpoint^^}" "PASS" "Endpoint for ${endpoint} is available"
            else
                track_result "VPC_ENDPOINT_${endpoint^^}" "FAIL" "Endpoint for ${endpoint} is ${endpoint_state}"
            fi
        else
            track_result "VPC_ENDPOINT_${endpoint^^}" "FAIL" "Endpoint for ${endpoint} not found"
        fi
    done
}

# Main execution
main() {
    echo "Starting Infrastructure Verification..."
    
    # Verify VPC
    echo -e "\nVerifying VPC..."
    vpc_id=$(verify_vpc)
    
    # Verify Subnets
    echo -e "\nVerifying Subnets..."
    verify_subnets "$vpc_id"
    
    # Verify Security Groups
    echo -e "\nVerifying Security Groups..."
    verify_security_groups "$vpc_id"
    
    # Verify NAT Gateways
    echo -e "\nVerifying NAT Gateways..."
    verify_nat_gateways "$vpc_id"
    
    # Verify Route Tables
    echo -e "\nVerifying Route Tables..."
    verify_route_tables "$vpc_id"
    
    # Verify Load Balancers
    echo -e "\nVerifying Load Balancers..."
    verify_load_balancers "$vpc_id"
    
    # Verify VPC Endpoints
    echo -e "\nVerifying VPC Endpoints..."
    verify_vpc_endpoints "$vpc_id"
}

# Execute if run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi