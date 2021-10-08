

build:
	cargo build ${FLAGS}

run:
	cargo run ${FLAGS}

debug:
	RUST_LOG=debug cargo run

e2e-test: build re-deploy
	RUST_LOG=debug target/debug/kubetui
.PHONY: e2e-test

re-deploy:
	-kubectl delete -f examples/manifests
	-kubectl apply -f examples/manifests
