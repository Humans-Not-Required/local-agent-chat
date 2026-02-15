#!/bin/bash
# discover-service.sh — Find local-agent-chat instances on the network
#
# Method 1: mDNS (requires avahi-browse or dns-sd)
# Method 2: HTTP discovery endpoint
#
# Usage:
#   ./discover-service.sh              # Try mDNS first, fall back to known hosts
#   ./discover-service.sh 192.168.0.79 # Query a specific host
#   CHAT_PORT=3006 ./discover-service.sh 192.168.0.79

set -euo pipefail

CHAT_PORT="${CHAT_PORT:-8000}"

# --- Method 1: mDNS discovery ---
discover_mdns() {
    if command -v avahi-browse &>/dev/null; then
        echo "🔍 Searching for _agentchat._tcp.local via mDNS..."
        # Browse for 3 seconds, resolve addresses
        timeout 3 avahi-browse -rpt _agentchat._tcp 2>/dev/null | while IFS=';' read -r iface proto name stype domain hostname ip port txt; do
            if [[ "$iface" == "=" ]]; then
                echo "✅ Found: ${name} at http://${ip}:${port}"
                echo "   Host: ${hostname}"
                echo "   TXT:  ${txt}"
            fi
        done
        return 0
    elif command -v dns-sd &>/dev/null; then
        echo "🔍 Searching for _agentchat._tcp.local via dns-sd..."
        timeout 3 dns-sd -B _agentchat._tcp local. 2>/dev/null || true
        return 0
    else
        echo "⚠️  No mDNS browser found (install avahi-utils or use Method 2)"
        return 1
    fi
}

# --- Method 2: HTTP discovery endpoint ---
discover_http() {
    local host="${1:-localhost}"
    local port="${2:-$CHAT_PORT}"
    local url="http://${host}:${port}/api/v1/discover"

    echo "🔍 Querying ${url}..."
    local response
    if response=$(curl -sf --connect-timeout 2 "$url" 2>/dev/null); then
        echo "✅ Found service:"
        echo "$response" | python3 -m json.tool 2>/dev/null || echo "$response"
    else
        echo "❌ No service at ${host}:${port}"
        return 1
    fi
}

# --- Main ---
if [[ $# -ge 1 ]]; then
    # Direct host query
    discover_http "$1" "${2:-$CHAT_PORT}"
else
    # Try mDNS first
    if ! discover_mdns 2>/dev/null; then
        echo ""
        echo "Trying common LAN addresses..."
        for host in localhost 192.168.0.79 192.168.1.1; do
            discover_http "$host" "$CHAT_PORT" 2>/dev/null && break || true
        done
    fi
fi
