FROM alpine:3.15

COPY --from=ghcr.io/terra-money/core:sha-a03a065 /usr/local/bin/terrad /bin/terrad

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