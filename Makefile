PROJ_DIR := $(dir $(abspath $(lastword $(MAKEFILE_LIST))))

# TODO: these values are currently duplicated in lib.rs. There's a PR open in
# duckdb-rs that fixes this
EXTENSION_NAME=hello
MINIMUM_DUCKDB_VERSION=v0.0.1


# Include makefiles from DuckDB
include extension-ci-tools/makefiles/c_api_extensions/base.Makefile
include extension-ci-tools/makefiles/c_api_extensions/rust.Makefile

.EXPORT_ALL_VARIABLES:
# In order to use buildtime_bindgen
# you need to build duckdb locally and export these envs
LD_LIBRARY_PATH=$(PWD)/duckdb:$LD_LIBRARY_PATH
DUCKDB_LIB_DIR=$(PWD)/duckdb
DUCKDB_INCLUDE_DIR=$(PWD)/duckdb
DUCKDB_STATIC=0

.PHONY: configure debug release test example clean clean_all

all: configure debug

configure: venv platform extension_version
debug: build_extension_library_debug build_extension_with_metadata_debug
release: build_extension_library_release build_extension_with_metadata_release

test:
	@cargo test

example:
	@cargo run --example basic_usage

run: # debug
	@duckdb -unsigned < examples/run.sql

clean: clean_build clean_rust
clean_all: clean_configure clean
