OUT_DIR="artifacts"

for contract in $OUT_DIR/*.wasm; do
  echo "Processing $contract"
  contract_name=`basename $contract`
  contract_name=${contract_name//.wasm/}
  if [[ "$contract_name" == *"@"* ]]; then
    echo "Skipping $contract_name as it already has a version."
    continue
  fi
  formated_name=${contract_name//_/-}
  version=$(cargo pkgid $formated_name | cut -d# -f2 | cut -d: -f2)
  mv "$contract" "$OUT_DIR/$contract_name@$version.wasm"
done