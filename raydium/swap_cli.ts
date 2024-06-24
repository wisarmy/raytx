import { swap } from "./swap";
import { bootstrap } from "global-agent";
bootstrap();

const args = process.argv.slice(2);
console.log(args);
if (args.length !== 3) {
  console.error("Usage: ts-node swap_test.ts <pool_id> <amount> <dir>");
  process.exit(1);
}

const pool_id = args[0];
const amount = parseFloat(args[1]);
const dir = parseInt(args[2]);
swap(pool_id, amount, dir);
