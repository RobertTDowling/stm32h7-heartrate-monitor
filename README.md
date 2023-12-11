# stm32h7-heartrate-monitor
## Parallel Drive 7-Segment Rust + Embassy Multi-threaded Async/Await Heartrate Monitor on STM32H743 Nucleo
![h743 nucleo and 14seg breadboard with HR sensor](/doc/h743-hr-14seg-plank1.jpg)

* Developed in Rust using [Embassy](https://embassy.dev/) [(Github)](https://github.com/embassy-rs/embassy) async/await cooperative multitasking
* [STM32H743 Nucleo](https://www.amazon.com/s?k=raspberry+pi+heart+rate+sensor) devkit with integrated ST-Link programmer and FTDI-compatible USB Serial link
* Off-the-shelf [Raspberry Pi Heartrate Sensor](https://www.amazon.com/s?k=raspberry+pi+heart+rate+sensor) sampled on ADC
* Surplus Common Cathode 2-digit 14-Segment display (used in 7-segment mode)
* Parallel 7-Segment LED Drive without series resistors, driven directly from MCU pin drivers (EE's, don't look!)
  * Ganged cathode drivers for increased current capacity when multiple segments lit at once
  * Bit-banged software PWM to limit long-term current
* Noise filtering and pulse wave peak detection in real-time


![HeartRate 3 Algorithm Block Diagram](/doc/HeartRateAlg3.png)
