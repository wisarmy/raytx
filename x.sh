curl https://tokyo.mainnet.block-engine.jito.wtf/api/v1/bundles -X POST -H "Content-Type: application/json" -d '
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "getInflightBundleStatuses",
  "params": [
    [
        "927e7ab1f598422e2e5da726495d1314e5504f26b9a862e081a7cbfd5858c5f7"
    ]
  ]
}
'

curl https://tokyo.mainnet.block-engine.jito.wtf/api/v1/bundles -X POST -H "Content-Type: application/json" -d '
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "getBundleStatuses",
    "params": [
      [
        "927e7ab1f598422e2e5da726495d1314e5504f26b9a862e081a7cbfd5858c5f7"
      ]
    ]
}
'
