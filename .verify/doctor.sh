#!/bin/sh
set -u

status=0
for tool in cargo git sh; do
  if command -v "$tool" >/dev/null 2>&1; then
    echo "doctor: $tool ok"
  else
    echo "doctor: $tool missing"
    status=1
  fi
done
exit $status
