ARG TERRA_VERSION=2.4.1

FROM ghcr.io/terra-money/core:sha-0324754

COPY scripts/setup_chain.sh /terra/setup.sh
# Make sript executable
RUN chmod +x /terra/setup.sh

ENV HOME /terra
WORKDIR $HOME

# rest server
EXPOSE 1317
# grpc
EXPOSE 9090
# grpc-web
EXPOSE 9091
# tendermint p2p
EXPOSE 26656
# tendermint rpc
EXPOSE 26657

ENTRYPOINT ["/terra/setup.sh"]