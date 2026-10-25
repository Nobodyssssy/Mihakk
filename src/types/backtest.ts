export type Candle = {
  date: string;
  open: number;
  high: number;
  low: number;
  close: number;
  volume: number;
};

export type Trade = {
  entry_date: string;
  exit_date: string;
  shares: number;
  entry_price: number;
  exit_price: number;
  net_pnl: number;
};

export type BacktestResult = {
  ending_cash: number;
  ending_equity: number;
  open_shares: number;
  trades: Trade[];
};

export type DemoRun = {
  candles: Candle[];
  result: BacktestResult;
};