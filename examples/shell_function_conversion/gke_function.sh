#!/bin/bash

function kseal() {
    # Check if kubeseal is installed
    if ! command -v kubeseal &> /dev/null; then
        echo "Error: kubeseal is not installed. Please install it first."
        return 1
    fi

    # Validate input
    if [ $# -ne 2 ]; then
        echo "Usage: kseal [dev|prod] <secret-value>"
        return 1
    fi

    local ENV="$1"
    local SECRET_VALUE="$2"

    # Use the existing gke function to set up context
    gke "$ENV"

    echo ""

    # Create and output sealed secret directly to CLI
    echo -n "$SECRET_VALUE" | kubeseal \
        --raw \
        --scope cluster-wide
}
