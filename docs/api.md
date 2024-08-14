# 买卖
```
curl -X POST http://127.0.0.1:7235/api/swap \
-H "Content-Type: application/json" \
-d '{
  "mint": "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
  "direction": "buy|sell",
  "amount_in": 0.001,
  "in_type": null,
  "slippage": 20,
  "jito": false|true
}'
```

# 按比例卖出
`in_type` 设置为pct
`amount_in` 为百分比，当`amount_in=1`时将全部卖出，并关闭ATA
```
curl -X POST http://127.0.0.1:7235/api/swap \
-H "Content-Type: application/json" \
-d '{
  "mint": "EKpQGSJtjMFqKZ9KQanSqYXRcF8fBopzLHYxdM65zcjm",
  "direction": "sell",
  "amount_in": 1,
  "in_type": pct,
  "slippage": 20,
  "jito": false|true
}'
```
