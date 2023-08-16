#!/bin/sh
set -e

DEFAULT_CHAIN_A_ID="localosmosis-a"
DEFAULT_CHAIN_A_MNEMONIC="black frequent sponsor nice claim rally hunt suit parent size stumble expire forest avocado mistake agree trend witness lounge shiver image smoke stool chicken"
DEFAULT_CHAIN_B_ID="localosmosis-b"
DEFAULT_CHAIN_B_MNEMONIC="black frequent sponsor nice claim rally hunt suit parent size stumble expire forest avocado mistake agree trend witness lounge shiver image smoke stool chicken"

CHAIN_A_MNEMONIC=${CHAIN_A_MNEMONIC:-$DEFAULT_CHAIN_A_MNEMONIC}
CHAIN_A_ID=${CHAIN_A_ID:-$DEFAULT_CHAIN_A_ID}
CHAIN_B_MNEMONIC=${CHAIN_B_MNEMONIC:-$DEFAULT_CHAIN_B_MNEMONIC}
CHAIN_B_ID=${CHAIN_B_ID:-$DEFAULT_CHAIN_B_ID}

install_prerequisites(){
    echo "🧰 Install prerequisites"
    apt update
    apt -y install curl
}

add_keys(){

    echo "🔑 Adding key for $CHAIN_A_ID"
    mkdir -p /home/hermes/mnemonics/
    echo $CHAIN_A_MNEMONIC > /home/hermes/mnemonics/$CHAIN_A_ID

    hermes keys add \
    --chain $CHAIN_A_ID \
    --mnemonic-file /home/hermes/mnemonics/$CHAIN_A_ID \
    --key-name $CHAIN_A_ID \
    --overwrite

    echo "🔑 Adding key for $CHAIN_B_ID"
    echo $CHAIN_B_MNEMONIC > /home/hermes/mnemonics/$CHAIN_B_ID

    hermes keys add \
    --chain $CHAIN_B_ID \
    --mnemonic-file /home/hermes/mnemonics/$CHAIN_B_ID \
    --key-name $CHAIN_B_ID \
    --overwrite
}

create_channel(){
    echo "🥱 Waiting for $CHAIN_A_ID to start"
    COUNTER=0
    until $(curl --output /dev/null --silent --fail --head http://$CHAIN_A_ID:26657/status); do
        echo 'Waiting for rpc'
        sleep 2
    done
    # GRPC Return an error with code 1 for HTTP/0.9 error. Its different than port not open hence we can check for error code 1 and wait for it to be enabled
    until [ $(curl --output /dev/null --silent --fail http://$CHAIN_A_ID:9090 || echo $?) -eq 1 ]; do
        echo 'Waiting for grpc'
        sleep 2
    done

    echo "🥱 Waiting for $CHAIN_B_ID to start"
    COUNTER=0
    until $(curl --output /dev/null --silent --fail --head http://$CHAIN_B_ID:26657/status); do
        echo 'Waiting for rpc'
        sleep 5
    done
    until [ $(curl --output /dev/null --silent --fail http://$CHAIN_B_ID:9090 || echo $?) -eq 1 ]; do
        echo 'Waiting for grpc'
        sleep 5
    done

    echo "📺 Creating channel $CHAIN_A_ID <> $CHAIN_B_ID"
    hermes create channel \
    --a-chain $CHAIN_A_ID \
    --b-chain $CHAIN_B_ID \
    --a-port transfer \
    --b-port transfer \
    --new-client-connection --yes
}

install_prerequisites
add_keys
create_channel

echo "✉️ Start Hermes"
hermes start
