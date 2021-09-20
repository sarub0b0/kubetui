

build:
	cargo build ${FLAGS}

run:
	cargo run ${FLAGS}

debug:
	RUST_LOG=debug cargo run

e2e-test: build
	kubectl delete -f example/
	kubectl apply -f example/
	RUST_LOG=debug target/debug/kubetui
.PHONY: e2e-test

re-deploy:
	kubectl delete -f example/
	kubectl apply -f example/
