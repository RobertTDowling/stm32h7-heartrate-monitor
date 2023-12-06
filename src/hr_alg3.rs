// hr_alg3: Heartrate Algorithm #3, including processing task

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

const CRAZY_HI: i32 =3000;
const CRAZY_LO: i32 =-1000;
const DC_ALPHA: f64 = 1.0/1000.0;
const LP_ALPHA: f64 = 1.0/100.0;
const THRESHOLD_ALPHA_UP: f64 = 1.0/100.0;
const THRESHOLD_ALPHA_DN: f64 = 1.0/2000.0;
const PEAK_DELAY: usize = 200;

const ABOVE_SIZE: usize = 200;
const BELOW_SIZE: usize = 200;

pub struct Hr {
    dc_ema: f64,
    lp_ema: f64,
    threshold_ema: f64,
    n : usize,
    state : u8,
    timer : usize,
    peak_flag : u8,
    wild_flag : u8,
    above_pts : ConstGenericRingBuffer<i32, ABOVE_SIZE>,
    below_pts : ConstGenericRingBuffer<i32, BELOW_SIZE>,

    last_peak_n : usize,
    hr : f64,
}

impl Hr {
    pub fn tick(&mut self, lp: bool, raw_sample: u32) -> (usize, u32, u8, u8, u8) { // 40us
        self.peak_flag = 0;
        self.wild_flag = 0;
        let mut hr_update_flag : u8 = 0;

        let fx = raw_sample as f64;
        self.dc_ema += (fx - self.dc_ema) * DC_ALPHA;

        let (x, fx) = if lp {
            self.lp_ema += (fx - self.lp_ema) * LP_ALPHA;
            (self.lp_ema as u32, self.lp_ema)
        } else {
            (raw_sample, fx)
        };

        let yc: u32 = self.dc_ema as u32;
        let y0: u32 = ((yc as i32)+CRAZY_LO) as u32;
        let y1: u32 = ((yc as i32)+CRAZY_HI) as u32;
        if y0 < x && x < y1 {
            if self.threshold_ema < fx {
                self.threshold_ema += (fx - self.threshold_ema) * THRESHOLD_ALPHA_UP;
                if self.state == 0 && self.timer >= PEAK_DELAY {
                    self.state = 1;
                    self.timer = 0;
                    self.peak_flag = 1;
                }
            } else {
                self.threshold_ema += (fx - self.threshold_ema) * THRESHOLD_ALPHA_DN;
                if self.state == 1 && self.timer >= PEAK_DELAY {
                    self.update_hr(self.n - PEAK_DELAY);
                    hr_update_flag = 1;
                    self.state = 0;
                    self.timer = 0;
                }
            }
            if self.state == 0 {
                self.below_pts.push(x as i32 - self.threshold_ema as i32);
            } else {
                self.above_pts.push(x as i32 - self.threshold_ema as i32);
            }
        } else {
            self.state = 0;
            self.timer = 0;
            self.wild_flag = 1;
        }
        self.n += 1;
        self.timer += 1;

        // Return Tick count, filtered input, peak_flag, state, hr_update_flag
        (self.n, x, self.peak_flag, self.state, hr_update_flag)
    }
    fn update_hr(&mut self, start_n : usize) -> u32 {
        // Search for peak in above data
        if self.above_pts.capacity() > 1 {
            let mut above_max = 0i32;
            let mut above_ix = 0usize;
            for (i, val) in self.above_pts.iter().enumerate() {
                if above_max < *val {
                    above_max = *val;
                    above_ix = i;
                }
            }
            // Given when above_pts started, and above_ix, calc delta to last peak
            let this_peak_n = start_n + above_ix;
            let delta_n = this_peak_n - self.last_peak_n;
            self.last_peak_n = this_peak_n;

            // if delta > 200 && delta < 2000 {
            self.hr = 60000f64 / delta_n as f64;

            self.hr as u32
        } else {
            0
        }
    }
    pub fn hr(&self) -> f64 { self.hr }
    pub fn above_below(&self) -> (i32, i32) { // 140us
        let mut above = 0i32;
        if self.above_pts.capacity() > 1 {
            let i = self.above_pts.iter();
            for ii in i {
                if above < *ii {
                    above = *ii;
                }
            }
        }
        let mut below = 0i32;
        if self.below_pts.capacity() > 1 {
            let i = self.below_pts.iter();
            for ii in i {
                if below > *ii {
                    below = *ii;
                }
            }
        }
        (above, below)
    }
    pub fn new() -> Hr {
        let yc = 32768i32;
        Hr {
            dc_ema: yc as f64,
            lp_ema: yc as f64,
            threshold_ema: yc as f64,
            n : 0usize,
            state : 0u8,
            timer : 0usize,
            peak_flag : 0u8,
            wild_flag : 0u8,
            above_pts : ConstGenericRingBuffer::<i32, ABOVE_SIZE>::new(),
            below_pts : ConstGenericRingBuffer::<i32, BELOW_SIZE>::new(),
            last_peak_n : 0usize,
            hr : 0f64,
        }
    }
    pub fn help(&self) -> (u32, u32) {
        (self.dc_ema as u32, self.threshold_ema as u32)
    }
}
