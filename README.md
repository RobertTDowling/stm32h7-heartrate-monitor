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
* FW Architecture
  * Cooperative multitasking with quasi-real-time requirement in display task

![HR FW Task Diagram](/doc/HR%20FW%20Architecture.png)

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
* Algorithm for Finding the Pulse
  * Background Noise
    * Low Pass Filter
  * Sensor Motion Noise
    * DC Estimate and Crazy Filter
  * Isolating the Peak Region
    * Above/Below State Machine and Asymmetric Filter
  * Finding the Peak in the Peak Region
  * Heart Rate
* Rust + Embassy Specific Development Issues
  * Balancing Cooperative Multitasking


![HeartRate 3 Algorithm Block Diagram](/doc/Heart%20Rate%20Alg%203.png)


