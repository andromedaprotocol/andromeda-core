#!/bin/sh

STAKE_ANDR=${STAKE_TOKEN:-stake}
COIN_ANDR=${COIN_TOKEN:-uandr}

ANDROMEDA_HOME=$HOME/.andromedad
CONFIG_FOLDER=$ANDROMEDA_HOME/config

install_prerequisites () {
    apk add dasel
}


edit_genesis () {

    GENESIS=$CONFIG_FOLDER/genesis.json
    # staking/governance token is hardcoded in config, change this
    sed -i "s/\"uandr\"/\"$COIN_ANDR\"/" $GENESIS

    # Update staking module
    dasel put string -f $GENESIS '.app_state.staking.params.bond_denom' $STAKE_ANDR
    dasel put string -f $GENESIS '.app_state.staking.params.unbonding_time' '240s'

    # Update crisis module
    dasel put string -f $GENESIS '.app_state.crisis.constant_fee.denom' $COIN_ANDR

    # Udpate gov module
    dasel put string -f $GENESIS '.app_state.gov.voting_params.voting_period' '60s'
    dasel put string -f $GENESIS '.app_state.gov.deposit_params.min_deposit.[0].denom' $COIN_ANDR

    # Update poolincentives module
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[0]' "120s"
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[1]' "180s"
    dasel put string -f $GENESIS '.app_state.poolincentives.lockable_durations.[2]' "240s"
    dasel put string -f $GENESIS '.app_state.poolincentives.params.minted_denom' $COIN_ANDR

    # Update incentives module
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[0]' "1s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[1]' "120s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[2]' "180s"
    dasel put string -f $GENESIS '.app_state.incentives.lockable_durations.[3]' "240s"

    # Update mint module
    dasel put string -f $GENESIS '.app_state.mint.params.mint_denom' $COIN_ANDR

    # Update gamm module
    dasel put string -f $GENESIS '.app_state.gamm.params.pool_creation_fee.[0].denom' $COIN_ANDR

    # Update txfee basedenom
    dasel put string -f $GENESIS '.app_state.txfees.basedenom' $COIN_ANDR

    # Update wasm permission (Nobody or Everybody)
    dasel put string -f $GENESIS '.app_state.wasm.params.code_upload_access.permission' "Everybody"
}

add_account () {
    local MNEMONIC=$1
    local MONIKER=$2

    echo $MNEMONIC | andromedad keys add $MONIKER --recover --keyring-backend=test --home $ANDROMEDA_HOME
    local ACCOUNT=$(andromedad keys show -a $MONIKER --keyring-backend test --home $ANDROMEDA_HOME)
    andromedad add-genesis-account $ACCOUNT "1000000000000$COIN_ANDR,1000000000000$STAKE_ANDR" --home $ANDROMEDA_HOME
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
    for addr in andr12xxey4enkcfgv522cxl03xmk7tdpmy6k5m5zhr; do
        echo $addr
        andromedad add-genesis-account "$addr" "10000000000$COIN_ANDR,10000000000$STAKE_ANDR"
    done

    andromedad gentx $VALIDATOR_MONIKER "250000000$STAKE_ANDR" --amount="250000000$STAKE_ANDR" --keyring-backend=test --chain-id="$CHAIN_ID" --home $ANDROMEDA_HOME
    andromedad collect-gentxs --home $ANDROMEDA_HOME
}

edit_config () {
    # Remove seeds
    dasel put string -f $CONFIG_FOLDER/config.toml '.p2p.seeds' ''

    # Expose the rpc
    dasel put string -f $CONFIG_FOLDER/config.toml '.rpc.laddr' "tcp://0.0.0.0:26657"
    dasel put string -f $CONFIG_FOLDER/config.toml '.consensus.timeout_commit' "1500ms"
    dasel put string -f $CONFIG_FOLDER/config.toml '.rpc.cors_allowed_origins.[0]' "*"
    
    dasel put string -f $CONFIG_FOLDER/config.toml '.consensus.timeout_commit' "1500ms"
}

edit_app () {
    local APP=$CONFIG_FOLDER/app.toml

    # Enable lcd
    dasel put bool -f $APP '.api.enable' true
    dasel put bool -f $APP '.api.enabled-unsafe-cors' true
    # Gas Price
    dasel put string -f $APP 'minimum-gas-prices' "0.25$COIN_ANDR"
}

if [[ ! -d $CONFIG_FOLDER ]]
then
    install_prerequisites
    echo "🧪 Creating Andromeda home for $VALIDATOR_MONIKER"
    andromedad init --chain-id=$CHAIN_ID --home $ANDROMEDA_HOME $VALIDATOR_MONIKER
    edit_genesis
    add_genesis_accounts
    edit_config
    edit_app
fi

echo "🏁 Starting $CHAIN_ID..."

mkdir -p /root/log
andromedad start --trace