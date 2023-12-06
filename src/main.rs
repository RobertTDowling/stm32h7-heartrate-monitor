#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#[allow(arithmetic_overflow)]

// use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::gpio::{Level, Input, Output, Pull, Speed};
use embassy_stm32::usart::{Config, UartTx};
use embassy_time::{Timer,Instant,Delay};
use embassy_stm32::gpio::Level::{High,Low};

use heapless::String;
use {defmt_rtt as _, panic_probe as _};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};

use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use static_cell::StaticCell;

mod c5412;

pub static C5412PINS_INST: StaticCell<c5412::C5412Pins> = StaticCell::new();

const CRAZY_HI: i32 =3000;
const CRAZY_LO: i32 =-1000;
const DC_ALPHA: f64 = 1.0/1000.0;
const LP_ALPHA: f64 = 1.0/100.0;
const THRESHOLD_ALPHA_UP: f64 = 1.0/100.0;
const THRESHOLD_ALPHA_DN: f64 = 1.0/2000.0;
const PEAK_DELAY: usize = 200;

const ABOVE_SIZE: usize = 200;
const BELOW_SIZE: usize = 200;

const DUMP_MODE: bool = true;

struct Hr {
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
    fn tick(&mut self, lp: bool, raw_sample: u32) -> (usize, u32, u8, u8) { // 40us
        self.peak_flag = 0;
        self.wild_flag = 0;

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
        (self.n, x, self.peak_flag, self.state) // self.peak_flag)
    }
    fn update_hr(&mut self, start_n : usize) {
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

            c5412::set_value(self.hr as u32);
        }
    }
    fn hr(&self) -> f64 { self.hr }
    fn above_below(&self) -> (i32, i32) { // 140us
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
    fn new() -> Hr {
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
    fn help(&self) -> (u32, u32) {
        (self.dc_ema as u32, self.threshold_ema as u32)
    }
}

static SAMPLE_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();

type UART = embassy_stm32::usart::UartTx<'static, embassy_stm32::peripherals::USART3, embassy_stm32::peripherals::DMA1_CH1>;
static UART_INST: StaticCell<UART> = StaticCell::new();

type LED1 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB0>;
static LED1_INST: StaticCell<LED1> = StaticCell::new();
type LED3 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB14>;
static LED3_INST: StaticCell<LED3> = StaticCell::new();

type BUTTON1 = embassy_stm32::gpio::Input<'static, embassy_stm32::peripherals::PC13>;
static BUTTON1_INST: StaticCell<BUTTON1> = StaticCell::new();

#[embassy_executor::task]
async fn process(uart_ref: &'static mut UART,
                 led1_ref: &'static mut LED1,
                 led3_ref: &'static mut LED3,
                 button1_ref: &'static mut BUTTON1) {
    let mut msg : String<64> = String::new();
    msg.clear();
    core::fmt::write(&mut msg, format_args!("Boot\n")).unwrap();
    _ = (uart_ref).write(msg.as_bytes()).await;
    let mut hr = Hr::new();
    let mut n0 = 0usize;
    let mut count0 = 0u32;
    loop {
        let sample = SAMPLE_SIGNAL.wait().await;
        let lp = button1_ref.get_level() == Level::Low;
        led3_ref.set_level(if !lp { High } else { Low });
        let (n, cooked_sample, peak, state) = hr.tick(lp, sample);
        led1_ref.set_level(if state != 0 { High } else { Low });
        if DUMP_MODE {
            msg.clear();
            // core::fmt::write(&mut msg, format_args!("{} {}\n",  sample, state+peak)).unwrap();
            core::fmt::write(&mut msg, format_args!("{} {}\n",  cooked_sample, if lp {1} else {0})).unwrap();
            // core::fmt::write(&mut msg, format_args!("{}\n",  sample)).unwrap();
            _ = (uart_ref).write(msg.as_bytes()).await;
            continue;
        }
        if peak != 0 {
            let rate = hr.hr();
            let (a, b) = hr.above_below();
            let d = a-b;
            msg.clear();
            let count = c5412::get_count();
            let refresh = 1000f64 * (count-count0) as f64 / (n-n0) as f64;
            core::fmt::write(&mut msg, format_args!("{} {} rate={:.2} refresh={:.2}\n",
                                                    n-n0, d, rate, refresh)).unwrap();
            _ = (uart_ref).write(msg.as_bytes()).await;
            count0 = count;
            n0 = n;
        } else if n-n0 > 3000 {
            let (dc, thresh) = hr.help();
            msg.clear();
            core::fmt::write(&mut msg, format_args!("Help: {} {}\n",  dc, thresh)).unwrap();
            _ = uart_ref.write(msg.as_bytes()).await;
            n0 = n;
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut p = embassy_stm32::init(Default::default());
    let button1 = Input::new(p.PC13, Pull::None);
    let led1 = Output::new(p.PB0, Level::High, Speed::Low);
    let led3 = Output::new(p.PB14, Level::High, Speed::Low);
    let uart = UartTx::new(p.USART3, p.PD8, p.DMA1_CH1, Config::default()).unwrap();
    let mut delay = Delay;
    let mut adc = Adc::new(p.ADC1, &mut delay);

    // Turn on ADC oversampling
    // 0x4002200c: 0x80000008
    // 0x40022010: 0x000f0001 // f=16x oversample, 001 = oversample on
    unsafe { let p : *mut u32 = 0x4002200c as *mut u32; *p = 0x80000008; } // 008=12 bit
    unsafe { let p : *mut u32 = 0x40022010 as *mut u32; *p = 0x000f0001; } // f=16x oversample, 001=ovs on
    // Turn down clock
    unsafe { let p : *mut u32 = 0x40022308 as *mut u32; *p = 6 << 18; } // Slow down clock to /12 (0x0018)

    let uart_ref = UART_INST.init(uart);
    let led1_ref = LED1_INST.init(led1);
    let led3_ref = LED3_INST.init(led3);
    let button1_ref = BUTTON1_INST.init(button1);
    _ = spawner.spawn(process(uart_ref, led1_ref, led3_ref, button1_ref));

    let c5412pins = c5412::C5412Pins {
        p11: Output::new(p.PD7, Level::High, Speed::Low).degrade(),
        p12: Output::new(p.PD6, Level::High, Speed::Low).degrade(),
        p13: Output::new(p.PD5, Level::High, Speed::Low).degrade(),
        p14: Output::new(p.PD4, Level::High, Speed::Low).degrade(),
        p15: Output::new(p.PD3, Level::High, Speed::Low).degrade(),
        p16: Output::new(p.PE2, Level::High, Speed::Low).degrade(),
        p17: Output::new(p.PF2, Level::High, Speed::Low).degrade(),
        p18: Output::new(p.PF1, Level::High, Speed::Low).degrade(),
        p21: Output::new(p.PE4, Level::High, Speed::Low).degrade(),
        p22: Output::new(p.PE5, Level::High, Speed::Low).degrade(),
        p23: Output::new(p.PE6, Level::High, Speed::Low).degrade(),
        p24: Output::new(p.PE3, Level::High, Speed::Low).degrade(),
        p25: Output::new(p.PF8, Level::High, Speed::Low).degrade(),
        p26: Output::new(p.PF7, Level::High, Speed::Low).degrade(),
        p27: Output::new(p.PF9, Level::High, Speed::Low).degrade(),
        p28: Output::new(p.PG1, Level::High, Speed::Low).degrade(),
        sah: Output::new(p.PC0, Level::High, Speed::Low).degrade(),
        sbh: Output::new(p.PB1, Level::High, Speed::Low).degrade(),
        sch: Output::new(p.PD1, Level::High, Speed::Low).degrade(),
        sdh: Output::new(p.PD0, Level::High, Speed::Low).degrade(),
        seh: Output::new(p.PG0, Level::High, Speed::Low).degrade(),
        sfh: Output::new(p.PF10, Level::High, Speed::Low).degrade(),
        sjh: Output::new(p.PF0, Level::High, Speed::Low).degrade(),
        snh: Output::new(p.PA3, Level::High, Speed::Low).degrade(),
    };
    let c5412pins_ref = C5412PINS_INST.init(c5412pins);

    _ = spawner.spawn(c5412::process(c5412pins_ref));

    let mut now = Instant::now().as_millis();
    loop {
        now += 1;
        Timer::at(Instant::from_millis(now)).await;
        let sample = adc.read(&mut p.PA0) as u32;
        SAMPLE_SIGNAL.signal(sample);
    }
}
