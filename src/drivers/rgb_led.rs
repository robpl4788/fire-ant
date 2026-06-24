use embedded_hal::pwm::SetDutyCycle;

pub struct RGBLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle,
    B: SetDutyCycle,
{
    red: R,
    green: G,
    blue: B,
    max_duty: u16,
    inverted: bool,
}

impl<R, G, B> RGBLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle,
    B: SetDutyCycle,
{
    /// Create a new RGB LED driver.
    ///
    /// `max_duty` is the maximum duty cycle of the PWM slice (e.g. 65535).
    /// `inverted = true` for common-anode LEDs.
    pub fn new(red: R, green: G, blue: B, max_duty: u16, inverted: bool) -> Self {
        Self {
            red,
            green,
            blue,
            max_duty,
            inverted,
        }
    }

    /// Set the LED color using 0 to 1 brightness values.
    pub fn set_rgb(&mut self, r: f32, g: f32, b: f32) {
        let r = self.scale(r);
        let g = self.scale(g);
        let b = self.scale(b);

        let _ = self.red.set_duty_cycle(r);
        let _ = self.green.set_duty_cycle(g);
        let _ = self.blue.set_duty_cycle(b);
    }

    /// Turn the LED off.
    pub fn off(&mut self) {
        if self.inverted {
            let _ = self.red.set_duty_cycle_fully_on();
            let _ = self.green.set_duty_cycle_fully_on();
            let _ = self.blue.set_duty_cycle_fully_on();
        } else {
            let _ = self.red.set_duty_cycle_fully_off();
            let _ = self.green.set_duty_cycle_fully_off();
            let _ = self.blue.set_duty_cycle_fully_off();
        }
    }

    /// Set the LED Red.
    pub fn red(&mut self) {
        if self.inverted {
            let _ = self.red.set_duty_cycle_fully_off();
            let _ = self.green.set_duty_cycle_fully_on();
            let _ = self.blue.set_duty_cycle_fully_on();
        } else {
            let _ = self.red.set_duty_cycle_fully_on();
            let _ = self.green.set_duty_cycle_fully_off();
            let _ = self.blue.set_duty_cycle_fully_off();
        }
    }

    /// Set the LED Green.
    pub fn green(&mut self) {
        if self.inverted {
            let _ = self.red.set_duty_cycle_fully_on();
            let _ = self.green.set_duty_cycle_fully_off();
            let _ = self.blue.set_duty_cycle_fully_on();
        } else {
            let _ = self.red.set_duty_cycle_fully_off();
            let _ = self.green.set_duty_cycle_fully_on();
            let _ = self.blue.set_duty_cycle_fully_off();
        }
    }

    /// Set the LED Blue.
    pub fn blue(&mut self) {
        if self.inverted {
            let _ = self.red.set_duty_cycle_fully_on();
            let _ = self.green.set_duty_cycle_fully_on();
            let _ = self.blue.set_duty_cycle_fully_off();
        } else {
            let _ = self.red.set_duty_cycle_fully_off();
            let _ = self.green.set_duty_cycle_fully_off();
            let _ = self.blue.set_duty_cycle_fully_on();
        }
    }

    /// Convert 0 to 1 brightness to PWM duty.
    fn scale(&self, value: f32) -> u16 {
        let value = value.clamp(0.0, 1.0);

        let duty = (value * self.max_duty as f32) as u16;
        if self.inverted {
            self.max_duty - duty
        } else {
            duty
        }
    }
}
