.PHONY: help
help:
	@echo 'Makefile for g-man project. Check available tasks'

CI_REGISTRY_IMAGE := ghcr.io/glaeqen
VERSION := $(shell git describe --always)
DOCKER_IMAGE_CI := $(CI_REGISTRY_IMAGE)/g-man

.PHONY: check-if-clean-checkout
check-if-clean-checkout:
	@git ls-files -omsuk --directory --exclude-standard | sed q1 || \
		(echo Working directory is dirty! && exit 1)

.PHONY: build
build:
	cargo build --release

.PHONY: build-docker
build-docker: check-if-clean-checkout build
	! docker manifest inspect $(DOCKER_IMAGE_CI):$(VERSION)
	docker build -t $(DOCKER_IMAGE_CI):$(VERSION) .

# Local docker must be authenticated and authorized to do `write_registry` operations.
.PHONY: push-docker
push-docker: build-docker
	docker push $(DOCKER_IMAGE_CI):$(VERSION)
