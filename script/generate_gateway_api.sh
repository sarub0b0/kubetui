#!/bin/bash

# ------------------------------------------------------------------------------
# This script will automatically generate API updates for new Gateway API
# releases. Update the $VERSION to the new release version before executing.
#
# This script requires kopium, which can be installed with:
#
#   cargo install kopium
#
# See: https://github.com/kube-rs/kopium
# ------------------------------------------------------------------------------

# set -eou pipefail

KOPIUM_OPTIONS="--schema=disabled --derive=Default --derive=PartialEq --derive=Eq --derive=PartialOrd --derive=Ord"

VERSION="v1.0.0"

VERSIONS=(
	v1
	v1beta1
	v1alpha2
)

STANDARD_APIS=(
	gatewayclasses
	gateways
	httproutes
	referencegrants
)

EXPERIMENTAL_APIS=(
	gatewayclasses
	gateways
	httproutes
	referencegrants
	grpcroutes
	tcproutes
	tlsroutes
	udproutes
)

rm -rf tmp/apis/

mkdir -p tmp/apis/
cat <<EOF >tmp/apis/mod.rs
pub mod v1;
pub mod v1beta1;
pub mod v1alpha2;
EOF

for v in "${VERSIONS[@]}"; do

	mkdir -p tmp/apis/$v/

	echo "// WARNING! generated file do not edit" >tmp/apis/$v/mod.rs

	for API in "${STANDARD_APIS[@]}"; do
		echo "generating standard api ${API}"
		curl -sSL "https://raw.githubusercontent.com/kubernetes-sigs/gateway-api/main/config/crd/standard/gateway.networking.k8s.io_${API}.yaml?ref=${VERSION}" | kopium --api-version=$v $KOPIUM_OPTIONS -f - >tmp/apis/$v/${API}.rs
		if [ $? -eq 0 ]; then
			echo -e "mod ${API};\npub use ${API}::*;\n" >>tmp/apis/$v/mod.rs
		else
			rm tmp/apis/$v/${API}.rs
		fi
	done

	echo "// WARNING! generated file do not edit" >tmp/apis/$v/mod.rs

	for API in "${EXPERIMENTAL_APIS[@]}"; do
		echo "generating experimental api $API"
		curl -sSL "https://raw.githubusercontent.com/kubernetes-sigs/gateway-api/main/config/crd/experimental/gateway.networking.k8s.io_${API}.yaml?ref=${VERSION}" | kopium --api-version=$v $KOPIUM_OPTIONS -f - >tmp/apis/$v/${API}.rs

		if [ $? -eq 0 ]; then
			echo -e "mod ${API};\npub use ${API}::*;\n" >>tmp/apis/$v/mod.rs
		else
			rm tmp/apis/$v/${API}.rs
		fi
	done
done
