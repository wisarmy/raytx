import {
  ComputeBudgetProgram,
  Connection,
  Keypair,
  PublicKey,
  TransactionMessage,
  VersionedTransaction,
} from "@solana/web3.js";
import {
  createAssociatedTokenAccountIdempotentInstruction,
  createCloseAccountInstruction,
  getAssociatedTokenAddress,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  LIQUIDITY_STATE_LAYOUT_V4,
  Liquidity,
  LiquidityPoolInfo,
  LiquidityPoolKeys,
  LiquidityPoolKeysV4,
  LiquidityStateV4,
  Percent,
  Token,
  TokenAmount,
  WSOL,
} from "@raydium-io/raydium-sdk";
import {
  DefaultTransactionExecutor,
  TransactionExecutor,
} from "./transactions";
import {
  createPoolKeys,
  logger,
  MinimalMarketLayoutV3,
  getMinimalMarketV3,
  LOG_LEVEL,
  TRANSACTION_EXECUTOR,
  CUSTOM_FEE,
  RPC_ENDPOINT,
  RPC_WEBSOCKET_ENDPOINT,
  COMMITMENT_LEVEL,
  PRIVATE_KEY,
  QUOTE_MINT,
  getWallet,
  getToken,
  MAX_BUY_RETRIES,
  COMPUTE_UNIT_LIMIT,
  COMPUTE_UNIT_PRICE,
  BUY_SLIPPAGE,
  SELL_SLIPPAGE,
  MAX_SELL_RETRIES,
} from "./helpers";
import { WarpTransactionExecutor } from "./transactions/warp-transaction-executor";
import { JitoTransactionExecutor } from "./transactions/jito-rpc-transaction-executor";

export interface BotConfig {
  wallet: Keypair;
  quoteToken: Token;
  quoteAmount: TokenAmount;
  quoteAta: PublicKey;
  buySlippage: number;
  maxBuyRetries: number;
  unitLimit: number;
  unitPrice: number;
  sellSlippage: number;
  maxSellRetries: number;
  isPump: boolean;
}

export class Bot {
  public readonly isWarp: boolean = false;
  public readonly isJito: boolean = false;

  constructor(
    private readonly connection: Connection,
    private readonly txExecutor: TransactionExecutor,
    readonly config: BotConfig,
  ) {
    this.isWarp = txExecutor instanceof WarpTransactionExecutor;
    this.isJito = txExecutor instanceof JitoTransactionExecutor;
  }

  public async buy(
    accountId: PublicKey,
    poolState: LiquidityStateV4,
  ): Promise<boolean> {
    logger.trace({ mint: poolState.baseMint }, `Processing new pool...`);

    try {
      const [market, mintAta] = await Promise.all([
        this.fetch(poolState.marketId.toString()),
        getAssociatedTokenAddress(
          poolState.baseMint,
          this.config.wallet.publicKey,
        ),
      ]);

      const poolKeys: LiquidityPoolKeysV4 = createPoolKeys(
        accountId,
        poolState,
        market,
      );
      // console.log({
      //   baseMint: poolState.baseMint,
      //   quoteMint: poolState.quoteMint,
      //   poolKeysBaseMint: poolKeys.baseMint,
      //   poolKeysQuoteMint: poolKeys.quoteMint,
      //   mintAta: mintAta,
      // });

      for (let i = 0; i < this.config.maxBuyRetries; i++) {
        try {
          logger.info(
            { mint: poolState.baseMint.toString() },
            `Send buy transaction attempt: ${i + 1}/${this.config.maxBuyRetries}`,
          );
          const tokenOut = new Token(
            TOKEN_PROGRAM_ID,
            poolKeys.baseMint,
            poolKeys.baseDecimals,
          );
          const result = await this.swap(
            poolKeys,
            this.config.quoteAta,
            mintAta,
            this.config.quoteToken,
            tokenOut,
            this.config.quoteAmount,
            this.config.buySlippage,
            this.config.wallet,
            "buy",
          );

          if (result.confirmed) {
            logger.info(
              {
                mint: poolState.baseMint.toString(),
                signature: result.signature,
                url: `https://solscan.io/tx/${result.signature}`,
              },
              `Confirmed buy tx`,
            );

            return true;
          }

          logger.info(
            {
              mint: poolState.baseMint.toString(),
              signature: result.signature,
              error: result.error,
            },
            `Error confirming buy tx`,
          );
        } catch (error) {
          logger.debug(
            { mint: poolState.baseMint.toString(), error },
            `Error confirming buy transaction`,
          );
          return false;
        }
      }
    } catch (error) {
      logger.error(
        { mint: poolState.baseMint.toString(), error },
        `Failed to buy token`,
      );
      return false;
    }
    return false;
  }

  public async sell(
    accountId: PublicKey,
    poolState: LiquidityStateV4,
    amount: string | number,
    close = false,
  ): Promise<boolean> {
    try {
      logger.trace({ mint: poolState.baseMint }, `Processing new token...`);

      const [market, mintAta] = await Promise.all([
        this.fetch(poolState.marketId.toString()),
        getAssociatedTokenAddress(
          poolState.baseMint,
          this.config.wallet.publicKey,
        ),
      ]);
      // console.log(
      //   "baseMint:" + poolState.baseMint,
      //   "baseDecimal:" + poolState.baseDecimal.toNumber(),
      //   "quoteMint:" + poolState.quoteMint,
      //   "quoteDecimal:" + poolState.quoteDecimal.toNumber(),
      //   "mintAta:" + mintAta,
      // );
      const tokenIn = new Token(
        TOKEN_PROGRAM_ID,
        poolState.baseMint,
        poolState.baseDecimal.toNumber(),
      );

      const tokenAmountIn = new TokenAmount(tokenIn, amount, false);

      const poolKeys: LiquidityPoolKeysV4 = createPoolKeys(
        accountId,
        poolState,
        market,
      );

      for (let i = 0; i < this.config.maxSellRetries; i++) {
        try {
          logger.info(
            { mint: poolState.baseMint },
            `Send sell transaction attempt: ${i + 1}/${this.config.maxSellRetries}`,
          );

          const dir = close ? "sell_and_close" : "sell";

          const result = await this.swap(
            poolKeys,
            mintAta,
            this.config.quoteAta,
            tokenIn,
            this.config.quoteToken,
            tokenAmountIn,
            this.config.sellSlippage,
            this.config.wallet,
            dir,
          );

          if (result.confirmed) {
            logger.info(
              {
                dex: `https://dexscreener.com/solana/${poolState.baseMint.toString()}?maker=${this.config.wallet.publicKey}`,
                mint: poolState.baseMint.toString(),
                signature: result.signature,
                url: `https://solscan.io/tx/${result.signature}`,
              },
              `Confirmed sell tx`,
            );
            return true;
          }

          logger.info(
            {
              mint: poolState.baseMint.toString(),
              signature: result.signature,
              error: result.error,
            },
            `Error confirming sell tx`,
          );
        } catch (error) {
          logger.debug(
            { mint: poolState.baseMint.toString(), error },
            `Error confirming sell transaction`,
          );
          return false;
        }
      }
    } catch (error) {
      logger.error(
        { mint: poolState.baseMint.toString(), error },
        `Failed to sell token`,
      );
      return false;
    }
    return false;
  }

  // noinspection JSUnusedLocalSymbols
  private async swap(
    poolKeys: LiquidityPoolKeysV4,
    ataIn: PublicKey,
    ataOut: PublicKey,
    tokenIn: Token,
    tokenOut: Token,
    amountIn: TokenAmount,
    slippage: number,
    wallet: Keypair,
    direction: "buy" | "sell" | "sell_and_close",
  ) {
    const slippagePercent = new Percent(slippage, 100);
    const poolInfo = await fetchPoolInfo(
      this.connection,
      poolKeys,
      this.config.isPump,
    );
    const computedAmountOut = Liquidity.computeAmountOut({
      poolKeys,
      poolInfo,
      amountIn,
      currencyOut: tokenOut,
      slippage: slippagePercent,
    });

    const latestBlockhash = await this.connection.getLatestBlockhash();
    const { innerTransaction } = Liquidity.makeSwapFixedInInstruction(
      {
        poolKeys: poolKeys,
        userKeys: {
          tokenAccountIn: ataIn,
          tokenAccountOut: ataOut,
          owner: wallet.publicKey,
        },
        amountIn: amountIn.raw,
        minAmountOut: computedAmountOut.minAmountOut.raw,
      },
      poolKeys.version,
    );

    const messageV0 = new TransactionMessage({
      payerKey: wallet.publicKey,
      recentBlockhash: latestBlockhash.blockhash,
      instructions: [
        ...(this.isWarp || this.isJito
          ? []
          : [
              ComputeBudgetProgram.setComputeUnitPrice({
                microLamports: this.config.unitPrice,
              }),
              ComputeBudgetProgram.setComputeUnitLimit({
                units: this.config.unitLimit,
              }),
            ]),
        ...(direction === "buy"
          ? [
              createAssociatedTokenAccountIdempotentInstruction(
                wallet.publicKey,
                ataOut,
                wallet.publicKey,
                tokenOut.mint,
              ),
            ]
          : []),
        ...innerTransaction.instructions,
        ...(direction === "sell"
          ? [
              // createCloseAccountInstruction(
              //   ataIn,
              //   wallet.publicKey,
              //   wallet.publicKey,
              // ),
            ]
          : []),
        ...(direction === "sell_and_close"
          ? [
              createCloseAccountInstruction(
                ataIn,
                wallet.publicKey,
                wallet.publicKey,
              ),
            ]
          : []),
      ],
    }).compileToV0Message();

    const transaction = new VersionedTransaction(messageV0);
    transaction.sign([wallet, ...innerTransaction.signers]);

    return this.txExecutor.executeAndConfirm(
      transaction,
      wallet,
      latestBlockhash,
    );
  }

  private fetch(marketId: string): Promise<MinimalMarketLayoutV3> {
    return getMinimalMarketV3(
      this.connection,
      new PublicKey(marketId),
      this.connection.commitment,
    );
  }
}
// 获取pool state
export const getPoolState = async (
  connection: Connection,
  pool_id: PublicKey,
): Promise<LiquidityStateV4> => {
  try {
    // example to get pool info
    const info = await connection.getAccountInfo(pool_id);

    if (!info) {
      throw new Error("Account not found");
    }
    const poolState = LIQUIDITY_STATE_LAYOUT_V4.decode(
      info.data,
    ) as LiquidityStateV4;

    return poolState;
  } catch (error) {
    console.error("Error during token swap:", error);
    throw new Error("Failed to decode pool state");
  }
};
export const fetchPoolInfo = async (
  connection: Connection,
  poolKeys: LiquidityPoolKeys,
  isPump: boolean,
): Promise<LiquidityPoolInfo> => {
  const poolInfo = await Liquidity.fetchInfo({
    connection: connection,
    poolKeys,
  });

  // 坑...pump池子信息 base 与 quote 与普通池子颠倒
  if (isPump) {
    const baseDecimals = poolInfo.baseDecimals;
    const baseReserve = poolInfo.baseReserve;
    poolInfo.baseDecimals = poolInfo.quoteDecimals;
    poolInfo.baseReserve = poolInfo.quoteReserve;
    poolInfo.quoteDecimals = baseDecimals;
    poolInfo.quoteReserve = baseReserve;
  }
  return poolInfo;
};

export const swap = async (
  pool_id: string,
  amount: string | number,
  dir: number,
): Promise<boolean> => {
  try {
    logger.level = LOG_LEVEL;
    const connection = new Connection(RPC_ENDPOINT, {
      wsEndpoint: RPC_WEBSOCKET_ENDPOINT,
      commitment: COMMITMENT_LEVEL,
    });

    let txExecutor: TransactionExecutor;

    switch (TRANSACTION_EXECUTOR) {
      case "warp": {
        txExecutor = new WarpTransactionExecutor(CUSTOM_FEE);
        break;
      }
      case "jito": {
        txExecutor = new JitoTransactionExecutor(CUSTOM_FEE, connection);
        break;
      }
      default: {
        txExecutor = new DefaultTransactionExecutor(connection);
        break;
      }
    }

    const wallet = getWallet(PRIVATE_KEY.trim());
    const quoteToken = getToken(QUOTE_MINT);
    const botConfig = <BotConfig>{
      wallet,
      quoteAta: getAssociatedTokenAddressSync(
        quoteToken.mint,
        wallet.publicKey,
      ),
      quoteToken,
      maxBuyRetries: MAX_BUY_RETRIES,
      unitLimit: COMPUTE_UNIT_LIMIT,
      unitPrice: COMPUTE_UNIT_PRICE,
      buySlippage: BUY_SLIPPAGE,
      sellSlippage: SELL_SLIPPAGE,
      maxSellRetries: MAX_SELL_RETRIES,
      isPump: false,
    };

    const account_id = new PublicKey(pool_id);
    const poolState = await getPoolState(connection, account_id);
    // 坑...pump池子信息 base 与 quote 与普通池子颠倒
    if (poolState.baseMint.toString() == WSOL.mint) {
      const baseMint = poolState.baseMint;
      const baseDecimal = poolState.baseDecimal;
      poolState.baseMint = poolState.quoteMint;
      poolState.baseDecimal = poolState.quoteDecimal;
      poolState.quoteMint = baseMint;
      poolState.quoteDecimal = baseDecimal;
      // set pump
      botConfig.isPump = true;
    }

    const bot = new Bot(connection, txExecutor, botConfig);
    if (dir === 0) {
      // 如果是买，设置quoteAmount
      botConfig.quoteAmount = new TokenAmount(quoteToken, amount, false);
      return await bot.buy(account_id, poolState);
    } else if (dir === 1) {
      // 卖出
      return await bot.sell(account_id, poolState, amount);
    } else if (dir === 11) {
      // 卖出并关闭帐户
      return await bot.sell(account_id, poolState, amount, true);
    } else {
      logger.warn({ dir: dir }, `Dir is not supported`);
      return false;
    }
  } catch (error) {
    logger.error({ pool_id: pool_id }, `Swap error: ${error}`);
    return false;
  }
};
