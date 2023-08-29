FROM alpine:3.15

COPY ./andromedad /bin/andromedad
# Make binary executable
RUN chmod +x /bin/andromedad

COPY scripts/setup_chain.sh /andromeda/setup.sh
# Make sript executable
RUN chmod +x /andromeda/setup.sh


ENV HOME /andromeda
WORKDIR $HOME

EXPOSE 26656
# RPC
EXPOSE 26657
# Rest
EXPOSE 1317
# Note: uncomment the line below if you need pprof in localosmosis
# We disable it by default in out main Dockerfile for security reasons
# EXPOSE 6060

ENTRYPOINT ["/andromeda/setup.sh"]