// hr_alg3: Heartrate Algorithm #3, including processing task

use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

const CRAZY_HI: u32 = 3000;
const CRAZY_LO: u32 = 1000;
const DC_ALPHA: f64 = 1.0 / 1000.0;
const LP_ALPHA: f64 = 1.0 / 100.0;
const THRESHOLD_ALPHA_UP: f64 = 1.0 / 100.0;
const THRESHOLD_ALPHA_DN: f64 = 1.0 / 2000.0;
const PEAK_DELAY: usize = 200;

const ABOVE_SIZE: usize = 200;

pub struct Hr {
    dc_ema: f64,        // DC filter
    lp_ema: f64,        // Low Pass filter
    threshold_ema: f64, // Asymmetric filter
    n: usize,           // Monotonic counter of calls to `tick`
    state: u8,
    timer: usize,
    above_pts: ConstGenericRingBuffer<u32, ABOVE_SIZE>,

    last_peak_n: usize,
    hr: f64,
}

impl Hr {
    pub fn new() -> Hr {
        let yc: u32 = 32768; // Assumed center of range for starting filters out
        Hr {
            dc_ema: yc as f64,
            lp_ema: yc as f64,
            threshold_ema: yc as f64,
            n: 0,
            state: 0,
            timer: 0,
            above_pts: ConstGenericRingBuffer::<u32, ABOVE_SIZE>::new(),
            last_peak_n: 0,
            hr: 0.0,
        }
    }
    // Process one sample; output a mess of things....
    //    Currently takes about 40us to complete
    // Parameters:
    //    lp: Low pass input if true
    //    raw_sample: value to process
    // Return tuple:
    //    tick count: number of times we were called since boot
    //    filtered input: either raw_sample or a low-pass version of it
    //    state: 1 if collecting peaks samples, 0 if not
    //    hr_update_flag: 1 if heartrate value was updated this tick
    pub fn tick(&mut self, lp: bool, raw_sample: u32) -> (usize, u32, u8, u8) {
        let mut hr_update_flag: u8 = 0;

        let fx = raw_sample as f64;
        self.dc_ema += (fx - self.dc_ema) * DC_ALPHA;

        let (x, fx) = if lp {
            self.lp_ema += (fx - self.lp_ema) * LP_ALPHA;
            (self.lp_ema as u32, self.lp_ema)
        } else {
            (raw_sample, fx)
        };

        let yc: u32 = self.dc_ema as u32;
        let y0: u32 = yc - CRAZY_LO;
        let y1: u32 = yc + CRAZY_HI;
        if y0 < x && x < y1 {
            if self.threshold_ema < fx {
                self.threshold_ema += (fx - self.threshold_ema) * THRESHOLD_ALPHA_UP;
                if self.state == 0 && self.timer >= PEAK_DELAY {
                    self.state = 1;
                    self.timer = 0;
                    self.above_pts.clear();
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
            if self.state == 1 {
                self.above_pts.push(x);
            }
        } else {
            // Crazy value, reset state machine
            self.state = 0;
            self.timer = 0;
        }
        self.n += 1;
        self.timer += 1;

        // Return Tick count, filtered input, state, hr_update_flag
        (self.n, x, self.state, hr_update_flag)
    }
    // Called internally when exiting state 1, that is, after the peak data has been
    //   collected.  Process it to find the max, and then the inter-peak distance
    //   and ultimately, the heart rate.
    // Return the heartrate
    fn update_hr(&mut self, start_n: usize) -> u32 {
        // Search for peak in above data
        if self.above_pts.capacity() > 1 {
            let mut above_max: u32 = 0;
            let mut above_ix: usize = 0;
            // This is slow when not --release, and after_ticks is the problem
            for (i, val) in self.above_pts.iter().enumerate() {
                // Timer::after_ticks(0).await; // yield
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
    // Return most recent heartrate result
    pub fn hr(&self) -> f64 {
        self.hr
    }
    // Return some internal values for debugging
    pub fn help(&self) -> (u32, u32) {
        (self.dc_ema as u32, self.threshold_ema as u32)
    }
}
