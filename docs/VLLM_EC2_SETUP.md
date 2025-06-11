# vLLM on AWS EC2 Setup Guide

## Overview

This guide shows you how to run vLLM on AWS EC2 with GPU acceleration and connect to it from your local macOS development environment. This is the **recommended approach for macOS users** who want high-performance vLLM inference without the local compatibility issues.

## 🚀 Benefits of EC2 + vLLM

### Why This Approach Works Best
- **🔥 GPU Acceleration**: NVIDIA A100, V100, or T4 instances
- **⚡ High Performance**: 10-100x faster than local macOS CPU
- **💰 Cost Effective**: Pay only when running inference
- **🛠️ Easy Setup**: No local build dependencies or version conflicts
- **🔄 Scalable**: Spin up/down instances as needed
- **🌐 Remote Access**: Connect from anywhere

### Performance Comparison
| Setup | Throughput | Cost | Setup Time |
|-------|------------|------|------------|
| **macOS vLLM (CPU)** | 5-15 req/s | Free | Hours (often fails) |
| **macOS Ollama** | 20-50 req/s | Free | 5 minutes |
| **EC2 vLLM (GPU)** | 500-5000 req/s | $0.50-3/hour | 15 minutes |

## 📋 Prerequisites

### Local Requirements
- AWS CLI installed and configured
- SSH key pair for EC2 access
- Circuit Breaker running locally

### AWS Requirements
- AWS Account with EC2 access
- Sufficient EC2 limits for GPU instances
- VPC with internet gateway (default VPC works)

## 🛠️ EC2 Instance Setup

### 1. Choose the Right Instance Type

**Recommended Instance Types:**

| Instance Type | GPU | VRAM | Cost/Hour | Use Case |
|---------------|-----|------|-----------|----------|
| **g4dn.xlarge** | T4 | 16GB | ~$0.50 | Development, small models |
| **g4dn.2xlarge** | T4 | 16GB | ~$0.75 | Production, 7B models |
| **p3.2xlarge** | V100 | 16GB | ~$3.00 | High performance, 13B models |
| **p4d.24xlarge** | A100 | 40GB | ~$32.00 | Large models, maximum performance |

**For getting started, `g4dn.xlarge` is perfect.**

### 2. Launch EC2 Instance

```bash
# Create security group for vLLM
aws ec2 create-security-group \
    --group-name vllm-sg \
    --description "Security group for vLLM server"

# Get the security group ID
SG_ID=$(aws ec2 describe-security-groups \
    --group-names vllm-sg \
    --query 'SecurityGroups[0].GroupId' \
    --output text)

# Allow SSH access
aws ec2 authorize-security-group-ingress \
    --group-id $SG_ID \
    --protocol tcp \
    --port 22 \
    --cidr 0.0.0.0/0

# Allow vLLM API access (port 8000)
aws ec2 authorize-security-group-ingress \
    --group-id $SG_ID \
    --protocol tcp \
    --port 8000 \
    --cidr 0.0.0.0/0

# Launch instance
aws ec2 run-instances \
    --image-id ami-0c02fb55956c7d316 \
    --count 1 \
    --instance-type g4dn.xlarge \
    --key-name your-key-pair \
    --security-group-ids $SG_ID \
    --tag-specifications 'ResourceType=instance,Tags=[{Key=Name,Value=vllm-server}]'
```

### 3. Alternative: Using AWS Console

1. **Go to EC2 Console** → Launch Instance
2. **Choose AMI**: Deep Learning AMI (Ubuntu 20.04) - has CUDA pre-installed
3. **Instance Type**: g4dn.xlarge (or larger)
4. **Key Pair**: Select or create a key pair
5. **Security Group**: 
   - SSH (22) from your IP
   - Custom TCP (8000) from 0.0.0.0/0
6. **Storage**: 100GB+ (for models)
7. **Launch Instance**

## 🔧 Server Setup

### 1. Connect to Your Instance

```bash
# Get instance public IP
INSTANCE_IP=$(aws ec2 describe-instances \
    --filters "Name=tag:Name,Values=vllm-server" "Name=instance-state-name,Values=running" \
    --query 'Reservations[0].Instances[0].PublicIpAddress' \
    --output text)

# SSH into instance
ssh -i ~/.ssh/your-key.pem ubuntu@$INSTANCE_IP
```

### 2. Install vLLM on Ubuntu

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install Python and pip
sudo apt install -y python3.11 python3.11-pip python3.11-venv

# Create virtual environment
python3.11 -m venv vllm_env
source vllm_env/bin/activate

# Install vLLM with CUDA support
pip install --upgrade pip
pip install vllm --extra-index-url https://download.pytorch.org/whl/cu118

# Verify installation
python -c "import vllm; print('vLLM installed successfully!')"
```

### 3. Download and Start a Model

```bash
# Start vLLM server with a 7B model
vllm serve meta-llama/Llama-2-7b-chat-hf \
    --host 0.0.0.0 \
    --port 8000 \
    --gpu-memory-utilization 0.9 \
    --max-num-seqs 256

# For coding tasks, use CodeLlama
vllm serve codellama/CodeLlama-7b-Instruct-hf \
    --host 0.0.0.0 \
    --port 8000 \
    --gpu-memory-utilization 0.9

# For lighter testing, use a smaller model
vllm serve microsoft/DialoGPT-medium \
    --host 0.0.0.0 \
    --port 8000 \
    --gpu-memory-utilization 0.7
```

### 4. Verify Server is Running

```bash
# From the EC2 instance
curl http://localhost:8000/v1/models

# From your local machine
curl http://$INSTANCE_IP:8000/v1/models
```

## 🔗 Local Circuit Breaker Configuration

### 1. Configure Environment Variables

```bash
# On your local macOS machine
export VLLM_BASE_URL=http://YOUR_EC2_IP:8000
export VLLM_DEFAULT_MODEL=meta-llama/Llama-2-7b-chat-hf
export VLLM_TIMEOUT_SECONDS=120
```

### 2. Update Circuit Breaker Configuration

Create or update `~/.config/circuit-breaker/vllm.env`:

```bash
# Circuit Breaker vLLM Configuration (EC2 Remote)
VLLM_BASE_URL=http://YOUR_EC2_IP:8000
VLLM_DEFAULT_MODEL=meta-llama/Llama-2-7b-chat-hf
VLLM_API_KEY=
VLLM_VERIFY_SSL=false
VLLM_TIMEOUT_SECONDS=120

# Mark as remote instance
VLLM_REMOTE=true
VLLM_INSTANCE_TYPE=g4dn.xlarge
```

### 3. Test Integration

```bash
# Start Circuit Breaker locally
cd circuit-breaker
cargo run --bin server

# Test the connection
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "vllm://meta-llama/Llama-2-7b-chat-hf",
    "messages": [{"role": "user", "content": "Hello from EC2!"}],
    "max_tokens": 100
  }'
```

## 🚀 Production Setup

### 1. Create Startup Script

Create `/home/ubuntu/start_vllm.sh` on EC2:

```bash
#!/bin/bash

# vLLM Startup Script for EC2
cd /home/ubuntu
source vllm_env/bin/activate

# Configuration
MODEL=${1:-meta-llama/Llama-2-7b-chat-hf}
PORT=${2:-8000}
GPU_MEMORY=${3:-0.9}

echo "Starting vLLM server..."
echo "Model: $MODEL"
echo "Port: $PORT"
echo "GPU Memory: $GPU_MEMORY"

# Start vLLM with optimized settings for EC2
vllm serve "$MODEL" \
    --host 0.0.0.0 \
    --port $PORT \
    --gpu-memory-utilization $GPU_MEMORY \
    --max-num-seqs 512 \
    --max-model-len 4096 \
    --tensor-parallel-size 1 \
    --trust-remote-code
```

Make it executable:
```bash
chmod +x /home/ubuntu/start_vllm.sh
```

### 2. Create Systemd Service (Optional)

Create `/etc/systemd/system/vllm.service`:

```ini
[Unit]
Description=vLLM Server
After=network.target

[Service]
Type=simple
User=ubuntu
WorkingDirectory=/home/ubuntu
Environment=PATH=/home/ubuntu/vllm_env/bin
ExecStart=/home/ubuntu/start_vllm.sh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl enable vllm
sudo systemctl start vllm
sudo systemctl status vllm
```

### 3. Load Balancer Setup (Advanced)

For high availability, use multiple instances behind an Application Load Balancer:

```bash
# Create target group
aws elbv2 create-target-group \
    --name vllm-targets \
    --protocol HTTP \
    --port 8000 \
    --vpc-id vpc-12345678 \
    --health-check-path /v1/models

# Create load balancer
aws elbv2 create-load-balancer \
    --name vllm-lb \
    --subnets subnet-12345678 subnet-87654321 \
    --security-groups sg-12345678
```

## 💰 Cost Optimization

### 1. Spot Instances

Save 70-90% with Spot Instances:

```bash
# Launch spot instance
aws ec2 run-instances \
    --image-id ami-0c02fb55956c7d316 \
    --instance-type g4dn.xlarge \
    --key-name your-key-pair \
    --security-group-ids $SG_ID \
    --instance-market-options 'MarketType=spot,SpotOptions={SpotInstanceType=one-time,MaxPrice=0.25}'
```

### 2. Auto Shutdown

Add to your startup script:
```bash
# Auto-shutdown after 2 hours of inactivity
echo "sudo shutdown -h +120" | at now
```

### 3. Scheduled Start/Stop

```bash
# Create Lambda function to start/stop instances on schedule
# Or use EC2 Instance Scheduler
```

## 🔍 Monitoring and Logging

### 1. CloudWatch Monitoring

```bash
# Install CloudWatch agent
wget https://s3.amazonaws.com/amazoncloudwatch-agent/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb
sudo dpkg -i amazon-cloudwatch-agent.deb

# Monitor GPU usage
nvidia-smi --query-gpu=utilization.gpu,memory.used,memory.total --format=csv -l 1
```

### 2. Application Monitoring

```bash
# Monitor vLLM metrics
curl http://localhost:8000/metrics

# Log requests
tail -f /var/log/vllm.log
```

## 🔐 Security Best Practices

### 1. Network Security

```bash
# Restrict access to your IP only
aws ec2 authorize-security-group-ingress \
    --group-id $SG_ID \
    --protocol tcp \
    --port 8000 \
    --cidr YOUR_IP/32
```

### 2. API Authentication

Add API key authentication to vLLM:

```bash
# Start vLLM with API key
vllm serve model-name \
    --host 0.0.0.0 \
    --port 8000 \
    --api-key your-secret-key
```

Update local config:
```bash
export VLLM_API_KEY=your-secret-key
```

### 3. VPC and Private Subnets

For production, deploy in private subnets with NAT Gateway.

## 🛠️ Troubleshooting

### Common Issues

**1. Connection Refused**
```bash
# Check security group allows port 8000
# Verify vLLM is running: ps aux | grep vllm
# Check logs: journalctl -u vllm -f
```

**2. Out of Memory**
```bash
# Reduce GPU memory utilization
--gpu-memory-utilization 0.7

# Use smaller model
# Add swap space
sudo fallocate -l 8G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

**3. Slow Performance**
```bash
# Check GPU utilization
nvidia-smi

# Increase batch size
--max-num-seqs 512

# Use tensor parallelism for large models
--tensor-parallel-size 2
```

## 📊 Performance Benchmarks

### Expected Performance (g4dn.xlarge)

| Model Size | Throughput (req/s) | Latency (ms) | VRAM Usage |
|------------|-------------------|--------------|------------|
| 2B | 800-1200 | 50-100 | 4GB |
| 7B | 300-500 | 100-200 | 14GB |
| 13B | 150-250 | 200-400 | 16GB (max) |

### Scaling Guidelines

- **Single GPU**: Up to 13B models
- **Multi-GPU**: 30B+ models with tensor parallelism
- **Multiple Instances**: Load balance for high throughput

## 🚀 Quick Start Script

Save this as `setup_ec2_vllm.sh`:

```bash
#!/bin/bash

# Quick EC2 vLLM Setup Script
INSTANCE_TYPE=${1:-g4dn.xlarge}
MODEL=${2:-meta-llama/Llama-2-7b-chat-hf}

echo "Setting up vLLM on EC2..."
echo "Instance Type: $INSTANCE_TYPE"
echo "Model: $MODEL"

# Launch instance
INSTANCE_ID=$(aws ec2 run-instances \
    --image-id ami-0c02fb55956c7d316 \
    --count 1 \
    --instance-type $INSTANCE_TYPE \
    --key-name your-key-pair \
    --security-group-ids sg-12345678 \
    --query 'Instances[0].InstanceId' \
    --output text)

echo "Launched instance: $INSTANCE_ID"
echo "Waiting for instance to be running..."

aws ec2 wait instance-running --instance-ids $INSTANCE_ID

INSTANCE_IP=$(aws ec2 describe-instances \
    --instance-ids $INSTANCE_ID \
    --query 'Reservations[0].Instances[0].PublicIpAddress' \
    --output text)

echo "Instance IP: $INSTANCE_IP"
echo "SSH command: ssh -i ~/.ssh/your-key.pem ubuntu@$INSTANCE_IP"
echo "Setup vLLM with: ./start_vllm.sh $MODEL"
echo "Test with: curl http://$INSTANCE_IP:8000/v1/models"
```

## 🎯 Summary

**EC2 + vLLM is the best solution for macOS users because:**

✅ **No local compatibility issues** - runs on proven Linux environment
✅ **GPU acceleration** - 10-100x faster than local CPU inference  
✅ **Cost effective** - pay only when running ($0.50-3/hour)
✅ **Scalable** - can handle production workloads
✅ **Easy setup** - 15 minutes vs hours of troubleshooting locally
✅ **Professional grade** - same setup used by production applications

**Total setup time: ~15 minutes**
**Total cost: ~$0.50/hour for development**
**Performance: 300-500 req/s vs 5-15 req/s on macOS**

This approach gives you the full power of vLLM without any of the macOS compatibility headaches!