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
* Finding the Peak in the Peak Region
* Heart Rate

## Development Story
* First Steps
  * Original goal was measuring breathing to support mindfulness
  * Settled on less ambitious goal of detecting pulse from photo sensor
  * Got a cheap sensor off Amazon which had Arduino example code
  * Got a quick data acquisition loop running at 50Hz using ST CubeMX
* Look at the Data
  * Sampling Rate
* Confounding Problems
* More on Data Acquisition

## Rust + Embassy Specific Development Issues
* Balancing Cooperative Multitasking
  * Yielding to Scheduler
  * Irony of background task yielding slowing foreground task
* Peripheral Ownership Among Tasks

# Development Story

## Look at the Data

It is always a good idea to look at your data before you make any decisions. I remember a [Fields Medal](https://www.google.com/search?q=fields+medal) [recipient](https://math.berkeley.edu/~reb/) trying to impress upon me that one should do calculations by hand the first few times to get the flavor. I pass that on in case you need to hear the voice of authority.

One of the sensor vendors posted a short Arduino sketch to exercise the device. It was less than a screen of code. It read the ADC at 20ms intervals, compared this sample to the last, looking for a large positive change to “detected a peak”. It computed a heart rate from the time between peaks.

I wrote a very cheap loop in C using [ST’s CubeMX](https://www.st.com/en/development-tools/stm32cubemx.html) generated HAL to read the ADC at 50Hz or 1kHz and printf the raw value on the UART. Most Nucleo boards conveniently provide a built-in FTDI, so from the PC, I could log the data using PuTTY and start looking at it in Octave.

> [!NOTE]
> Manual captures were made with [PuTTY](https://www.chiark.greenend.org.uk/~sgtatham/putty/latest.html) and analyzed in [Octave](https://octave.org/) (Matlab)

The data didn’t look bad--once I got a signal--but it took a lot of fussing to actually get a signal. The biggest problem was that if you didn't hold the sensor just right, it picked up nothing.

* Lightly press the lit side of the sensor to the center of the pad of your thumb
* Your thumb must be warm or there won’t be enough circulation to detect a signal
* The pressure must be light, or the pressure will cause the top surface of the skin to drain away blood, and there won’t be a signal
* The pressure must be heavy enough to hold the sensor against the skin and seal out ambient light, and also press the skin against the photodiode so that there is no direct path from the light source to the photodiode
* You can’t change pressure. Changing pressure will drown out any pulse signal
* You can’t move. Movement signals will drown out any pulse signal

I’ve been doing it for weeks now, and it is still hard to get and maintain a signal without visual feedback. I found that when I can’t get a reading from either thumb, the lips work well.

Here is a zoomed up capture of data from early in the project. The data is sampled at 50Hz like the Arduino sketch. Notice the vertical scale: Those “big” peaks are only 600 counts tall out of a total range of 65000. It’s like 1% of full scale.


### First data acquired at 50Hz

![graphic0a.png](/doc/graphic0a.png)

The peaks are pronounced, but there is a lot of what appears to be 10Hz noise, even on the peaks. Thinking about that, it seemed likely it was really 60Hz noise, which is everywhere in my house, [aliasing](https://en.wikipedia.org/wiki/Aliasing) down to 10Hz due to sampling at 50Hz without an [anti-aliasing](https://en.wikipedia.org/wiki/Anti-aliasing_filter) filter.

I took some samples at 1kHz, and sure enough, there is a lot of 60Hz. I wasn’t going to add a filter to the hardware, so I went with sampling at 1kHz, figuring I can remove the 60Hz in software.

Zooming out on a capture similar to the one above we can see more problems.

#### Zoom out to show context:
* DC drift
* Sensor Motion Noise
* Background Noise
* Amplitude changes

![graphic0c.png](/doc/graphic0c.png)


#### Zoom out further to show relative size of sensor motion artifacts

![graphic0b.png](/doc/graphic0b.png)

### Comparison of background noise and wave shape when scale is normalized

Here are 6 different captures of a few pulses, shown at slightly different scales so that they appear the same size.

* Notice some are relatively clean, and others noisy
* Some have large ringing artifacts, and others much less so

![graphic0d.png](/doc/graphic0d.png)

#### Zoom up of a single pulse wave

Here is a zoom up on a single pulse, lasting just over 1200 ms. This signal is actually pretty quiet.

![graphic0e.png](/doc/graphic0e.png)

## Summary of Confounding Problems

* Very sensitive sensor placement requirements
* In general, a small signal
  * A strong signal is about 1/50th of full scale on the ADC
* (Electronic) Background Noise
  * 60Hz hum and other coupled noise
* Finger Noise resulting from motion of the sensor
  * Small movements can look like a pulse
  * Large movements drive the sensor into clipping and it takes a few seconds to recover
* Ringing after main pulse appearing like a second pulse
* Changing signal amplitude and unpredictable signal amplitude
* Changing signal baseline
  * Due to AC coupling, especially after finger motion

## More on Data Acquisition

* RaspberryPi pulse heart rate sensor hooked up to an ADC input.

The sensor is really small, and it has a very green LED light. It has 3 leads: Gnd, 3.3V, and Signal. It didn’t come with any more documentation. Presumably the sensor has a high-gain amplifier that is AC coupled with an approximately 5 second time constant. If everything is normal, the sensor drifts back to center of the ADC range after this much time.

The Nucleo-L073 can be configured to read a built-in 12 bit ADC in 16x oversample mode and still achieve 1MHz sampling rate or more. The oversampling is further configured to not down-shift the data, so the ADC effectively reads 16 bit samples. It takes less than a microsecond to read the ADC in this mode, so the sample-and-hold is left open as long as possible to reduce noise. Using a timer, the ADC is read once a millisecond.

Data is sampled at 1kHz as explained above. There is nothing magical about that value, it is just relatively slow, yet high enough that we can be fairly confident we can see the things we want to see–even the things we don’t want to see (60Hz)–without flooding ourselves with data. It also makes the numbers work out nicely because we can work with 1 sample = 1 millisecond.

Printf(“%d”) ASCII data is sent out the UART3 serial port at 115200 baud. At this rate, a 16-bit decimal number of 5 digits plus a newline takes a little over half the UART bandwidth to transmit. The Nucleo conveniently provides a USB FTDI device to a PC, which can run a terminal program to capture the output.

At least for collecting data, nothing more was needed.

Manual captures were made and analyzed in the Matlab clone [Octave](https://octave.org/). Once that manual process got too frustrating, a Python app using [pyserial](https://pypi.org/project/pyserial/) and matplotlib, created a real-time “oscilloscope” display of the data.

Analysis was done on data files in Octave in batch mode, that is, with the liberty to see all the data at once and loop over it as many times as I liked. The vector-based Matlab algorithms were then ported to real-time streaming versions to run in Python and added to the oscilloscope app. The same algorithms were ported to Rust, first on the PC to process file data, then UART data, and finally ported to the hardware using Embassy to make a stand-alone heart rate monitor


## The Essential Problem of Detecting Pulse

The Essential Problem is to find the pulse wave peaks and use the distance
between them to determine the heart rate. In the plot below, that would be the
horizontal distance between the magenta squares.

![graphic1.png](/doc/graphic1.png)

When the signal is this clean, you think it would be easy to locate the
peaks. It wasn’t.

The first challenge was to determine the part of the wave above the baseline
(green dashed line) that contains just the tall pulse peak, and then find the
maximum (magenta square) within that section.

## Background Noise

![graphic3.png](/doc/graphic3.png)

### Noisier data from default H743 ADC driver under Rust

Migrating over to the larger H7 nucleo board, the electrical noise
skyrocketed.

![gallery-over.png](/doc/gallery-over.png)

A low-pass filter is a simple fist step to removing the background noise that would otherwise make peak finding impossible.

> [!TIP]
> All the low-pass filters used in this project are based on the [Exponential Moving Average](https://en.wikipedia.org/wiki/Exponential_smoothing) (EMA)
> which is very easy to compute and more or less intuitive. The EMA is sort of a
> degenerate IIR filter that models an [RC charge/discharge curve](https://en.wikipedia.org/wiki/RC_filter), with an exponential decay set
> by a single parameter α.
>
> For $0<\alpha<1$, small:
>
> $$y_n=(1-\alpha)y_{n-1} + x_n$$
>
> This can be coded up
>
> ```y = (1-a)*y + a*x```
>
> or
>
> ```y += a*(x-y)```
>
> ### EMA Factoids
>
> $\textrm{Let }T=1/\alpha$
>
> * After T samples, the output will have converged 63.2% (or $1-e^{-1}$) to the input
> * Rule of thumb: 5T gets you within ½% of the final value, since exp(-5) = 0.0067
> * Any given input sample will “spend” T samples worth of time in the filter when all the fractions multiplied by their age are added up
> * The cutoff frequency (FC) of the EMA filter is (for small $\alpha$)
>   * FC=radians/sample
>   * FC=SampleRate/2 Hz
>   * As a period, this is 2T samples
> * A sine wave at the cutoff frequency will be attenuated to half power, which is $\sqrt{1/2}= 0.707$ amplitude

### Same data with Low Pass

Empirically, an EMA with α=1/100 (FC=1.6Hz) when sampling at 1kHz does a good
job of rejecting 50/60Hz and other electrical noise while leaving the
asymmetric shape of the ~1Hz heart pulse intact. There is some amplitude loss,
but not a whole lot, and now the peaks are fairly smooth.

![gallery-over-lowpass.png](/doc/gallery-over-lowpass.png)

## Sensor Motion Artifacts

Because sensor motion creates such large excursions compared to the actual pulse signal, a pair of thresholds can be set around the current signal baseline to discard samples that are “crazy”. A current baseline is found by low-passing the original raw signal by a very low cutoff, α=1/1000.

If the time constant is made much longer, it won’t track the normal drifting and turn-on ramp we see. If the time constant is made much smaller, it will track the pulse too closely and won’t provide an adequately stable estimate of the signal baseline.

![graphic3aa.png](/doc/graphic3aa.png)

Zooming out on the same graph, we can see that finger motion can throw the baseline estimate off for quite a while (particularly at 45 seconds below).

While it would be nice to throw out crazy samples before computing the baseline, we really can’t without more information. For instance, we could factory-calibrate the expected baseline (and that is what I did initially) but it would be different for each device and would not allow for aging or drift after manufacturing. And we can’t safely use the estimated baseline itself to decide when to update the estimated baseline, because if anything went wrong and the estimated baseline got too far away from the real baseline, it would stop accepting new samples and never get unstuck.

So we have to live with our system going out of kilter for a few seconds when the user moves their hand.

With a reasonably good estimate of the baseline, we can define some thresholds based on a large number of samples. Looking at a few hours of pulse data from a couple different users, I established the high level at baseline+3000, and the low level-1000 for a 16 bit ADC. (FIXME: more can be done here to make it less ad-hoc.)

![graphic3a.png](/doc/graphic3a.png)

## Finding the cutting line to isolate the pulse peak region

I really struggled with a robust way to segment the large peak from the rest of the pulse signal, a way that could withstand offset and gain drifts, and be immune from noise. The noise issue is virtually eliminated by the α=1/100 low pass filter, but during early algorithm development, when I tackled segmentation, I was using unfiltered data.

Actually, the baseline estimate curve from the previous section comes tantalizingly close to cutting the peak in the right place, and ultimately something similar was used. But when noise levels got too high, it became too easy for stray wiggles to appear to be the start of another pulse.

I considered many approaches
Adding a small offset to the estimated baseline, but it would not work with a very wide range of signal/noise ratios.
Looking at the slope of the data (like the Arduino sketch), but simply comparing neighboring samples would not be robust in the face of noise.
Computing a local maximum and local minimum, say of the last 2 seconds of data, along with a fixed ratio like 1/3rd of the way between the minimum and maximum to set a threshold. But it seemed like a lot of data to hang onto, and a lot of maxes and mins to compute on every sample. While a modern MCU could easily handle the load, it didn’t seem elegant. And when the new Nucleo proved to have much more ADC noise, that fixed ratio between low and high would have been a problem.
Look a histogram of the last, say 2 seconds, of data, expecting a long tail on the right side, and put the threshold on the knee to the right of the mode. Way too much computation and no simple way to detect the distribution knee!
Bandpass filter the data to only accept the peak. This almost makes the problem worse as such a tight filter rings like crazy. Now have you 5 peaks for every pulse!

I finally landed on an asymmetric version of the EMA, where the α parameter varies based on whether the new data point is above or below the current moving average. (There is probably a name for this… if you know it, let me know!)

```
if x>y:
    y += a_above*(x-y)
else:
    y += a_below*(x-y)
```

Setting `a_above = a_below`, we get the normal EMA. Making a_above significantly larger (i.e, a shorter time constant, faster moving) than a_below, the filter skews high. It is quick to move up, but slow to come back down again. In practice, this works amazingly well, and is trivially more expensive to compute. No history, no sorting to find a histogram, no numerically unstable narrow band filter.

![graphic4.png](/doc/graphic4.png)

It is easy enough to consider the pulse as starting when
enough previous samples have all been below the asymmetric EMA, and
the next sample is above the asymmetric EMA

### For the first criteria, what is enough samples?

Based on ideas from the [Pan–Tompkins algorithm](https://en.wikipedia.org/wiki/Pan%E2%80%93Tompkins_algorithm) for detecting pulses in electrocardiogram waves, the main peak of the pulse is stated to have a minimum width of 150ms–the heart just can’t pulse again that quickly.

Mulling over the data I had, the pulse widths seem closer to 200ms. Although they look superficially similar, a photoplethysmogram is not an ECG. The waves are different, and the mechanism that generates them is different too. The ECG is a measure of the electrical activation of the heart muscle. The photoplethysmogram is a measure of the changing blood volume in an extremity, which is a result of the pressure wave from the heart beat, acting against the resistance of the arteries. They don’t seem comparable. I felt justified in making this change and modeled my pulse with as 200ms.

How do we determine when the pulse ends?

Because the asymmetric EMA cuts through the pulse wave higher on the right than on the left, it didn’t make sense to use the same technique to find the end of the pulse. It would cut off too much of the pulse, and we’d have less data to work with. If we decided to do curve fitting, for example, less data would mean a less accurate fit, or one more subject to noise. In fact, that downward crossing comes long before the expected 200ms width. I punted here and declared the pulse ended exactly 200ms after it starts. Depending on how late we start, we might pick up some non-peak data at the end.  The peak detector would have to deal with that.

## The Hair Plots

The hair plots show a collection of peak regions from real data. They show the 200ms of peak data in red, with an extra 50ms on either side in blue for context. When the peaks are all superimposed on each other, the 200ms data distributes roughly evenly on both sides of the peak. In some cases, horrible things happen, but mostly it works well. Changing the end point to 150ms after the start would probably make them all lopsided.

![graphic5a-hairplots.png](/doc/graphic5a-hairplots.png)

## Determining the peak within the region

![graphic6.png](/doc/graphic6.png)

Peak finding is an art. Octave uses this trick:

* Fit a quadratic polynomial and find its maximum

Experimentally the 200ms region does not fit a parabola well, and doing so almost always moves the peak dozens of samples to the right. I know people say not to fit to higher order polynomials, but a 3rd order fits better and a 5th order fit very well.

But the current implementation does not do any curve fitting. Since the data is low passed, it is fairly smooth, so it simply searches for the maximum. (FIXME: this probably should be changed, since the heart rate seems to fluctuate a fair amount currently, more than I think it does physiologically. Misplacing the peak will have a direct impact on the heart rate calculation.)
