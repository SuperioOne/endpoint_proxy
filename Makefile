PROJECT_NAME     := $(shell cargo metadata --no-deps --offline --format-version 1 | jq -r ".packages[0].name")
PROJECT_VER      := $(shell cargo metadata --no-deps --offline --format-version 1 | jq -r ".packages[0].version")
BIN_DIR          := ./bin
DOCKER_TEMPLATE  := ./containers/Dockerfile.template
BUILD_CONFIG     := ./build.config.json
PROJECT_SRCS     := $(shell find . -type f -iregex "\./src/.*") \
					$(shell find . -type f -iname Cargo.toml) \
					$(shell find . -type f -iname Cargo.lock)
DIST_DIR         := $(BIN_DIR)/dist
BINARY_TARGETS   := $(shell jq -r '.binary[]' "$(BUILD_CONFIG)")

fn_output_path    = $(BIN_DIR)/$(1)/$(PROJECT_NAME)
fn_target_path    = target/$(1)/$(PROJECT_NAME)

.PHONY: help
help:
	@echo "RECEIPES"
	@echo "  build                : Generates server binary for the current system's CPU architecture and OS."
	@echo "  build-all            : Cross compiles everything."
	@echo "  clean                : Clears all build directories."
	@echo "  gen-dockerfiles      : Generates dockerfiles for all supported architectures."
	@echo "  pack                 : Compresses (.tar.gz) all compiled targets under the bin directory."
	@echo ""
	@echo "Specific build targets:"
	@echo "  build-aarch64-gnu   : linux/arm64/v8, self contained with gnu."
	@echo "  build-x86-64-gnu    : linux/amd64, self contained with gnu."
	@echo "  build-x86-64-v3-gnu : linux/amd64/v3, self contained with gnu."

# Default toolchain

.PHONY: build
build: $(PROJECT_SRCS)
	@echo "Building binaries for the current system's architecture."
	@cargo build --release
	@install -D $(call fn_target_path,release) $(call fn_output_path,release)

# x86-64

.PHONY: build-x86-64-gnu
build-x86-64-gnu: $(call fn_output_path,x86-64-gnu)

$(call fn_output_path,x86-64-gnu) &: $(PROJECT_SRCS)
	@echo "Building for x86_64-unknown-linux-gnu"
	@export RUSTFLAGS="-Ctarget-cpu=x86-64 -Ctarget-feature=+crt-static" && \
		cargo build --target=x86_64-unknown-linux-gnu --release
	@install -D $(call fn_target_path,x86_64-unknown-linux-gnu/release) \
		$(call fn_output_path,x86-64-gnu)

# x86-64-v3

.PHONY: build-x86-64-v3-gnu
build-x86-64-v3-gnu: $(call fn_output_path,x86-64-v3-gnu)

$(call fn_output_path,x86-64-v3-gnu) &: $(PROJECT_SRCS)
	@echo "Building for x86_64-v3-unknown-linux-gnu"
	@export RUSTFLAGS="-Ctarget-cpu=x86-64-v3 -Ctarget-feature=+crt-static" && \
		cargo build --target=x86_64-unknown-linux-gnu --release
	@install -D $(call fn_target_path,x86_64-unknown-linux-gnu/release) \
		$(call fn_output_path,x86-64-v3-gnu)

# ARM64/v8

.PHONY: build-aarch64-gnu
build-aarch64-gnu: $(call fn_output_path,aarch64-gnu)

$(call fn_output_path,aarch64-gnu) &: $(PROJECT_SRCS)
	@echo "Building for aarch64-unknown-linux-gnu"
	@export RUSTFLAGS="-Clinker=aarch64-linux-gnu-gcc -Ctarget-feature=+crt-static" && \
		cargo build --target=aarch64-unknown-linux-gnu --release
	@install -D $(call fn_target_path,aarch64-unknown-linux-gnu/release) \
		$(call fn_output_path,aarch64-gnu)

.PHONY: build-all
build-all: $(addprefix build-,$(BINARY_TARGETS))

.PHONY: pack
pack: build-all
	@install -d $(DIST_DIR)
	@for target in $(BINARY_TARGETS); do \
		if [ -f "$(BIN_DIR)/$${target}/$(PROJECT_NAME)" ]; then \
			OUTPUT_TARGZ="$(DIST_DIR)/$(PROJECT_NAME)_$(PROJECT_VER)_$${target}.tar.gz"; \
			tar -czf "$${OUTPUT_TARGZ}" -C "$(BIN_DIR)/" "$${target}"; \
			echo "Packed $${target}.tar.gz"; \
			sha256sum "$${OUTPUT_TARGZ}" > "$${OUTPUT_TARGZ}.sha256"; \
			echo "Generated $${target}.tar.gz.sha256"; \
		fi; \
	done;

.PHONY: gen-dockerfiles
gen-dockerfiles:
	@install -d "$(BIN_DIR)/dockerfiles"
	@for entry in $$(jq -rc '.oci.images[]' "$(BUILD_CONFIG)"); do \
			export PLATFORM="$$(echo $$entry | jq -r '.platform')"; \
			export TARGET="$$(echo $$entry | jq -r '.target')"; \
			export BASE_CONTAINER_IMAGE="$$(echo $$entry | jq -r '.base_image')"; \
			export EXE_DIR="$(BIN_DIR)/$$TARGET"; \
			echo "Creating $${TARGET}.dockerfile"; \
			cat "$(DOCKER_TEMPLATE)" | envsubst > "$(BIN_DIR)/dockerfiles/$${TARGET}.Dockerfile"; \
		done;
	@echo "Creating annotation.json"
	@REVISION="$$(git rev-parse --verify HEAD)"; \
		cargo metadata \
			--no-deps \
			--frozen \
			--format-version 1 \
			--manifest-path "./Cargo.toml" \
		| jq -r \
			--arg revision "$$REVISION" \
			'.packages[0] | { title:.name, version:.version, url:.homepage, licenses:.license, documentation:.documentation, source:.repository, description:.description, authors:(.authors | join(";")), revision: $$revision}' \
		> "$(BIN_DIR)/dockerfiles/annotations.json";

.PHONY: check
check: init
	@cargo check --all-features

.PHONY: clean
clean:
	@echo "Cleaning artifacts"
	@cargo clean
	@if [ -d "$(BIN_DIR)" ]; then rm -r "$(BIN_DIR)"; fi;
	@echo "Clean completed"
