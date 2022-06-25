apt:
	apt-get install -y libxcb-shape0-dev libxcb-render0-dev libxcb-xfixes0-dev libssl-dev pkg-config

build:
	cargo build ${FLAGS}

run:
	cargo run ${FLAGS}

debug:
	RUST_LOG=debug cargo run

e2e-test: build re-deploy
	RUST_LOG=debug target/debug/kubetui
.PHONY: e2e-test

re-deploy: purge deploy

deploy:
	-kubectl apply -f examples/manifests

purge:
	-kubectl delete -f examples/manifests
