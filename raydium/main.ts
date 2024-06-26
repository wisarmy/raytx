import { Connection, PublicKey } from "@solana/web3.js";
import { LIQUIDITY_STATE_LAYOUT_V4 } from "@raydium-io/raydium-sdk";
import { OpenOrders } from "@project-serum/serum";
import express from "express";
import { swap } from "./swap";
import {
  COMMITMENT_LEVEL,
  RPC_ENDPOINT,
  RPC_WEBSOCKET_ENDPOINT,
} from "./helpers";
import bodyParser from "body-parser";

// raydium pool id can get from api: https://api.raydium.io/v2/sdk/liquidity/mainnet.json
const OPENBOOK_PROGRAM_ID = new PublicKey(
  "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
);

export async function parsePoolInfo(pool_id: string) {
  const connection = new Connection(RPC_ENDPOINT, {
    wsEndpoint: RPC_WEBSOCKET_ENDPOINT,
    commitment: COMMITMENT_LEVEL,
  });
  // example to get pool info
  const info = await connection.getAccountInfo(new PublicKey(pool_id));

  if (!info) return;
  const poolState = LIQUIDITY_STATE_LAYOUT_V4.decode(info.data);

  const openOrders = await OpenOrders.load(
    connection,
    poolState.openOrders,
    OPENBOOK_PROGRAM_ID, // OPENBOOK_PROGRAM_ID(marketProgramId) of each pool can get from api: https://api.raydium.io/v2/sdk/liquidity/mainnet.json
  );

  const baseDecimal = 10 ** poolState.baseDecimal.toNumber(); // e.g. 10 ^ 6
  const quoteDecimal = 10 ** poolState.quoteDecimal.toNumber();

  const baseTokenAmount = await connection.getTokenAccountBalance(
    poolState.baseVault,
  );
  const quoteTokenAmount = await connection.getTokenAccountBalance(
    poolState.quoteVault,
  );

  const basePnl = poolState.baseNeedTakePnl.toNumber() / baseDecimal;
  const quotePnl = poolState.quoteNeedTakePnl.toNumber() / quoteDecimal;
  const openOrdersBaseTokenTotal =
    openOrders.baseTokenTotal.toNumber() / baseDecimal;
  const openOrdersQuoteTokenTotal =
    openOrders.quoteTokenTotal.toNumber() / quoteDecimal;

  const base =
    (baseTokenAmount.value?.uiAmount || 0) + openOrdersBaseTokenTotal - basePnl;
  const quote =
    (quoteTokenAmount.value?.uiAmount || 0) +
    openOrdersQuoteTokenTotal -
    quotePnl;
  const result = {
    base: base,
    quote: quote,
  };
  return result;
}

const app = express();
app.use(bodyParser.json());

app.get("/pools/:pool_id", async (req, res) => {
  console.log("pool_id: " + req.params.pool_id);
  try {
    const info = await parsePoolInfo(req.params.pool_id);
    res.send(info);
  } catch (error) {
    console.error("Error fetching pool info:", error);
    res.status(500).json({ error: "Error fetching pool info" });
  }
});

app.post("/swap", async (req, res) => {
  const { poolId, amountIn, dir } = req.body;
  console.log(
    `swap request - poolId: ${poolId}, amountIn: ${amountIn} ,dir: ${dir}`,
  );

  try {
    const result = await swap(poolId, amountIn, dir);
    if (result) {
      res.json({ success: true, message: "Swap operation was successful." });
    } else {
      res
        .status(500)
        .json({ success: false, message: "Swap operation failed." });
    }
  } catch (error) {
    console.error("Error during swap operation:", error);
    res
      .status(500)
      .json({ success: false, message: "Error during swap operation" });
  }
});

const port = process.env.PORT ? parseInt(process.env.PORT) : 3000;
app.listen(port, () => {
  console.log(`The application is listening on port ${port}!`);
  console.log("GET /pools/:pool_id  get pool information");
  console.log("POST /swap   swap to sol");
});
