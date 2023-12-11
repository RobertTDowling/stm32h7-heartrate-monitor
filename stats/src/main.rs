#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;

use stats::Stats;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _ = embassy_stm32::init(Default::default());
    let mut s = Stats::new();
    s.add(3.0);
    s.add(4.0);
    s.add(3.0);
    s.add(4.0);
    println!("{}", s.n());
    println!("{}", s.min());
    println!("{}", s.max());
    println!("{}", s.mean());
    println!("{}", s.std()); // Sqrt(1/3)
}
