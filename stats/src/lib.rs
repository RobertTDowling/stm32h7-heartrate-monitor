#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use libm::{sqrt,fmin,fmax};

#[derive(Copy, Clone)]
pub struct Stats {
    n : f64,
    sx : f64,
    sxx : f64,
    minx : f64,
    maxx : f64
}

impl Stats {
    pub fn new() -> Self { Self { n:0.0, sx:0.0, sxx:0.0, minx:0.0, maxx:0.0}}
    pub fn reset(&mut self) { self.n=0.0; self.sx=0.0; self.sxx=0.0;
                              self.minx=0.0; self.maxx=0.0;}
    pub fn add(&mut self, x:f64) {
        if self.n == 0.0 {
            self.minx = x;
            self.maxx = x;
        } else {
            self.minx = fmin(self.minx, x);
            self.maxx = fmax(self.maxx, x);
        }
        self.n += 1.0;
        self.sx += x;
        self.sxx += x*x;
    }
    pub fn n(&self) -> u32 { self.n as u32 }
    pub fn min(&self) -> f64 {
        if self.n > 0.0 { self.minx } else { f64::NAN }
    }
    pub fn max(&self) -> f64 {
        if self.n > 0.0 { self.maxx } else { f64::NAN }
    }
    pub fn mean(&self) -> f64 {
        if self.n > 0.0 { self.sx/self.n } else { f64::NAN }
    }
    pub fn std(&self) -> f64 {
        if self.n > 1.0 {
            sqrt((self.sxx - self.sx*self.sx/self.n) / (self.n-1.0))
        } else { f64::NAN }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut s = Stats::new();
        s.add(3.0);
        s.add(4.0);
        s.add(3.0);
        s.add(4.0);
        assert_eq!(s.n(), 4u32);
        assert_eq!(s.min(), 3.0);
        assert_eq!(s.max(), 4.0);
        assert_eq!(s.mean(), 3.5);
        assert_eq!(s.std(), sqrt(1.0/3.0));
    }
}
