package-install:
	apt-get install -y libssl-dev pkg-config


test/bin/kind:
	mkdir -p test/bin
	curl -Lo test/bin/kind https://kind.sigs.k8s.io/dl/v0.19.0/kind-linux-amd64
	chmod +x test/bin/kind

create-kind: test/bin/kind
	test/bin/kind create cluster --name kubetui

delete-kind: test/bin/kind
	test/bin/kind delete cluster --name kubetui

deploy-nginx-gateway-fabric:
	kubectl apply -f https://github.com/kubernetes-sigs/gateway-api/releases/download/v1.0.0/standard-install.yaml
	kubectl apply -f https://github.com/nginxinc/nginx-gateway-fabric/releases/download/v1.2.0/crds.yaml
	kubectl apply -f https://github.com/nginxinc/nginx-gateway-fabric/releases/download/v1.2.0/nginx-gateway.yaml


deploy: 
	-kubectl apply -f test/manifests

purge:
	-kubectl delete -f test/manifests

clean: purge delete-kind

