#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
use core::sync::atomic::{AtomicU32, Ordering};

#[allow(arithmetic_overflow)]
// use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, Resolution};
use embassy_stm32::gpio::Level::{High, Low};
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::usart::{Config, UartTx};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Instant, Timer};
use heapless::String;
use static_cell::StaticCell;
use stats::Stats;
use time_stats::TimeStats;
use {defmt_rtt as _, panic_probe as _};

//
// Debug configuration
//
#[allow(dead_code)]
#[derive(PartialEq, Eq)]
enum HrDebugMode {
    Info,           // Show difference bewteen adc and process_hr counts
    Debug,          // Show that adc, now.as_millis and process_hr are all in lockstep
    Stats,          // Show lots of stuff including min/mean/max time in hr task
    DisplayOverrun, // Show display task overrun counter
}
#[allow(dead_code)]
#[derive(PartialEq, Eq)]
enum DebugMode {
    None,
    Hr(HrDebugMode),
    DumpSamples, // Display raw samples
    DumpTiming,  // Display time spent in ADC and process_hr task
}

const DEBUG_MODE: DebugMode = DebugMode::DumpSamples;

//
// Things needed for 14-segment driver processing task
//

mod c5412;

static C5412PINS_INST: StaticCell<c5412::C5412Pins> = StaticCell::new();

// Async communication: value to display, 0-99, to c5412 task
static DISP_VALUE_ATOMIC: AtomicU32 = AtomicU32::new(0);

//
// Things needed for HR processing task
//

mod hr_alg3;

// Async communication: ADC overrun detection.  Expect ADC_N == elapsed millis
static ADC_N_ATOMIC: AtomicU32 = AtomicU32::new(0);

// Async communication: ADC value from main (ADC) task to HR processing task
// Worst case seen was 150ms delay during one version of HR processing,
// so sized channel to be somewhat larger, at 1kHz sample rate.
// Note that if sending to the channel overruns, the ADC task will panic, so
// it should be easyish to tune this value.
static SAMPLE_CHANNEL: Channel<CriticalSectionRawMutex, u32, 200> = Channel::new();

//
// Gymnastics to pass peripherals into tasks.
// The "type" trick gets around tasks not allowing generics yet.
// The "static" instances allow the peripherals, which themselves are statics,
// to keep their static lifetime designation when passed in the task, which
// itself is static.
//
type UART =
    embassy_stm32::usart::UartTx<'static, embassy_stm32::peripherals::USART3, embassy_stm32::peripherals::DMA1_CH1>;
static UART_INST: StaticCell<UART> = StaticCell::new();

type LED1 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB0>;
static LED1_INST: StaticCell<LED1> = StaticCell::new();

type LED3 = embassy_stm32::gpio::Output<'static, embassy_stm32::peripherals::PB14>;
static LED3_INST: StaticCell<LED3> = StaticCell::new();

type BUTTON1 = embassy_stm32::gpio::Input<'static, embassy_stm32::peripherals::PC13>;
static BUTTON1_INST: StaticCell<BUTTON1> = StaticCell::new();

// Heartrate computation task
// Simply call hr::tick(sample) and output something based on results
#[embassy_executor::task]
async fn process_hr(
    uart_ref: &'static mut UART,
    led1_ref: &'static mut LED1, // Used to show pulse
    led3_ref: &'static mut LED3, // Used to show "lp" flag for debugging
    button1_ref: &'static mut BUTTON1,
    display_value_atomic: &'static AtomicU32,
) {
    let mut msg: String<128> = String::new();
    msg.clear();
    core::fmt::write(&mut msg, format_args!("Boot\n")).unwrap();
    _ = (uart_ref).write(msg.as_bytes()).await;
    let mut hr = hr_alg3::Hr::new();
    let mut count0 = 0u32;
    let mut proc_n0 = 0usize;
    let mut adc_n0 = ADC_N_ATOMIC.load(Ordering::Relaxed);
    let mut now0 = Instant::now().as_micros();
    let mut ts = TimeStats::new();
    let mut s = Stats::new();
    ts.loop_tick();
    loop {
        let sample = SAMPLE_CHANNEL.receive().await;
        ts.loop_tick();
        let now = Instant::now().as_micros();
        let adc_n = ADC_N_ATOMIC.load(Ordering::Relaxed);
        let lp = button1_ref.get_level() == Level::Low;
        let count = c5412::get_count();
        led3_ref.set_level(if !lp { High } else { Low });
        let (proc_n, cooked_sample, state, hr_update) = hr.tick(lp, sample);
        // If we got a heartrate update, reflect it on LED
        if hr_update != 0 {
            display_value_atomic.store(hr.hr() as u32, Ordering::Relaxed);
        }
        led1_ref.set_level(if state != 0 { High } else { Low });
        match DEBUG_MODE {
            DebugMode::DumpTiming => {
                let dadc_n = adc_n - adc_n0;
                let dnow = now - now0;
                msg.clear();
                core::fmt::write(&mut msg, format_args!("{} {}\n", dadc_n, dnow)).unwrap();
                _ = (uart_ref).write(msg.as_bytes()).await;
                adc_n0 = adc_n;
                now0 = now;
                // NOTE: we restart loop early here to avoid other UART output!
                continue;
            }
            DebugMode::DumpSamples => {
                msg.clear();
                core::fmt::write(
                    &mut msg,
                    format_args!("{} {:.1}\n", cooked_sample, if hr_update != 0 { hr.hr() } else { 0.0 }),
                )
                .unwrap();
                _ = (uart_ref).write(msg.as_bytes()).await;
                // NOTE: we restart loop early here to avoid other UART output!
                continue;
            }
            DebugMode::Hr(m) => {
                // If we got a heartrate update, reflect it on UART console
                if hr_update != 0 {
                    let rate = hr.hr();
                    let dcount = count - count0;
                    let dproc_n = proc_n - proc_n0;
                    let dadc_n = adc_n - adc_n0;
                    let dnow = now - now0;
                    let refresh = 1000000f64 * dcount as f64 / dnow as f64;
                    msg.clear();
                    match m {
                        HrDebugMode::Stats => {
                            ts.stats(&mut s);
                            core::fmt::write(&mut msg, format_args!("rate={:.2} refresh={:.2} dcount={} dproc={} dadc={} dnow={} n:{} {:.2}/{:.1}/{:.2} {:.2}\n",
                                                                    rate, refresh, dcount, dproc_n, dadc_n, dnow,
                                                                    s.n(), s.min(), s.mean(), s.max(), s.std())).unwrap();
                            ts.reset();
                        }
                        HrDebugMode::Debug => {
                            core::fmt::write(
                                &mut msg,
                                format_args!(
                                    "rate={:.2} refresh={:.2} dcount={} dproc={} dadc={} dnow={}\n",
                                    rate, refresh, dcount, dproc_n, dadc_n, dnow
                                ),
                            )
                            .unwrap();
                        }
                        HrDebugMode::Info => {
                            let err = dadc_n as i32 - dproc_n as i32;
                            core::fmt::write(&mut msg, format_args!("{:.2} {:.2} {}\n", rate, refresh, err)).unwrap();
                        }
                        HrDebugMode::DisplayOverrun => {
                            let overrun = c5412::get_overrun();
                            core::fmt::write(&mut msg, format_args!("{:.2} {:.2} {}\n", rate, refresh, overrun))
                                .unwrap();
                        }
                    }
                    _ = (uart_ref).write(msg.as_bytes()).await;
                    count0 = count;
                    adc_n0 = adc_n;
                    proc_n0 = proc_n;
                    now0 = now;
                }
            }
            DebugMode::None => {}
        }
        // Put some feedback on the console if no pulse for 3 seconds
        if proc_n - proc_n0 > 3000 {
            let (dc, thresh) = hr.help();
            msg.clear();
            core::fmt::write(&mut msg, format_args!("Help: {} {}\n", dc, thresh)).unwrap();
            _ = uart_ref.write(msg.as_bytes()).await;
            proc_n0 = proc_n;
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
    _ = spawner.spawn(process_hr(
        uart_ref,
        led1_ref,
        led3_ref,
        button1_ref,
        &DISP_VALUE_ATOMIC,
    ));

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

    //
    // Setup the ADC. This is a bit fancy right now!
    //
    let mut delay = Delay;
    let mut adc = Adc::new(p.ADC1, &mut delay);

    // Configure things that don't come with default:
    //    Turn on 16x oversampling to reduce noise
    //        Need to reduce sampling resolution to 12 bit
    //        And set oversampling to 16x and enable it
    //    Slow down ADC clock to /12 to also reduce noise

    // Reduce resolution: that is exposed in Embassy HAL
    adc.set_resolution(Resolution::TwelveBit);

    // Turn on oversampling directly using PAC
    let adc1 = embassy_stm32::pac::ADC1;
    adc1.cfgr2().modify(|m| m.set_osvr(0xf));
    adc1.cfgr2().modify(|m| m.set_rovse(true));

    // Slow down clock directly using PAC
    let adcc = embassy_stm32::pac::ADC_COMMON;
    adcc.ccr()
        .modify(|m| m.set_presc(embassy_stm32::pac::adccommon::vals::Presc::DIV12));

    // Peform the ADC task
    let mut now = Instant::now().as_millis();
    loop {
        now += 1; // Sample at 1kHz -- Using "tick-hz-1_000_000" feature of embassy-time
        ADC_N_ATOMIC.store(now as u32, Ordering::Relaxed);
        Timer::at(Instant::from_millis(now)).await;
        let sample = adc.read(&mut p.PA0) as u32;
        SAMPLE_CHANNEL.try_send(sample).expect("adc sample channel overrun");
    }
}
