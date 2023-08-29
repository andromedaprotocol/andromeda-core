#!/bin/sh
set -eo pipefail

OSMOSIS_HOME=$HOME/.osmosisd
CONFIG_FOLDER=$OSMOSIS_HOME/config

install_prerequisites () {
    apk add dasel
}

edit_genesis () {

    GENESIS=$CONFIG_FOLDER/genesis.json

    # Update staking module
    dasel put string -f $GENESIS '.app_state.staking.params.bond_denom' 'uosmo'
    dasel put string -f $GENESIS '.app_state.staking.params.unbonding_time' '240s'

    # Update crisis module
    dasel put string -f $GENESIS '.app_state.crisis.constant_fee.denom' 'uosmo'

    # Udpate gov module
    dasel put string -f $GENESIS '.app_state.gov.voting_params.voting_period' '60s'
    dasel put string -f $GENESIS '.app_state.gov.deposit_params.min_deposit.[0].denom' 'uosmo'

    # Update epochs module
    dasel put string -f $GENESIS '.app_state.epochs.epochs.[1].duration' "60s"

    # Update poolincentives module
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[0]' "120s"
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[1]' "180s"
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[2]' "240s"
    dasel put string -f $GENESIS '.app_state.poolincentives.params.minted_denom' "uosmo"

    # Update incentives module
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[0]' "1s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[1]' "120s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[2]' "180s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[3]' "240s"
    dasel put string -f $GENESIS '.app_state.incentives.params.distr_epoch_identifier' "day"

    # Update mint module
    dasel put string -f $GENESIS '.app_state.mint.params.mint_denom' "uosmo"
    dasel put string -f $GENESIS '.app_state.mint.params.epoch_identifier' "day"

    # Update gamm module
    dasel put string -f $GENESIS '.app_state.gamm.params.pool_creation_fee.[0].denom' "uosmo"

    # Update txfee basedenom
    dasel put string -f $GENESIS '.app_state.txfees.basedenom' "uosmo"

    # Update wasm permission (Nobody or Everybody)
    dasel put string -f $GENESIS '.app_state.wasm.params.code_upload_access.permission' "Everybody"
}

add_account () {
    local MNEMONIC=$1
    local MONIKER=$2

    echo $MNEMONIC | osmosisd keys add $MONIKER --recover --keyring-backend=test --home $OSMOSIS_HOME
    ACCOUNT=$(osmosisd keys show -a $MONIKER --keyring-backend test --home $OSMOSIS_HOME)
    osmosisd add-genesis-account $ACCOUNT 100000000000uosmo,100000000000uion,100000000000stake --home $OSMOSIS_HOME
}

add_genesis_accounts () {
    # Validator
    echo "⚖️ Add validator account"
    add_account "$VALIDATOR_MNEMONIC" "$VALIDATOR_MONIKER"
    
    # Faucet
    echo "🚰 Add faucet account"
    add_account "$FAUCET_MNEMONIC" "faucet"

    # Relayer
    echo "🔗 Add relayer account"
    add_account "$RELAYER_MNEMONIC" "relayer"

    # Add test user accounts
    add_account "$TEST_USER_1_MNEMONIC" "test-user-1"
    add_account "$TEST_USER_2_MNEMONIC" "test-user-2"
    add_account "$TEST_USER_3_MNEMONIC" "test-user-3"

    # (optionally) add a few more genesis accounts
    for addr in osmo12xxey4enkcfgv522cxl03xmk7tdpmy6kyt0sau; do
        echo $addr
        osmosisd add-genesis-account "$addr" 1000000000uosmo,1000000000uion,1000000000stake --home $OSMOSIS_HOME
    done
    
    osmosisd gentx $VALIDATOR_MONIKER 500000000uosmo --keyring-backend=test --chain-id=$CHAIN_ID --home $OSMOSIS_HOME
    osmosisd collect-gentxs --home $OSMOSIS_HOME
}

edit_config () {
    # Remove seeds
    dasel put string -f $CONFIG_FOLDER/config.toml '.p2p.seeds' ''

    # Expose the rpc
    dasel put string -f $CONFIG_FOLDER/config.toml '.rpc.laddr' "tcp://0.0.0.0:26657"
    dasel put string -f $CONFIG_FOLDER/config.toml '.consensus.timeout_commit' "1500ms"
    dasel put string -f $CONFIG_FOLDER/config.toml '.rpc.cors_allowed_origins.[0]' "*"

}

edit_app () {
    local APP=$CONFIG_FOLDER/app.toml

    # Enable lcd
    dasel put bool -f $APP '.api.enable' true
    dasel put bool -f $APP '.api.enabled-unsafe-cors' true

    # Gas Price
    dasel put string -f $APP 'minimum-gas-prices' "0.25uosmo"
}

if [[ ! -d $CONFIG_FOLDER ]]
then
    install_prerequisites
    echo "🧪 Creating Osmosis home for $VALIDATOR_MONIKER"
    echo $VALIDATOR_MNEMONIC | osmosisd init -o --chain-id=$CHAIN_ID --home $OSMOSIS_HOME --recover $VALIDATOR_MONIKER
    edit_genesis
    add_genesis_accounts
    edit_config
    edit_app
fi

echo "🏁 Starting $CHAIN_ID..."
osmosisd start --home $OSMOSIS_HOME