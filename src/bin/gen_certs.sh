#!/usr/bin/env bash

# Bash Strict Mode
set -euo pipefail
IFS=$'\n\t'

# Constants
readonly DEFAULT_CA_CN="RailyardCA"
readonly DEFAULT_CERTS_DIR="certs"

# Functions
to_snake_case() {
  local temp_string
  temp_string=$(echo "$1" | sed 's/\([a-z0-9]\)\([A-Z]\)/\1_\2/g')
  echo "$temp_string" | tr '[:upper:]' '[:lower:]'
}

generate_ca() {
  echo "Generating CA Cert"
  openssl genpkey -algorithm RSA -out "$1/$2.key" 2>/dev/null
  openssl req -new -key "$1/$2.key" -subj "/CN=$3" -out "$1/$2.csr" 2>/dev/null
  openssl x509 -req -in "$1/$2.csr" -signkey "$1/$2.key" -out "$1/$2.crt" 2>/dev/null
  rm "$1/$2.csr"
}

generate_node_cert() {
  echo "Generating Node Cert"
  openssl genpkey -algorithm RSA -out "$1/$2.key" 2>/dev/null
  openssl req -new -key "$1/$2.key" -subj "/CN=$3" -out "$1/$2.csr" 2>/dev/null
  openssl x509 -req -in "$1/$2.csr" -CA "$1/$4.crt" -CAkey "$1/$4.key" -CAcreateserial -out "$1/$2.crt" 2>/dev/null
  rm "$1/$2.csr"
}

if [[ -z "${1:-}" ]]; then
  echo "Node name required as an argument."
  exit 1
fi

CA_CN="${CA_CN:-$DEFAULT_CA_CN}"
CA_FILENAME=$(to_snake_case "$CA_CN")
CERTS_DIR="${CERTS_DIR:-$DEFAULT_CERTS_DIR}"

mkdir -p "$CERTS_DIR"

# Generate CA if not available
if [[ ! -f "$CERTS_DIR/$CA_FILENAME.key" ]]; then
  generate_ca "$CERTS_DIR" "$CA_FILENAME" "$CA_CN"
fi

# Generate and sign node cert
NODE_NAME=$1
NODE_FILENAME=$(to_snake_case "$NODE_NAME")
generate_node_cert "$CERTS_DIR" "$NODE_FILENAME" "$NODE_NAME" "$CA_FILENAME"
