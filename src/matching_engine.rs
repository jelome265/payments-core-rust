use std::collections::BTreeMap;
use std::cmp::Ordering;

/// Deterministic orderbook for a single currency pair.
/// Runs as a single-actor event loop to prevent race conditions
/// and guarantee deterministic matching. One actor per market pair.

#[derive(Debug, Clone)]
pub struct Order {
    pub id: String,
    pub user_id: String,
    pub side: Side,
    pub price: f64,      // Exchange rate
    pub quantity: f64,    // Amount in sell currency
    pub remaining: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone)]
pub struct Trade {
    pub buy_order_id: String,
    pub sell_order_id: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp: u64,
}

/// Price-time priority orderbook.
/// Bids sorted descending by price (highest first), asks sorted ascending (lowest first).
pub struct Orderbook {
    pub pair: String,
    bids: BTreeMap<OrderKey, Order>,  // Buy orders
    asks: BTreeMap<OrderKey, Order>,  // Sell orders
    sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OrderKey {
    price_cents: i64,  // Price in integer cents for deterministic comparison
    timestamp: u64,
    id: String,
}

impl PartialOrd for OrderKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.price_cents.cmp(&other.price_cents)
            .then(self.timestamp.cmp(&other.timestamp))
            .then(self.id.cmp(&other.id))
    }
}

impl Orderbook {
    pub fn new(pair: &str) -> Self {
        Orderbook {
            pair: pair.to_string(),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            sequence: 0,
        }
    }

    /// Submit an order and attempt to match. Returns any trades produced.
    pub fn submit_order(&mut self, mut order: Order) -> Vec<Trade> {
        self.sequence += 1;
        order.timestamp = self.sequence;
        let mut trades = Vec::new();

        match order.side {
            Side::Buy => {
                // Match against asks (lowest price first)
                let mut matched_keys = Vec::new();
                for (key, ask) in self.asks.iter_mut() {
                    if order.remaining <= 0.0 {
                        break;
                    }
                    if order.price >= ask.price {
                        let fill_qty = order.remaining.min(ask.remaining);
                        trades.push(Trade {
                            buy_order_id: order.id.clone(),
                            sell_order_id: ask.id.clone(),
                            price: ask.price,
                            quantity: fill_qty,
                            timestamp: self.sequence,
                        });
                        order.remaining -= fill_qty;
                        ask.remaining -= fill_qty;
                        if ask.remaining <= 0.0 {
                            matched_keys.push(key.clone());
                        }
                    } else {
                        break; // No more matchable asks
                    }
                }
                for key in matched_keys {
                    self.asks.remove(&key);
                }
                // If order has remaining quantity, add as resting bid
                if order.remaining > 0.0 {
                    let key = OrderKey {
                        price_cents: -(order.price * 10000.0) as i64, // Negate for descending sort
                        timestamp: order.timestamp,
                        id: order.id.clone(),
                    };
                    self.bids.insert(key, order);
                }
            }
            Side::Sell => {
                // Match against bids (highest price first)
                let mut matched_keys = Vec::new();
                for (key, bid) in self.bids.iter_mut() {
                    if order.remaining <= 0.0 {
                        break;
                    }
                    let bid_price = -(key.price_cents as f64) / 10000.0;
                    if order.price <= bid_price {
                        let fill_qty = order.remaining.min(bid.remaining);
                        trades.push(Trade {
                            buy_order_id: bid.id.clone(),
                            sell_order_id: order.id.clone(),
                            price: bid_price,
                            quantity: fill_qty,
                            timestamp: self.sequence,
                        });
                        order.remaining -= fill_qty;
                        bid.remaining -= fill_qty;
                        if bid.remaining <= 0.0 {
                            matched_keys.push(key.clone());
                        }
                    } else {
                        break;
                    }
                }
                for key in matched_keys {
                    self.bids.remove(&key);
                }
                if order.remaining > 0.0 {
                    let key = OrderKey {
                        price_cents: (order.price * 10000.0) as i64,
                        timestamp: order.timestamp,
                        id: order.id.clone(),
                    };
                    self.asks.insert(key, order);
                }
            }
        }

        trades
    }

    pub fn bid_count(&self) -> usize { self.bids.len() }
    pub fn ask_count(&self) -> usize { self.asks.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_match() {
        let mut book = Orderbook::new("USD/MWK");

        // Seller posts at 1000 MWK/USD
        let sell = Order {
            id: "s1".into(), user_id: "seller1".into(),
            side: Side::Sell, price: 1000.0, quantity: 100.0, remaining: 100.0, timestamp: 0,
        };
        let trades = book.submit_order(sell);
        assert!(trades.is_empty());
        assert_eq!(book.ask_count(), 1);

        // Buyer bids at 1000 MWK/USD
        let buy = Order {
            id: "b1".into(), user_id: "buyer1".into(),
            side: Side::Buy, price: 1000.0, quantity: 100.0, remaining: 100.0, timestamp: 0,
        };
        let trades = book.submit_order(buy);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 100.0);
        assert_eq!(book.ask_count(), 0);
        assert_eq!(book.bid_count(), 0);
    }

    #[test]
    fn test_partial_fill() {
        let mut book = Orderbook::new("USD/MWK");

        let sell = Order {
            id: "s1".into(), user_id: "seller1".into(),
            side: Side::Sell, price: 1000.0, quantity: 200.0, remaining: 200.0, timestamp: 0,
        };
        book.submit_order(sell);

        let buy = Order {
            id: "b1".into(), user_id: "buyer1".into(),
            side: Side::Buy, price: 1000.0, quantity: 50.0, remaining: 50.0, timestamp: 0,
        };
        let trades = book.submit_order(buy);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 50.0);
        assert_eq!(book.ask_count(), 1); // 150 remaining on the ask
    }
}
