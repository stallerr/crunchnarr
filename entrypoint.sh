#!/bin/sh
set -e

# Start the API server in the background
crunchy-api &
API_PID=$!

# Shut down both processes on exit
cleanup() {
  kill "$API_PID" 2>/dev/null
  wait "$API_PID" 2>/dev/null
  exit 0
}
trap cleanup INT TERM

# Start Next.js in the foreground
export HOSTNAME=0.0.0.0
export PORT=3000
exec node server.js
