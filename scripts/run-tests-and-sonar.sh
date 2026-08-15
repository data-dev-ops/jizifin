#!/usr/bin/env bash
set -eo pipefail

echo "=========================================="
echo " 1. Running Frontend Tests (Vitest)       "
echo "=========================================="
npm --prefix frontend run test:coverage

echo "=========================================="
echo " 2. Running Backend Tests (Pytest)         "
echo "=========================================="
(
  cd backend
  uv run pytest --cov=app --cov-report=xml:coverage.xml --cov-report=term
)

echo "=========================================="
echo " 3. Coverage Reports Generated            "
echo "   - Frontend: frontend/coverage/lcov.info"
echo "   - Backend:  backend/coverage.xml       "
echo "=========================================="

SONAR_HOST="${SONAR_HOST_URL:-http://host.docker.internal:9000}"

if [ -n "$SONAR_TOKEN" ]; then
  echo "Executing SonarScanner CLI with provided SONAR_TOKEN against $SONAR_HOST..."
  docker run --rm -v "$(pwd):/usr/src" sonarsource/sonar-scanner-cli \
    -Dsonar.host.url="$SONAR_HOST" \
    -Dsonar.token="$SONAR_TOKEN"
else
  echo "=========================================="
  echo " ℹ️ Local SonarScanner Analysis Notice    "
  echo "=========================================="
  echo "SonarQube requires an authentication token by default."
  echo ""
  echo "1. Log into your local SonarQube instance at http://localhost:9000 (Default: admin / admin)."
  echo "2. Generate an Analysis Token (User > Account > Security > Generate Token)."
  echo "3. Run the scanner with your token:"
  echo "   SONAR_TOKEN=<your_token> ./scripts/run-tests-and-sonar.sh"
  echo "   OR"
  echo "   docker run --rm -v \"\$(pwd):/usr/src\" sonarsource/sonar-scanner-cli -Dsonar.host.url=\"$SONAR_HOST\" -Dsonar.token=\"<your_token>\""
  echo ""
  echo "Alternatively, disable 'Force user authentication' in SonarQube: Administration > Security."
  echo "=========================================="
fi
