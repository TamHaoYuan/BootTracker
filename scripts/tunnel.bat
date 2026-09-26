@echo off
chcp 65001 >nul 2>&1
title BootTracker Cloudflare Tunnel
echo Starting Cloudflare Tunnel for BootTracker...
"%~dp0..\vendor\cloudflared.exe" tunnel --url http://localhost:18792
pause
