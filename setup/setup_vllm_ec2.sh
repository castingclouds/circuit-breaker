#!/bin/bash

# Circuit Breaker vLLM EC2 Setup Script (Simplified)
# This script creates an EC2 instance, generates setup scripts, and provides manual instructions

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DEFAULT_INSTANCE_TYPE="g5.2xlarge"
DEFAULT_REGION="us-west-2"
DEFAULT_AMI="ami-0c02fb55956c7d316"  # Amazon Linux 2023 AMI
DEFAULT_KEY_NAME=""
VLLM_PORT=8000
SSH_PORT=22

print_header() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║           Circuit Breaker vLLM EC2 Setup (Simplified)         ║"
    echo "║               Create Instance + Manual Setup                   ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_prerequisites() {
    print_step "Checking prerequisites..."

    # Check AWS CLI
    if ! command -v aws &> /dev/null; then
        print_error "AWS CLI not found. Please install AWS CLI first."
        echo "Install with: brew install awscli"
        exit 1
    fi

    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        print_error "AWS credentials not configured."
        echo "Configure with: aws configure"
        exit 1
    fi

    print_success "Prerequisites check passed"
}

get_user_inputs() {
    echo
    print_step "Getting configuration inputs..."

    # Instance type
    echo -n "Instance type (default: $DEFAULT_INSTANCE_TYPE): "
    read -r INSTANCE_TYPE
    INSTANCE_TYPE=${INSTANCE_TYPE:-$DEFAULT_INSTANCE_TYPE}

    # Region
    echo -n "AWS region (default: $DEFAULT_REGION): "
    read -r REGION
    REGION=${REGION:-$DEFAULT_REGION}

    # Key pair
    echo -n "Key pair name (required): "
    read -r KEY_NAME
    if [ -z "$KEY_NAME" ]; then
        print_error "Key pair name is required"
        exit 1
    fi

    # Hugging Face token
    echo -n "Hugging Face token (for CodeLlama access): "
    read -r HF_TOKEN
    if [ -z "$HF_TOKEN" ]; then
        print_warning "No Hugging Face token provided. Some models may not be accessible."
    fi

    # Model selection
    echo
    echo "Available models:"
    echo "1. codellama/CodeLlama-7b-Instruct-hf (recommended)"
    echo "2. microsoft/DialoGPT-medium (smaller, faster)"
    echo "3. meta-llama/Llama-2-7b-chat-hf"
    echo "4. Custom model"
    echo -n "Select model (1-4, default: 1): "
    read -r MODEL_CHOICE

    case $MODEL_CHOICE in
        2) MODEL="microsoft/DialoGPT-medium" ;;
        3) MODEL="meta-llama/Llama-2-7b-chat-hf" ;;
        4) 
            echo -n "Enter custom model name: "
            read -r MODEL
            ;;
        *) MODEL="codellama/CodeLlama-7b-Instruct-hf" ;;
    esac

    print_success "Configuration collected"
}

create_security_group() {
    print_step "Creating security group..."

    SG_NAME="vllm-sg-$(date +%s)"
    SG_ID=$(aws ec2 create-security-group \
        --group-name "$SG_NAME" \
        --description "Security group for vLLM server" \
        --region "$REGION" \
        --query 'GroupId' \
        --output text)

    # Add SSH rule
    aws ec2 authorize-security-group-ingress \
        --group-id "$SG_ID" \
        --protocol tcp \
        --port 22 \
        --cidr 0.0.0.0/0 \
        --region "$REGION"

    # Add vLLM HTTP rule
    aws ec2 authorize-security-group-ingress \
        --group-id "$SG_ID" \
        --protocol tcp \
        --port 8000 \
        --cidr 0.0.0.0/0 \
        --region "$REGION"

    print_success "Security group created: $SG_ID"
}

create_ec2_instance() {
    print_step "Creating EC2 instance..."

    INSTANCE_ID=$(aws ec2 run-instances \
        --image-id "$DEFAULT_AMI" \
        --count 1 \
        --instance-type "$INSTANCE_TYPE" \
        --key-name "$KEY_NAME" \
        --security-group-ids "$SG_ID" \
        --region "$REGION" \
        --tag-specifications "ResourceType=instance,Tags=[{Key=Name,Value=vllm-server-$(date +%s)}]" \
        --query 'Instances[0].InstanceId' \
        --output text)

    print_success "Instance created: $INSTANCE_ID"

    # Wait for instance to be running
    print_step "Waiting for instance to be running..."
    aws ec2 wait instance-running --instance-ids "$INSTANCE_ID" --region "$REGION"

    # Get public IP
    PUBLIC_IP=$(aws ec2 describe-instances \
        --instance-ids "$INSTANCE_ID" \
        --region "$REGION" \
        --query 'Reservations[0].Instances[0].PublicIpAddress' \
        --output text)

    print_success "Instance is running. Public IP: $PUBLIC_IP"
}

generate_setup_script() {
    print_step "Generating vLLM setup script..."

    cat > "vllm_setup.sh" << 'EOF'
#!/bin/bash

# vLLM Setup Script for EC2 Instance
# Run this script on your EC2 instance to install and configure vLLM

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_step() {
    echo -e "${BLUE}[STEP]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Configuration (will be replaced by main script)
MODEL="__MODEL__"
HF_TOKEN="__HF_TOKEN__"

echo "🚀 vLLM Setup Script"
echo "===================="
echo "Model: $MODEL"
echo "Hugging Face Token: ${HF_TOKEN:+***configured***}"
echo

# Update system
print_step "Updating system packages..."
sudo dnf update -y

# Install Docker
print_step "Installing Docker..."
sudo dnf install -y docker
sudo systemctl start docker
sudo systemctl enable docker
sudo usermod -a -G docker ec2-user

# Install nvidia-container-toolkit for GPU support
print_step "Installing NVIDIA Container Toolkit..."
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/libnvidia-container/stable/rpm/nvidia-container-toolkit.repo | \
    sudo tee /etc/yum.repos.d/nvidia-container-toolkit.repo
sudo dnf install -y nvidia-container-toolkit
sudo nvidia-ctk runtime configure --runtime=docker
sudo systemctl restart docker

# Verify GPU access
print_step "Verifying GPU access..."
if nvidia-smi; then
    print_success "GPU detected and accessible"
else
    print_error "GPU not detected. Please check instance type and drivers."
    exit 1
fi

# Create Docker run script
print_step "Creating Docker run script..."
cat > ~/run_vllm.sh << 'DOCKER_EOF'
#!/bin/bash

# Stop any existing container
docker stop vllm-server 2>/dev/null || true
docker rm vllm-server 2>/dev/null || true

# Run vLLM with Docker
echo "🐳 Starting vLLM Docker container..."
echo "Model: __MODEL__"
echo "Port: 8000"
echo

DOCKER_CMD="docker run -d \
  --name vllm-server \
  --gpus all \
  -p 8000:8000 \
  -e NVIDIA_VISIBLE_DEVICES=all"

# Add Hugging Face token if provided
if [ -n "__HF_TOKEN__" ]; then
    DOCKER_CMD="$DOCKER_CMD -e HUGGING_FACE_HUB_TOKEN=__HF_TOKEN__"
fi

DOCKER_CMD="$DOCKER_CMD vllm/vllm-openai:latest \
  --model __MODEL__ \
  --gpu-memory-utilization 0.8 \
  --max-model-len 4096 \
  --dtype float16 \
  --port 8000 \
  --host 0.0.0.0"

# Execute the command
eval $DOCKER_CMD

echo "✅ vLLM container started!"
echo "📝 Monitor logs with: docker logs -f vllm-server"
echo "🌐 Server will be available at: http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8000"
DOCKER_EOF

# Make the script executable
chmod +x ~/run_vllm.sh

# Replace placeholders in the Docker script
sed -i "s|__MODEL__|$MODEL|g" ~/run_vllm.sh
sed -i "s|__HF_TOKEN__|$HF_TOKEN|g" ~/run_vllm.sh

print_success "Setup complete!"
echo
echo "🎉 vLLM setup is ready!"
echo "======================"
echo
echo "To start vLLM:"
echo "  ./run_vllm.sh"
echo
echo "To monitor logs:"
echo "  docker logs -f vllm-server"
echo
echo "To check status:"
echo "  docker ps"
echo "  curl http://localhost:8000/v1/models"
echo
echo "Your server will be accessible at:"
echo "  http://$(curl -s http://169.254.169.254/latest/meta-data/public-ipv4):8000"
EOF

    # Replace placeholders
    sed -i "s|__MODEL__|$MODEL|g" vllm_setup.sh
    sed -i "s|__HF_TOKEN__|$HF_TOKEN|g" vllm_setup.sh

    chmod +x vllm_setup.sh
    print_success "Setup script generated: vllm_setup.sh"
}

generate_circuit_breaker_config() {
    print_step "Generating Circuit Breaker configuration..."

    cat > "update_circuit_breaker_env.sh" << EOF
#!/bin/bash

# Update Circuit Breaker .env file with new vLLM server

VLLM_URL="http://$PUBLIC_IP:8000"

echo "Updating Circuit Breaker .env file..."
echo "VLLM_BASE_URL=\$VLLM_URL"

# Update the .env file
if [ -f "../.env" ]; then
    # Update existing VLLM_BASE_URL or add it
    if grep -q "^VLLM_BASE_URL=" ../.env; then
        sed -i.bak "s|^VLLM_BASE_URL=.*|VLLM_BASE_URL=\$VLLM_URL|" ../.env
    else
        echo "VLLM_BASE_URL=\$VLLM_URL" >> ../.env
    fi
    echo "✅ Updated ../.env with VLLM_BASE_URL=\$VLLM_URL"
else
    echo "⚠️  .env file not found. Please manually set:"
    echo "   VLLM_BASE_URL=\$VLLM_URL"
fi
EOF

    chmod +x update_circuit_breaker_env.sh
    print_success "Circuit Breaker config script generated"
}

copy_scripts_to_server() {
    print_step "Copying setup script to server..."

    # Wait a bit more for SSH to be ready
    echo "Waiting for SSH to be ready..."
    sleep 30

    # Copy the setup script
    scp -i ~/.ssh/"$KEY_NAME".pem -o StrictHostKeyChecking=no vllm_setup.sh ec2-user@"$PUBLIC_IP":~/

    print_success "Setup script copied to server"
}

show_instructions() {
    echo
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    🎉 EC2 Instance Created!                    ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════════╝${NC}"
    echo
    echo -e "${BLUE}📋 Instance Information:${NC}"
    echo "   Instance ID: $INSTANCE_ID"
    echo "   Public IP: $PUBLIC_IP"
    echo "   Instance Type: $INSTANCE_TYPE"
    echo "   Model: $MODEL"
    echo
    echo -e "${BLUE}🔗 Next Steps:${NC}"
    echo
    echo -e "${YELLOW}1. SSH into your server:${NC}"
    echo "   ssh -i ~/.ssh/$KEY_NAME.pem ec2-user@$PUBLIC_IP"
    echo
    echo -e "${YELLOW}2. Run the setup script:${NC}"
    echo "   ./vllm_setup.sh"
    echo
    echo -e "${YELLOW}3. Start vLLM (after setup completes):${NC}"
    echo "   ./run_vllm.sh"
    echo
    echo -e "${YELLOW}4. Monitor the setup:${NC}"
    echo "   docker logs -f vllm-server"
    echo
    echo -e "${YELLOW}5. Test when ready:${NC}"
    echo "   curl http://$PUBLIC_IP:8000/v1/models"
    echo
    echo -e "${YELLOW}6. Update Circuit Breaker (run locally):${NC}"
    echo "   ./update_circuit_breaker_env.sh"
    echo
    echo -e "${BLUE}⏱️  Expected Timeline:${NC}"
    echo "   • Setup script: ~5-10 minutes"
    echo "   • Model download: ~10-20 minutes (depending on model size)"
    echo "   • Model loading: ~2-5 minutes"
    echo
    echo -e "${BLUE}🔧 Troubleshooting:${NC}"
    echo "   • If SSH fails, wait a few more minutes for instance to fully boot"
    echo "   • If model download fails, check Hugging Face token and model access"
    echo "   • If GPU issues, verify instance type supports GPU"
    echo
    echo -e "${BLUE}🗑️  Cleanup (when done):${NC}"
    echo "   aws ec2 terminate-instances --instance-ids $INSTANCE_ID --region $REGION"
    echo "   aws ec2 delete-security-group --group-id $SG_ID --region $REGION"
    echo
    echo -e "${GREEN}📁 Files created locally:${NC}"
    echo "   • vllm_setup.sh (copied to server)"
    echo "   • update_circuit_breaker_env.sh (run locally after vLLM is ready)"
}

main() {
    print_header
    
    check_prerequisites
    get_user_inputs
    
    echo
    print_step "Creating AWS resources..."
    create_security_group
    create_ec2_instance
    
    generate_setup_script
    generate_circuit_breaker_config
    copy_scripts_to_server
    
    show_instructions
}

# Run main function
main "$@"