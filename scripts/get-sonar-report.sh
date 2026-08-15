#!/usr/bin/env bash
set -eo pipefail

TOKEN="${SONAR_TOKEN:-$1}"
HOST="${SONAR_HOST_URL:-http://localhost:9000}"

if [ -z "$TOKEN" ]; then
  echo "Error: SONAR_TOKEN is required."
  echo "Usage: SONAR_TOKEN=<your_token> ./scripts/get-sonar-report.sh"
  echo "   or: ./scripts/get-sonar-report.sh <your_token>"
  exit 1
fi

echo "=========================================="
echo " 1. Quality Gate Status                   "
echo "=========================================="
curl -sS -u "$TOKEN:" "$HOST/api/qualitygates/project_status?projectKey=finance_tracker" | jq .

echo ""
echo "=========================================="
echo " 2. Project Metrics Summary               "
echo "=========================================="
curl -sS -u "$TOKEN:" "$HOST/api/measures/component?component=finance_tracker&metricKeys=coverage,bugs,vulnerabilities,code_smells,duplicated_lines_density,ncloc" | jq .

echo ""
echo "=========================================="
echo " 3. Detected Issues                       "
echo "=========================================="
curl -sS -u "$TOKEN:" "$HOST/api/issues/search?componentKeys=finance_tracker&ps=50" | jq '.issues[]? | {key, rule, severity, component, line, message}'
