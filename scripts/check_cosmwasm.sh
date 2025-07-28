#!/bin/bash

ENDPOINTS=(
  "https://pisco-lcd.terra.dev"
  "https://juno-testnet-api.polkachu.com"
  "https://rest.elgafar-1.stargaze-apis.com"
  "https://api.andromedaprotocol.io/rest/testnet"
  "https://k8s.testnet.lcd.injective.network"
  "https://api.constantine.archway.io"
  "https://rest-falcron.pion-1.ntrn.tech"
  "https://api-testnet5.composable-cosmos.composablenodes.tech"
  "https://rest.flixnet-4.omniflix.network:443"
  "https://testnet-api.pryzm.zone"
  "https://terp-testnet-api.itrocket.net"
  "https://stride.testnet-1.stridenet.co/api"
  "https://full-node.testnet-1.coreum.dev:1317"
  "https://vota-testnet-rest.dorafactory.org"
  "https://rest.testnet2.persistence.one"
  "https://lcd.testnet-1.nibiru.fi"
  "https://titan-testnet-lcd.titanlab.io:443"
  "https://api.cheqd.network"
  "https://kujira-testnet-api.polkachu.com"
  "https://migaloo-testnet-api.polkachu.com"
  "https://api.sandbox.nymtech.net"
  "https://lcd.testnet-2.nibiru.fi"
  "https://lcd.aura.network"
  "https://lcd.testnet.seda.xyz"
  "https://quasar-testnet-api.polkachu.com"
  "https://lcd.morpheus.desmos.network"
  "https://canon-4.api.network.umee.cc"
  "https://nois-testnet-api.polkachu.com"
  "https://lcd.dhealth.com"
  "https://realio-api.genznodes.dev"
  "https://rest-dorado.fetch.ai"
  "https://api.gopanacea.org"
  "https://node.testnet.like.co"
  "https://evmos.test.api.coldyvalidator.net"
  "https://api-challenge.blockchain.ki"
  "https://dydx-testnet-api.polkachu.com"
)

for URL in "${ENDPOINTS[@]}"; do
  echo -n "$URL => "
  RESPONSE=$(curl -s "$URL/cosmos/base/tendermint/v1beta1/node_info")
  if echo "$RESPONSE" | grep -q "github.com/CosmWasm/wasmvm/v2"; then
    echo "✅ Found CosmWasm 2.2+"
  elif echo "$RESPONSE" | grep -q "github.com/CosmWasm/wasmvm"; then
    echo "⚠️ Found CosmWasm 1.x"
  else
    STATUS_CODE=$(curl -s -o /dev/null -w "%{http_code}" "$URL/cosmos/base/tendermint/v1beta1/node_info")
    echo "❌ Error: HTTP Status $STATUS_CODE"
  fi
done


#  "https://sei-chain-incentivized.com/sei-chain-app"
#  "https://lcd.serenity.aura.network"
#  "https://test3-rest.comdex.one"