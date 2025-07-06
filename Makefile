NGINX_GATEWAY_FABRIC_VERSION ?= v2.0.1

OS := $(shell uname -s | tr '[:upper:]' '[:lower:]')

ifeq ($(OS),darwin)
	KIND_URL := https://kind.sigs.k8s.io/dl/latest/kind-darwin-arm64
else ifeq ($(OS),linux)
	KIND_URL := https://kind.sigs.k8s.io/dl/latest/kind-linux-amd64
else
	$(error Unsupported OS: $(OS))
endif

test/bin/kind:
	mkdir -p test/bin
	curl -Lo test/bin/kind $(KIND_URL)
	chmod +x test/bin/kind

create-kind: test/bin/kind
	test/bin/kind create cluster --name kubetui

delete-kind: test/bin/kind
	test/bin/kind delete cluster --name kubetui

install-gateway-controller:
	kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/standard-install.yaml

install-nginx-gateway-fabric:
	kubectl kustomize "https://github.com/nginx/nginx-gateway-fabric/config/crd/gateway-api/standard?ref=$(NGINX_GATEWAY_FABRIC_VERSION)" | kubectl apply -f -
	helm install ngf oci://ghcr.io/nginx/charts/nginx-gateway-fabric --create-namespace -n nginx-gateway

deploy-manifests:
	-kubectl apply -f test/manifests -R

deploy: create-kind install-gateway-controller install-nginx-gateway-fabric deploy-manifests

purge:
	-kubectl delete -f test/manifests

clean: delete-kind
	-rm test/bin/kind

