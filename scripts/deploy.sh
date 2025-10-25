#!/usr/bin/env bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ENV_FILE="${SCRIPT_DIR}/../env.sh"

if [ -e "${ENV_FILE}" ]; then
	echo Loading environment from $ENV_FILE
	source "$ENV_FILE"
fi

echo "Deploying with CDK to ${swm_stage} in ${swm_account}:${swm_region}"
cd "${SCRIPT_DIR}/../infrastructure"
npm ci

npx cdk deploy \
	--ci \
	--require-approval never \
	--outputs-file "${SCRIPT_DIR}/.cdk-outputs.json"
