#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

//use libm::{sqrt,fmin,fmax};
//use heapless::String;
use embassy_time::{Instant,Duration};
use stats::Stats;

pub struct TimeStats {
    started: bool,
    start: Instant,
    stop: Instant,
    delta: Duration,
    stats: Stats
}

impl TimeStats {
    pub fn new() -> Self {
        Self {
            stats: Stats::new(),
            started: false,
            start: Instant::from_ticks(0),
            stop: Instant::from_ticks(0),
            delta: Duration::from_ticks(0),
        }
    }
    pub fn loop_tick(&mut self) {
        if self.started {
            self.stop = Instant::now();
            self.delta = self.stop - self.start;
            self.start = self.stop;
            self.stats.add(self.delta.as_micros() as f64)
        } else {
            self.start = Instant::now();
            self.started = true;
        }
    }
    pub fn start_tick(&mut self) {
        if self.started {
            panic!("Can't do this")
        } else {
            self.start = Instant::now();
            self.started = true;
        }
    }
    pub fn stop_tick(&mut self) {
        if self.started {
            self.stop = Instant::now();
            self.delta = self.stop - self.start;
            self.started = false;
            // self.start = self.stop;
            self.stats.add(self.delta.as_micros() as f64)
        } else {
            panic!("Can't do this")
        }
    }
    pub fn reset(&mut self) {
        self.stats.reset();
    }
    pub fn stats(&self, stats: &mut Stats) {
        *stats = self.stats;
    }
}
