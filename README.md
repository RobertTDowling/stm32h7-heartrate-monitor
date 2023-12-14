# stm32h7-heartrate-monitor
## Parallel Drive 7-Segment Rust + Embassy Multi-threaded Async/Await Heart Rate Monitor on STM32H743 Nucleo
![h743 nucleo and 14seg breadboard with HR sensor](/doc/h743-hr-14seg-plank1.jpg)

* Developed in Rust using [Embassy](https://embassy.dev/) [(Github)](https://github.com/embassy-rs/embassy) async/await cooperative multitasking
* [STM32H743 Nucleo](https://www.amazon.com/s?k=raspberry+pi+heart+rate+sensor) devkit with integrated ST-Link programmer and FTDI-compatible USB Serial link
* Off-the-shelf [Raspberry Pi Heart Rate Sensor](https://www.amazon.com/s?k=raspberry+pi+heart+rate+sensor) sampled on ADC
* Surplus Common Cathode 2-digit 14-Segment display (used in 7-segment mode)
* Parallel 7-Segment LED Drive without series resistors, driven directly from MCU pin drivers (EE's, don't look!)
  * Ganged cathode drivers for increased current capacity when multiple segments lit at once
  * Bit-banged software PWM to limit long-term current
* Noise filtering and pulse wave peak detection in real-time

# Stuff to write about

* Project Goals
  * Explore cooperative async/await as an alternative to time slice multitasking
  * Drive an LED display without driver transistors or resistors
  * Explore heart rate calculations needed in [photoplethysmography](https://en.wikipedia.org/wiki/Photoplethysmogram)

## Hardware Architecture

* Goals
  * All off-the-shelf hardware with exception of bare 7-segment LED display
* BOM
  * RaspberryPi/Arduino-style 3.3V analog 3-pin pulse heart rate sensor with integrated low noise signal amp, $4 (Amazon)
For a stand-alone CubeMX data acquisition
  * [NUCLEO-L073RZ](https://www.mouser.com/ProductDetail/STMicroelectronics/NUCLEO-L073RZ) devkit with USB power and built in FTDI and ST-Link, $14 (mouser)
For a well supported Rust embedded platform:
  * [NUCLEO-H743ZI2](https://www.mouser.com/ProductDetail/511-NUCLEO-H743ZI2) devkit with USB power and built in FTDI and ST-Link, $27 (mouser)
  * Surplus 7-segment Common Cathode LED display

## Firmware Architecture

* Cooperative multitasking with quasi-real-time requirement in sampling and display tasks
  * ADC sampling task ticks at 1kHz. Ideally a very precise 1kHz for signal processing reasons
  * Display task needs to tick overall at >50Hz to avoid flicker. Variations of or off periods will appear as visual glitches or brighter or darker digits
  * HR task takes up the background processing slack, but at this time, only the UART I/O and sample channel operate async. Ideally, the processing would also have scheduler yields embedded in it, but without compiler optimization, they noticeably degrade the performance of the display task, so they were removed.  Something to revisit and explain!

![HR FW Task Diagram](/doc/HR%20FW%20Architecture.png)


* Reading the ADC
  * Simply sample ADC at a regular rate and place output in Channel queue for processing later
  * ADC configured to minimize noise
    * 16x oversampling enabled (with `unsafe` raw point: FIXME, use `stm-metapac`) with no down-shift on 12 bit conversion to result in 16 bit samples
    * Changes to ADC Sample/Hold did not seem to have any effect on noise, so lowered ADC clock rate instead and found sweet spot at about 1/10th the default sample rate
    * ADC sampling takes about 50us in this mode, far less than 1kHz overall sample rate.
  * Size of channel determined empirically by watching amount of overruns with various processing and I/O loads during algorithm development
* Heart Rate Task for processing samples
  * This is the only task with access to the UART, so some strange things are done like messaging metrics out of the display task so they can be logged here.
  * The processing is considered "background" since it only has to do minimal processing at the sample rate, and longer processing is done at the heart beat rate, about 1/1000 of the sample rate.
* Driving the Display
  * All LED inputs are driven directly from MCU GPIO output pins, which have an assumed lowish current limit of approximately 20mA (FIXME: Check this)
  * Each segment is driven by 1 dedicated output GPIO; each cathode is driven by 8 dedicated output GPIOs to distribute the load
  * Display is PWM'd to avoid frying the device
  * Overall refresh rate must exceed visual detection, >50Hz
    * Can drive 7 segments at once, but only 1 digit at a time due to common cathode
    * Drive segments for 2ms on, 5ms off, for an overall refresh of 2*(2+5)=14ms, or 71.4Hz
      * More brightness can be achieved by shifting some off-time into on-time, but don't make the overall loop time be >20ms to avoid flickering.

## Algorithm for Finding the Pulse

![Heart Rate 3 Algorithm Block Diagram](/doc/Heart%20Rate%20Alg%203.png)

* Background Noise
  * Low Pass Filter
* Sensor Motion Noise
  * DC Estimate and Crazy Filter
* Isolating the Peak Region
  * Above/Below State Machine and Asymmetric Filter
  * Above Circular Buffer
* Finding the Peak in the Peak Region
  * Peak detection in Above Circular Buffer
* Heart Rate
  * Different in time between consecutive peaks -> Heart Rate

## Rust + Embassy Specific Development Issues
* General IPC
  * Atomics to drive display update, since we don't care if we miss a change, we'll pick it up next refresh
  * Channel to pass data from ADC task to Processing task so we don't lose data
    * Use of Channel (queue) to buffer periodic slowdown in processing task
    * Sizing that buffer
    * Detecting overflow and handling (or not)
* Passing Parameters to Tasks
  * Parameter references must have `static` lifetime
  * Parameters can not be generics
  * But Embassy STM Peripherals are generics!
  * Peripheral Ownership Among Tasks
    * The `type` trick to passing generics into tasks
    * Needing to share or own Peripherals
      * Can't have two threads accessing a single peripheral
* Balancing Cooperative Multitasking
  * Yielding to Scheduler
  * Display driver is highest priority, since it protects the LEDs
  * ADC task is same or higher priority since it is needs to sample evenly
  * Processing task is lowest priority and can be considered "background.
    * Performs some slow operations like peak finding
    * Irony of background task frequent yielding slowing foreground task

# Algorithm Development Story

See [ALGORITHM.md](ALGORITHM.md)
