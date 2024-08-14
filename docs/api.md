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
