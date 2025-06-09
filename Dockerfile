ARG OPTIMIZER_IMAGE=cosmwasm/optimizer:0.16.1
FROM ${OPTIMIZER_IMAGE}
RUN apk add --no-cache clang