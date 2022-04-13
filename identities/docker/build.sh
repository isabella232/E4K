docker build -f ./identities/docker/builder.dockerfile . -t builder && \
docker build -f ./identities/docker/server.dockerfile . -t local/serverd && \
docker build -f ./identities/docker/agent.dockerfile . -t local/agentd
