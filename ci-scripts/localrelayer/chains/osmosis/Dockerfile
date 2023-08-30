FROM alpine:3.15

COPY --from=osmolabs/osmosis-dev:19.0.0-rc0 /bin/osmosisd /bin/osmosisd

COPY scripts/setup_chain.sh /osmosis/setup.sh
# Make sript executable
RUN chmod +x /osmosis/setup.sh


ENV HOME /osmosis
WORKDIR $HOME

EXPOSE 26656
# RPC
EXPOSE 26657
# REST
EXPOSE 1317
# Note: uncomment the line below if you need pprof in localosmosis
# We disable it by default in out main Dockerfile for security reasons
# EXPOSE 6060

ENTRYPOINT ["/osmosis/setup.sh"]