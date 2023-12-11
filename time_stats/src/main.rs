#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_time::{Timer,Instant};

use stats::Stats;
use time_stats::TimeStats;

fn show(s: Stats) {
    println!("{} {}/{}/{} {}",
             s.n(), s.min(), s.mean(), s.max(), s.std());
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _ = embassy_stm32::init(Default::default());
    let s = Stats::new();
    let mut ts = TimeStats::new();
    // let mut t = Instant::now().as_millis();
    for _i in 1..=100 {
        // t += 1;
        // Timer::at(Instant::from_millis(t)).await;
        ts.loop_tick();
    }
    show(s);
    Timer::after_millis(100).await;
    println!("Done!");
}
