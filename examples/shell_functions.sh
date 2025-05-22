#!/bin/bash

# Simple function to check if a number is even or odd
check_even_odd() {
  local number=${1:-$(date +%H)}
  
  echo "Checking if $number is even or odd..."
  
  if (( number % 2 == 0 )); then
    echo "$number is EVEN"
    return 0
  else
    echo "$number is ODD"
    return 1
  fi
}

# Function to backup a directory with confirmation
backup_dir() {
  local src_dir="$1"
  local dest_dir="${2:-/tmp/backups}"
  
  # Check if source directory exists
  if [ ! -d "$src_dir" ]; then
    echo "Error: Source directory '$src_dir' does not exist!"
    return 1
  fi
  
  # Create destination directory if it doesn't exist
  if [ ! -d "$dest_dir" ]; then
    echo "Creating destination directory: $dest_dir"
    mkdir -p "$dest_dir"
  fi
  
  # Get timestamp for backup name
  local timestamp=$(date +%Y%m%d%H%M%S)
  local backup_name=$(basename "$src_dir")
  local backup_file="$dest_dir/${backup_name}_${timestamp}.tar.gz"
  
  # Confirm before backup
  echo "Will backup '$src_dir' to '$backup_file'"
  read -p "Continue? (y/n): " confirm
  
  if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
    echo "Backup cancelled"
    return 1
  fi
  
  # Create backup
  echo "Creating backup..."
  tar -czf "$backup_file" -C "$(dirname "$src_dir")" "$(basename "$src_dir")"
  
  # Check if backup was successful
  if [ $? -eq 0 ]; then
    echo "Backup created successfully: $backup_file"
    return 0
  else
    echo "Backup failed!"
    return 1
  fi
}

# Function to deploy to different environments
deploy_env() {
  local env="${1:-dev}"
  local version="$2"
  
  if [ -z "$version" ]; then
    echo "Error: Version parameter is required!"
    echo "Usage: deploy_env [dev|staging|prod] <version>"
    return 1
  fi
  
  echo "Preparing to deploy version $version to $env environment..."
  
  case "$env" in
    dev)
      echo "Deploying to DEV environment..."
      echo "Running tests..."
      sleep 1
      echo "Pushing to dev server..."
      sleep 1
      ;;
    staging)
      echo "Deploying to STAGING environment..."
      echo "Running integration tests..."
      sleep 1
      echo "Getting approval for staging deployment..."
      read -p "Approve staging deployment? (y/n): " confirm
      if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Deployment cancelled"
        return 1
      fi
      echo "Pushing to staging server..."
      sleep 1
      ;;
    prod)
      echo "Deploying to PRODUCTION environment..."
      echo "Verifying all tests passed..."
      sleep 1
      echo "CRITICAL: Getting approval for production deployment..."
      read -p "Approve PRODUCTION deployment? (y/n): " confirm
      if [[ "$confirm" != "y" && "$confirm" != "Y" ]]; then
        echo "Deployment cancelled"
        return 1
      fi
      echo "Pushing to production server..."
      sleep 1
      echo "Verifying deployment..."
      sleep 1
      ;;
    *)
      echo "Error: Unknown environment '$env'!"
      echo "Valid environments: dev, staging, prod"
      return 1
      ;;
  esac
  
  echo "Deployment to $env complete!"
  return 0
}