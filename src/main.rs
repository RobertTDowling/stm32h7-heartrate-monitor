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

use {defmt_rtt as _, panic_probe as _};

use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use static_cell::StaticCell;
use heapless::String;
use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicU32;

//
// Things needed for 14-segment driver processing task
//

mod c5412;

// Async communication: value to display, 0-99, to c5412 task
static DISP_VALUE_ATOMIC: AtomicU32 = AtomicU32::new(0);

static C5412PINS_INST: StaticCell<c5412::C5412Pins> = StaticCell::new();

//
// Things needed for HR processing task
//

mod hr_alg3;

// Async communication: ADC value from main (ADC) task to HR processing task
static SAMPLE_SIGNAL: Signal<CriticalSectionRawMutex, u32> = Signal::new();

type UART = embassy_stm32::usart::UartTx<'static, embassy_stm32::peripherals::USART3, embassy_stm32::peripherals::DMA1_CH1>;
static UART_INST: StaticCell<UART> = StaticCell::new();

type LED1 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB0>;
static LED1_INST: StaticCell<LED1> = StaticCell::new();

type LED3 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB14>;
static LED3_INST: StaticCell<LED3> = StaticCell::new();

type BUTTON1 = embassy_stm32::gpio::Input<'static, embassy_stm32::peripherals::PC13>;
static BUTTON1_INST: StaticCell<BUTTON1> = StaticCell::new();

const DUMP_MODE: bool = false;

// Heartrate computation task
// Simply call hr::tick(sample) and output something based on results
#[embassy_executor::task]
async fn process_hr(uart_ref: &'static mut UART,
                    led1_ref: &'static mut LED1,
                    led3_ref: &'static mut LED3,
                    button1_ref: &'static mut BUTTON1,
                    display_value_atomic: &'static AtomicU32)
{
    let mut msg : String<64> = String::new();
    msg.clear();
    core::fmt::write(&mut msg, format_args!("Boot\n")).unwrap();
    _ = (uart_ref).write(msg.as_bytes()).await;
    let mut hr = hr_alg3::Hr::new();
    let mut n0 = 0usize;
    let mut count0 = 0u32;
    loop {
        let sample = SAMPLE_SIGNAL.wait().await;
        let lp = button1_ref.get_level() == Level::Low;
        led3_ref.set_level(if !lp { High } else { Low });
        let (n, cooked_sample, peak, state, hr_update) = hr.tick(lp, sample);
        if hr_update != 0 {
            display_value_atomic.store(hr.hr() as u32, Ordering::Relaxed);
        }
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

//
// Main function sets up I/O and then performs as ADC sampling task
//
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut p = embassy_stm32::init(Default::default());

    // Set up I/O used to HR processing task (to communicate UX, debuging) and
    // place them in statics so we can pass into another task
    let button1 = Input::new(p.PC13, Pull::None);
    let button1_ref = BUTTON1_INST.init(button1);

    let led1 = Output::new(p.PB0, Level::High, Speed::Low);
    let led1_ref = LED1_INST.init(led1);

    let led3 = Output::new(p.PB14, Level::High, Speed::Low);
    let led3_ref = LED3_INST.init(led3);

    let uart = UartTx::new(p.USART3, p.PD8, p.DMA1_CH1, Config::default()).unwrap();
    let uart_ref = UART_INST.init(uart);

    // Kick off the HR processing task
    _ = spawner.spawn(process_hr(uart_ref, led1_ref, led3_ref, button1_ref,
                                 &DISP_VALUE_ATOMIC));

    // Set up the display, and place in static to pass into another task
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

    // Kick off the display task
    _ = spawner.spawn(c5412::process(c5412pins_ref, &DISP_VALUE_ATOMIC));

    // Setup the ADC. This is a bit too fancy right now!
    let mut delay = Delay;
    let mut adc = Adc::new(p.ADC1, &mut delay);
    // Turn on ADC oversampling - reduces noise 
    unsafe { let p : *mut u32 = 0x4002200c as *mut u32; *p = 0x80000008; } // 008=12 bit
    unsafe { let p : *mut u32 = 0x40022010 as *mut u32; *p = 0x000f0001; } // f=16x oversample, 001=ovs on
    // Turn down clock - reduces noise
    unsafe { let p : *mut u32 = 0x40022308 as *mut u32; *p = 6 << 18; } // Slow down clock to /12 (0x0018)

    // Peform the ADC task
    let mut now = Instant::now().as_millis();
    loop {
        now += 1; // Sample at 1kHz -- Using "tick-hz-1_000_000" feature of embassy-time
        Timer::at(Instant::from_millis(now)).await;
        let sample = adc.read(&mut p.PA0) as u32;
        SAMPLE_SIGNAL.signal(sample);
    }
}
