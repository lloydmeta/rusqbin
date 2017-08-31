OWNER=lloydmeta
IMAGE_NAME=rusqbin
VCS_REF=`git rev-parse --short HEAD`
IMAGE_VERSION=$(TRAVIS_TAG)
QNAME=$(OWNER)/$(IMAGE_NAME)

PWD=$(if $(TRAVIS_BUILD_DIR),$(TRAVIS_BUILD_DIR),$(pwd))

GIT_TAG=$(QNAME):$(VCS_REF)
BUILD_TAG=$(QNAME):$(IMAGE_VERSION)
LATEST_TAG=$(QNAME):latest

build: download-certs
	docker build \
		--squash \
		--build-arg CA_CERT=ca-certificates.crt \
		--build-arg VCS_REF=$(VCS_REF) \
		--build-arg BUILD_DATE=`date -u +"%Y-%m-%dT%H:%M:%SZ"` \
		-t $(GIT_TAG) .

download-certs:
	curl -o ca-certificates.crt https://curl.haxx.se/ca/cacert.pem

lint:
	docker run -it --rm -v "$(PWD)/Dockerfile:/Dockerfile:ro" redcoolbeans/dockerlint

tag:
	docker tag $(GIT_TAG) $(BUILD_TAG)
	docker tag $(GIT_TAG) $(LATEST_TAG)

login:
	@docker login -u "$(DOCKER_USER)" -p "$(DOCKER_PASS)"

push: login
	docker push $(GIT_TAG)
	docker push $(BUILD_TAG)
	docker push $(LATEST_TAG)