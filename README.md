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

# Stuff to write about

* Project Goals
  * Explore cooperative async/await as an alternative to time slice multitasking
  * Drive an LED display without driver transistors or resistors
  * Explore heartrate calculations needed in [photoplethysmography](https://en.wikipedia.org/wiki/Photoplethysmogram)
* HW Architecture + BOM
  * Goals
    * All off-the-shelf hardware with exception of bare 7-segment LED display
  * BOM
    * RaspberryPi/Arduino-style 3.3V analog 3-pin pulse heart rate sensor with integrated low noise signal amp, $4 (Amazon)
For a stand-alone CubeMX data acquisition
    * [NUCLEO-L073RZ](https://www.mouser.com/ProductDetail/STMicroelectronics/NUCLEO-L073RZ) dev kit with USB power and built in FTDI and ST-Link, $14 (mouser)
For a well supported Rust embedded platform:
    * [NUCLEO-H743ZI2](https://www.mouser.com/ProductDetail/511-NUCLEO-H743ZI2) dev kit with USB power and built in FTDI and ST-Link, $27 (mouser)
    * Surplus 7-segment Common Cathode LED display

![HR FW Task Diagram](/doc/HR%20FW%20Architecture.png)

* FW Architecture
  * Cooperative multitasking with quasi-real-time requirement in sampling and display tasks
    * ADC sampling task ticks at 1kHz. Ideally a very precise 1kHz for signal processing reasons
    * Display task needs to tick overall at >50Hz to avoid flicker. Variations of or off periods will appear as visual glitches or brighter or darker digits
    * HR task takes up the background processing slack, but at this time, only the UART I/O and sample channel operate async. Ideally, the processing would also have scheduler yields embedded in it, but without compiler optimization, they noticeably degrade the performance of the display task, so they were removed.  Something to revisit and explain!
  * Reading the ADC
    * Simply sample ADC at a regular rate and place output in Channel queue for processing later
    * ADC configured to minimize noise
      * 16x oversampling enabled (with `unsafe` raw point: FIXME, use `stm-metapac`) with no down-shift on 12 bit conversion to result in 16 bit samples
      * Changes to ADC Sample/Hold did not seem to have any effect on noise, so lowered ADC clock rate instead and found sweet spot at about 1/10th the default sample rate
      * ADC sampling takes about 50us in this mode, far less than 1kHz overall sample rate.
    * Size of channel determined empirically by watching amount of overruns with various processing and I/O loads during algorithm development
  * Task for processing samples
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

![HeartRate 3 Algorithm Block Diagram](/doc/Heart%20Rate%20Alg%203.png)

* Algorithm for Finding the Pulse
  * Background Noise
    * Low Pass Filter
  * Sensor Motion Noise
    * DC Estimate and Crazy Filter
  * Isolating the Peak Region
    * Above/Below State Machine and Asymmetric Filter
  * Finding the Peak in the Peak Region
  * Heart Rate

* Development Story
  * First Steps
    * Original goal was measuring breathing to support mindfulness
    * Settled on less ambitious goal of detecting pulse from photo sensor
    * Got a cheap sensor off Amazon which had Arduino example code
    * Got a quick data acquisition loop running at 50Hz using ST CubeMX
  * Look at the Data
    * Sampling Rate
  * Confounding Problems
  * More on Data Acquisition
  
* Rust + Embassy Specific Development Issues
  * Balancing Cooperative Multitasking
    * Yielding to Scheduler
    * Irony of background task yielding slowing foreground task
  * Peripheral Ownership Amongst Tasks

# Development Story
## First data acquired at 50Hz

![graphic0a.png](/doc/graphic0a.png)

### Zoom out to show context: 
* DC drift
* Sensor Motion Noise
* Background Noise
* Amplitude changes

![graphic0b.png](/doc/graphic0b.png)

### Zoom out further to show relative size of sensor motion artifacts

![graphic0c.png](/doc/graphic0c.png)

## Comparison of background noise and wave shape when scale is normalized

![graphic0d.png](/doc/graphic0d.png)

### Zoom up of a single pulse wave

![graphic0e.png](/doc/graphic0e.png)

## Noisier data from default H743 ADC driver under Rust

![gallery-over.png](/doc/gallery-over.png)

## Same data with Low Pass

![gallery-over-lowpass.png](/doc/gallery-over-lowpass.png)



![graphic1.png](/doc/graphic1.png)
![graphic3.png](/doc/graphic3.png)
![graphic3a.png](/doc/graphic3a.png)
![graphic3aa.png](/doc/graphic3aa.png)
![graphic4.png](/doc/graphic4.png)
![graphic5a-hairplots.png](/doc/graphic5a-hairplots.png)
![graphic6.png](/doc/graphic6.png)
