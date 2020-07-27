#![allow(dead_code, unused_variables)]

mod tangle;
mod utils;

use tangle::{tangle, Message, MessageKind};

use rand::Rng;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

const TPS_IN: u64 = 2;
const TPS_IN_PAUSE: u64 = (1_f64 / (TPS_IN as f64) * 1000_f64) as u64;
const TPS_OUT: u64 = 1; // rename: submit interval?
const TPS_OUT_PAUSE: u64 = (1_f64 / (TPS_OUT as f64) * 1000_f64) as u64;
const MILESTONE_INTERVAL: u64 = 10;
const INVALID_INTERVAL: u64 = 5;

static LAST_TX_ID: AtomicU64 = AtomicU64::new(0);
static IS_MILESTONE: AtomicBool = AtomicBool::new(false);

fn main() {
    let (last_tx_id, last_ms_index) = utils::make_tangle_1_milestone();

    LAST_TX_ID.store(last_tx_id + 1, Ordering::Relaxed);

    let mut handles = Vec::new();

    // insert gossiped transactions (without TSA: simply randomly picked parents with a tendency to pick more
    // recent ones)
    handles.push(thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut ms_index = last_ms_index + 1;

        loop {
            thread::sleep(Duration::from_millis(TPS_IN_PAUSE));

            // Simulate gossip
            let last = LAST_TX_ID.load(Ordering::Relaxed);
            let ma = rng.gen_range(last - 10, last);
            let pa = rng.gen_range(last - 10, last);

            let i = LAST_TX_ID.fetch_add(1, Ordering::Relaxed);

            if IS_MILESTONE.compare_and_swap(true, false, Ordering::Relaxed) {
                println!(
                    "[GOSSIP_IN ] Received milestone with index {} and parents ({},{})",
                    ms_index, ma, pa
                );

                tangle().insert_gossip(
                    i,
                    Message::new((), MessageKind::Milestone(ms_index)),
                    ma,
                    pa,
                );

                ms_index += 1;
            } else {
                println!(
                    "[GOSSIP_IN ] Received transaction: {} with parents ({},{})",
                    i, ma, pa
                );

                tangle().insert_gossip(i, Message::new((), MessageKind::Data), ma, pa);
            }
        }
    }));

    // insert own transactions (with TSA)
    handles.push(thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(TPS_OUT_PAUSE));

        if let Some((ma, pa)) = tangle().select_two_tips() {
            let i = LAST_TX_ID.fetch_add(1, Ordering::Relaxed);

            println!(
                "[BROADCAST ] Created transaction with id={} and parents ({},{})",
                i, ma, pa
            );

            tangle().insert_own(i, Message::new((), MessageKind::Data), ma, pa);
        } else {
            println!("tip pool empty");
        }
    }));

    // TODO: flag an existing transaction as milestone

    // insert gossiped milestones (coordinator TSA: previous milestone in past cone)
    handles.push(thread::spawn(move || {
        loop {
            // Issue a milestone every 10 seconds
            thread::sleep(Duration::from_secs(MILESTONE_INTERVAL));

            IS_MILESTONE.compare_and_swap(false, true, Ordering::Relaxed);
        }
    }));

    for handle in handles.pop() {
        handle.join().expect("error joining handle");
    }
}
