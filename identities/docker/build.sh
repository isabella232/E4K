docker build -f ./identities/docker/builder.dockerfile . -t builder && \
docker build -f ./identities/docker/server.dockerfile . -t local/serverd && \
docker build -f ./identities/docker/agent.dockerfile . -t local/agentd
docker build -f ./identities/docker/manager.dockerfile . -t local/managerd
