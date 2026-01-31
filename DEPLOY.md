# Deployment Guide - RustChat

## Quick Deploy (Free Options)

### Option 1: Railway (Recommended - Easiest)
1. Go to [railway.app](https://railway.app)
2. Sign up with GitHub
3. Click "New Project" → "Deploy from GitHub"
4. Select `keniogkennigen-png/rust-app`
5. Railway auto-detects Dockerfile and deploys
6. Get free URL like: `https://rust-chat-production.up.railway.app`

### Option 2: Render.com
1. Go to [render.com](https://render.com)
2. Create free account
3. New Web Service → Connect GitHub repo
4. Settings:
   - Build Command: `cargo build --release`
   - Start Command: `./target/release/rust_chat`
5. Create and get free URL

### Option 3: Fly.io
1. Install flyctl: `curl -L https://fly.io/install.sh | sh`
2. Run: `fly launch`
3. Follow prompts
4. Deploy: `fly deploy`

### Option 4: Docker (Any Server)
```bash
docker build -t rust-chat .
docker run -d -p 3030:3030 rust-chat
```

## Your GitHub Repo
https://github.com/keniogkennigen-png/rust-app

## Features Ready
- WebSocket chat
- User registration/login
- Contact management
- Typing indicators
- Online status
- Sci-fi themed UI

## Need Help?
Check DEPLOY_RAILWAY.md or DEPLOY_DOCKER.md for detailed instructions.
