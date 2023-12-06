use embassy_stm32::gpio::AnyPin;
use embassy_stm32::gpio::Level::{High,Low};
use embassy_time::Timer;

use core::sync::atomic::Ordering;
use core::sync::atomic::AtomicU32;

static COUNT: AtomicU32 = AtomicU32::new(0);
static VALUE: AtomicU32 = AtomicU32::new(0);

pub struct C5412Pins {
    pub p11: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p12: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p13: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p14: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p15: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p16: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p17: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p18: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p21: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p22: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p23: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p24: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p25: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p26: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p27: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub p28: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sah: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sbh: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sch: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sdh: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub seh: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sfh: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub sjh: embassy_stm32::gpio::Output<'static, AnyPin>,
    pub snh: embassy_stm32::gpio::Output<'static, AnyPin>,
}

impl C5412Pins {
    pub fn common_off(&mut self) {
        self.p11.set_level(High);
        self.p12.set_level(High);
        self.p13.set_level(High);
        self.p14.set_level(High);
        self.p15.set_level(High);
        self.p16.set_level(High);
        self.p17.set_level(High);
        self.p18.set_level(High);
        self.p21.set_level(High);
        self.p22.set_level(High);
        self.p23.set_level(High);
        self.p24.set_level(High);
        self.p25.set_level(High);
        self.p26.set_level(High);
        self.p27.set_level(High);
        self.p28.set_level(High);
    }
    pub fn a_n_off(&mut self) {
        self.sah.set_level(Low);
        self.sbh.set_level(Low);
        self.sch.set_level(Low);
        self.sdh.set_level(Low);
        self.seh.set_level(Low);
        self.sfh.set_level(Low);
        self.sjh.set_level(Low);
        self.snh.set_level(Low);
    }

    pub fn all_off(&mut self) {
        self.common_off();
        self.a_n_off();
    }

    pub fn common_1_on(&mut self) {
        self.p11.set_level(Low);
        self.p12.set_level(Low);
        self.p13.set_level(Low);
        self.p14.set_level(Low);
        self.p15.set_level(Low);
        self.p16.set_level(Low);
        self.p17.set_level(Low);
        self.p18.set_level(Low);
    }

    pub fn common_2_on(&mut self) {
        self.p21.set_level(Low);
        self.p22.set_level(Low);
        self.p23.set_level(Low);
        self.p24.set_level(Low);
        self.p25.set_level(Low);
        self.p26.set_level(Low);
        self.p27.set_level(Low);
        self.p28.set_level(Low);
    }

    pub fn digit_on(&mut self, digit: u8) {
        match digit {
            0 => { self.sah.set_level(High); self.sbh.set_level(High); self.sch.set_level(High); self.sdh.set_level(High);
                   self.seh.set_level(High); self.sfh.set_level(High);                                                     }
            1 => {                           self.sbh.set_level(High); self.sch.set_level(High);
                                                                                                                           }
            2 => { self.sah.set_level(High); self.sbh.set_level(High);                           self.sdh.set_level(High);
                   self.seh.set_level(High);                           self.sjh.set_level(High); self.snh.set_level(High); }
            3 => { self.sah.set_level(High); self.sbh.set_level(High); self.sch.set_level(High); self.sdh.set_level(High);
                   self.sjh.set_level(High); self.snh.set_level(High); }
            4 => {                           self.sbh.set_level(High); self.sch.set_level(High);
                                             self.sfh.set_level(High); self.sjh.set_level(High); self.snh.set_level(High); }
            5 => { self.sah.set_level(High);                           self.sch.set_level(High); self.sdh.set_level(High);
                   self.sfh.set_level(High); self.sjh.set_level(High); self.snh.set_level(High); }
            6 => { self.sah.set_level(High);                           self.sch.set_level(High); self.sdh.set_level(High);
                   self.seh.set_level(High); self.sfh.set_level(High); self.sjh.set_level(High); self.snh.set_level(High); }
            7 => { self.sah.set_level(High); self.sbh.set_level(High); self.sch.set_level(High);
                                                                                                                           }
            8 => { self.sah.set_level(High); self.sbh.set_level(High); self.sch.set_level(High); self.sdh.set_level(High);
                   self.seh.set_level(High); self.sfh.set_level(High); self.sjh.set_level(High); self.snh.set_level(High); }
            9 => { self.sah.set_level(High); self.sbh.set_level(High); self.sch.set_level(High); self.sdh.set_level(High);
                   self.sfh.set_level(High); self.sjh.set_level(High); self.snh.set_level(High); }
            _ => {}
        }
    }
}

#[embassy_executor::task]
pub async fn process(c5412pins_ref: &'static mut C5412Pins) {
    const M : u64 = 5;
    const N : u64 = 2;
    let mut count : u32 = 0;
    loop {
        COUNT.store(count, Ordering::Relaxed);
        let x : u32 = VALUE.load(Ordering::Relaxed);

        c5412pins_ref.all_off();
        Timer::after_millis(M).await;
        c5412pins_ref.common_1_on();
        c5412pins_ref.digit_on(((x/10)%10) as u8);
        Timer::after_millis(N).await;

        c5412pins_ref.all_off();
        Timer::after_millis(M).await;
        c5412pins_ref.common_2_on();
        c5412pins_ref.digit_on((x%10) as u8);
        Timer::after_millis(N).await;

        count+=1;
    }
}

pub fn set_value(value: u32) {
    VALUE.store(value, Ordering::Relaxed);
}

pub fn get_count() -> u32 {
    COUNT.load(Ordering::Relaxed)
}