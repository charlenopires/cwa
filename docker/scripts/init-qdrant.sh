#!/bin/bash
# Initialize Qdrant collections for CWA
# Usage: ./init-qdrant.sh [host] [port]

HOST="${1:-localhost}"
PORT="${2:-6333}"
BASE_URL="http://${HOST}:${PORT}"

echo "Initializing Qdrant collections at ${BASE_URL}..."

# Create memories collection (768 dimensions for nomic-embed-text)
curl -sf -X PUT "${BASE_URL}/collections/cwa_memories" \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": {
      "size": 768,
      "distance": "Cosine"
    }
  }' && echo " -> cwa_memories collection created" || echo " -> cwa_memories already exists or error"

# Create terms collection for domain terminology embeddings
curl -sf -X PUT "${BASE_URL}/collections/cwa_terms" \
  -H "Content-Type: application/json" \
  -d '{
    "vectors": {
      "size": 768,
      "distance": "Cosine"
    }
  }' && echo " -> cwa_terms collection created" || echo " -> cwa_terms already exists or error"

echo "Qdrant initialization complete."
