.PHONY: build build-front dev deb install check fmt clean

build:        ## Compile the module binary
	cargo build --release --bin kubuno-forum

build-front:  ## Build the frontend bundle (dist/entry.js)
	cd frontend && npm run build

dev:          ## Run the module in watch mode
	cargo watch -q -c -x 'run --bin kubuno-forum'

deb:          ## Build the Debian package
	bash build_deb.sh

install:      ## Build and install the package
	bash build_deb.sh --install

check:        ## cargo check + frontend typecheck
	cargo check --bin kubuno-forum
	cd frontend && npm run typecheck

fmt:          ## Format the code
	cargo fmt

clean:        ## Clean build artifacts
	cargo clean
	rm -rf frontend/dist dist
