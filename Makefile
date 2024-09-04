clean:
	-starship stop --config ibc-tests/configs/starship.yaml
	-pkill -f "port-forward"

restart-cluster:
	-kind delete cluster --name starship
	-kind create cluster --name starship
	-kubectl cluster-info --context kind-starship
start:
	-starship start --config ibc-tests/configs/starship.yaml



# loom 
# HERMES_ENABLED=false

# .PHONY: build
# build:
# 	@DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 docker-compose -f docker-compose.yml build

# .PHONY: setup-chains
# setup-chains:
# 	./setup_docker.sh setup_chains
# 	$(MAKE) build

# .PHONY: setup-chains-with-hermes
# setup-chains-with-hermes:
# 	./setup_docker.sh setup_chains_with_hermes
# 	$(MAKE) build

# start-chains:
# 	$(MAKE) setup-chains
# 	@docker-compose -f docker-compose.yml up

# start-chainsd:
# 	$(MAKE) setup-chains
# 	@docker-compose -f docker-compose.yml up -d

# start-chains-with-hermes:
# 	setup-chains-with-hermes
# 	@docker-compose -f docker-compose.yml up

# start-chains-with-hermesd:
# 	setup-chains-with-hermes
# 	@docker-compose -f docker-compose.yml up -d

# .PHONY: stop
# stop:
# 	@docker-compose -f docker-compose.yml down -t 3


# restart: stop
# 	@docker-compose -f docker-compose.yml up --force-recreate 

# restartd: stop
# 	@docker-compose -f docker-compose.yml up --force-recreate -d

# clean:
# 	@rm -rfI ./.config