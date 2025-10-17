#!/usr/bin/env bash

set -e


INPUT_PATH=$1

if [[ -z "$INPUT_PATH" ]]; then
	echo "Usage: $0 <FILENAME>"
	exit 1
fi

if ! command -v nccopy &> /dev/null; then
	echo This script requires nccopy which can be installed with
	echo
	echo brew install hdf5 netcdf
	echo
	exit 1
fi

INPUT_FILE=`basename $INPUT_PATH`
INPUT_DIR=`dirname $INPUT_PATH`
INPUT_BASE="${INPUT_FILE%.*}"
EXT="${INPUT_FILE##*.}"
OUTPUT_PATH="${INPUT_DIR}/${INPUT_BASE}.classic.${EXT}"

nccopy -k classic "$INPUT_PATH" "$OUTPUT_PATH"
