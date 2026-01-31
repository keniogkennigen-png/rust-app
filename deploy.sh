#!/bin/bash
# Deployment Script for RustChat
# This script helps you deploy to various platforms

set -e

echo "🚀 RustChat Deployment Script"
echo "=============================="
echo ""
echo "Choose your deployment platform:"
echo "1) Railway (Recommended - Easiest)"
echo "2) Render.com"
echo "3) Fly.io"
echo "4) Docker (Local/Server)"
echo "5) Coolify"
echo "6) Generate deployment files only"
echo ""
read -p "Enter option (1-6): " option

case $option in
    1)
        echo ""
        echo "🚂 Deploying to Railway..."
        echo ""
        echo "1. Make sure you have railway CLI installed: npm install -g @railway/cli"
        echo "2. Login: railway login"
        echo "3. Deploy: railway up"
        echo ""
        echo "Or deploy via GitHub:"
        echo "- Go to https://railway.app"
        echo "- Click 'New Project' → 'Deploy from GitHub'"
        echo "- Select: keniogkennigen-png/rust-app"
        echo "- Done! 🎉"
        ;;
    
    2)
        echo ""
        echo "🎨 Deploying to Render..."
        echo ""
        echo "1. Go to https://render.com"
        echo "2. Create account and connect GitHub"
        echo "3. New Web Service:"
        echo "   - Build Command: cargo build --release"
        echo "   - Start Command: ./target/release/rust_chat"
        echo "4. Create and wait for deployment"
        echo "5. Your URL: https://your-app.onrender.com"
        ;;
    
    3)
        echo ""
        echo "🦅 Deploying to Fly.io..."
        echo ""
        echo "1. Install: curl -L https://fly.io/install.sh | sh"
        echo "2. Login: flyctl auth login"
        echo "3. Launch: fly launch"
        echo "4. Deploy: fly deploy"
        echo ""
        echo "Your app will be at: https://rust-chat.fly.dev"
        ;;
    
    4)
        echo ""
        echo "🐳 Deploying with Docker..."
        echo ""
        echo "Building Docker image..."
        docker build -t rust-chat .
        echo ""
        echo "Running container..."
        docker run -d -p 3030:3030 --name rust-chat-app rust-chat
        echo ""
        echo "✅ App running at: http://localhost:3030"
        echo ""
        echo "To stop: docker stop rust-chat-app"
        echo "To restart: docker restart rust-chat-app"
        ;;
    
    5)
        echo ""
        echo "🔥 Deploying to Coolify..."
        echo ""
        echo "1. Install Coolify on your server or use cloud version"
        echo "2. Create new project"
        echo "3. Connect GitHub repository"
        echo "4. Configure:"
        echo "   - Build Pack: Docker"
        echo "   - Port: 3030"
        echo "5. Deploy!"
        ;;
    
    6)
        echo ""
        echo "📦 Generating deployment files..."
        echo ""
        
        # Generate systemd service
        cat > rust-chat.service << 'EOF'
[Unit]
Description=RustChat Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/rust-chat
ExecStart=/opt/rust-chat/target/release/rust_chat
Restart=always
RestartSec=5
Environment=PORT=3030
Environment=HOST=0.0.0.0

[Install]
WantedBy=multi-user.target
EOF
        
        echo "✅ Created: rust-chat.service"
        echo "   Install with: sudo cp rust-chat.service /etc/systemd/system/"
        echo "   Enable: sudo systemctl enable rust-chat"
        echo "   Start: sudo systemctl start rust-chat"
        echo ""
        
        # Generate nginx config
        cat > nginx.conf << 'EOF'
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:3030;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
    }
}
EOF
        
        echo "✅ Created: nginx.conf"
        echo "   Install with: sudo cp nginx.conf /etc/nginx/sites-available/rust-chat"
        echo "   Enable: sudo ln -s /etc/nginx/sites-available/rust-chat /etc/nginx/sites-enabled/"
        echo ""
        
        echo "📝 Files generated successfully!"
        ;;
    
    *)
        echo "❌ Invalid option"
        exit 1
        ;;
esac

echo ""
echo "=============================="
echo "For help, check:"
echo "- DEPLOY.md"
echo "- DEPLOY_RAILWAY.md"
echo "- DEPLOY_DOCKER.md"
echo ""
echo "Your repo: https://github.com/keniogkennigen-png/rust-app"
