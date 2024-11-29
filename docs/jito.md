# Introduction
[jito docs](https://docs.jito.wtf/)

# Get tip accounts
```
curl https://mainnet.block-engine.jito.wtf/api/v1/bundles -X POST -H "Content-Type: application/json" -d '
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getTipAccounts",
    "params": []
}
'
# response
{
  "jsonrpc": "2.0",
  "result": [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY"
  ],
  "id": 1
}
```
# Mainnet
## Tip Payment Program:
T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt
## Tip Distribution Program:
4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7
## WebSocket showing tip amounts:
ws://bundles-api-rest.jito.wtf/api/v1/bundles/tip_stream
## Tip dashoard
https://jito-labs.metabaseapp.com/public/dashboard/016d4d60-e168-4a8f-93c7-4cd5ec6c7c8d

# Mainnet Addresses
## Amsterdam
BLOCK_ENGINE_URL=https://amsterdam.mainnet.block-engine.jito.wtf
SHRED_RECEIVER_ADDR=74.118.140.240:1002
RELAYER_URL=http://amsterdam.mainnet.relayer.jito.wtf:8100
## Frankfurt
BLOCK_ENGINE_URL=https://frankfurt.mainnet.block-engine.jito.wtf
SHRED_RECEIVER_ADDR=145.40.93.84:1002
RELAYER_URL=http://frankfurt.mainnet.relayer.jito.wtf:8100
## New York
BLOCK_ENGINE_URL=https://ny.mainnet.block-engine.jito.wtf
SHRED_RECEIVER_ADDR=141.98.216.96:1002
RELAYER_URL=http://ny.mainnet.relayer.jito.wtf:8100
## Tokyo
BLOCK_ENGINE_URL=https://tokyo.mainnet.block-engine.jito.wtf
SHRED_RECEIVER_ADDR=202.8.9.160:1002
RELAYER_URL=http://tokyo.mainnet.relayer.jito.wtf:8100
## Salt Lake City
BLOCK_ENGINE_URL=https://slc.mainnet.block-engine.jito.wtf
SHRED_RECEIVER_ADDR=64.130.53.8:1002
RELAYER_URL=http://slc.mainnet.relayer.jito.wtf:8100
