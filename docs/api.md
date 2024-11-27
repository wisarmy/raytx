# Buy/Sell
```
curl -X POST http://127.0.0.1:7235/api/swap \
-H "Content-Type: application/json" \
-d '{
  "mint": "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
  "direction": "buy|sell",
  "amount_in": 0.001,
  "slippage": 20,
  "jito": false|true
}'
```

# Sell Proportionally
Set `in_type` to `pct`
`amount_in` is the percentage; when `amount_in=1`, it will sell all and close ATA
```
curl -X POST http://127.0.0.1:7235/api/swap \
-H "Content-Type: application/json" \
-d '{
  "mint": "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
  "direction": "sell",
  "amount_in": 1,
  "in_type": "pct",
  "slippage": 20,
  "jito": false|true
}'
```
# Get pool price
```
curl http://127.0.0.1:7235/api/pool/{pool_id}
```
Response:
```json
{
  "data": {
    "base": 152897118.502952,
    "price": 0.000103805,
    "quote": 110.340824464,
    "sol_price": 143.84
  },
  "status": "ok"
}
```
# Get coin
```
http://127.0.0.1:7235/api/coins/{mint}
```
```json
{
"data": {
"associated_bonding_curve": "E82g93v8gHWULYFfhmFushJZFEG4fP7PiBgNQefioCqj",
"bonding_curve": "4PobGYLLEs8niNg1bWNreNZgu8pDPwYH5ytgmCoxKpfC",
"complete": true,
"created_timestamp": 1732591590787,
"creator": "FzfTq6vGy8vvns5J6xbnh3WeTRWHm6MwATWrYBKyRyar",
"description": "",
"image_uri": "https://ipfs.io/ipfs/Qmayxq68yjipGKUWMPriCXVCENFqhd8P3tyszAyAnnLuVr",
"inverted": true,
"is_currently_live": false,
"king_of_the_hill_timestamp": 1732591699000,
"last_reply": 1732593476763,
"market_cap": 47.72,
"market_id": "7H6Ybc7LYTzTE6MK7Ai7h9utfqArvAoMpDHBH1CueGaK",
"metadata_uri": "https://ipfs.io/ipfs/QmP72w77xYPzoGNvYvietLVKKpjYX12uFFnpLmhdwaztfC",
"mint": "EQitNE2QozWdyaz11eq2nVtrLqLUgwKLyXxhBwtZpump",
"name": "Justice for Stephen Singleton",
"nsfw": false,
"profile_image": null,
"raydium_info": {
"base": 604542889.853835,
"price": 4.78322920049418e-8,
"quote": 28.916672037
},
"raydium_pool": "9XBq7pkEmhP7E7qEqEoko3hvadrNjiLJRfXS3NJdyLK8",
"reply_count": 301,
"show_name": true,
"symbol": "Stephen",
"telegram": null,
"total_supply": 1000000000000000,
"twitter": "https://x.com/marionawfal/status/1861249022159122444?s=46&t=f-10UPDsIV3KvlJrv0_W6A",
"usd_market_cap": 11361.1776,
"username": "meowster1",
"virtual_sol_reserves": 115005359175,
"virtual_token_reserves": 279900000000000,
"website": null
},
"status": "ok"
}
```


# Get token accounts
```
curl http://127.0.0.1:7235/api/token-accounts
```
Response:
```json
{
  "data": [
    {
      "amount": "0",
      "mint": "Fof1DyVSYiQGCnT3uTbmq8kQMPdwL35x1bD82NaTs9mM",
      "pubkey": "H3rveEcUaRwNEyaHgmo5F8Jnz1pqP7c1U8ePPHhyjdqV",
      "ui_amount": 0
    },
    {
      "amount": "0",
      "mint": "7ijK2wWEPSUHgMRpVawWQiAiMuNnEuvV5GbEyBrTpump",
      "pubkey": "F8qyryJjXESXcoEnw5TnVWpEpkQpvGz47oq41Mn8fuLE",
      "ui_amount": 0
    }
  ],
  "status": "ok"
}
```
# Get token account
```
curl http://127.0.0.1:7235/api/token-accounts/Fof1DyVSYiQGCnT3uTbmq8kQMPdwL35x1bD82NaTs9mM
```
Response:
```json
{
  "data": {
    "amount": "0",
    "mint": "Fof1DyVSYiQGCnT3uTbmq8kQMPdwL35x1bD82NaTs9mM",
    "pubkey": "H3rveEcUaRwNEyaHgmo5F8Jnz1pqP7c1U8ePPHhyjdqV",
    "ui_amount": 0
  },
  "status": "ok"
}
```
