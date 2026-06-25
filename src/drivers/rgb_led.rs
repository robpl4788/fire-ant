use crate::utils::SetDutyCycleExtras;
use embedded_hal::pwm::SetDutyCycle;

/// RGB LED controller for PWM-driven RGB LEDs
pub struct RGBLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle,
    B: SetDutyCycle,
{
    red: R,
    green: G,
    blue: B,
}

#[allow(dead_code)]
impl<R, G, B> RGBLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle,
    B: SetDutyCycle,
{
    /// Create a new RGB LED driver (initialized to off state)
    pub fn new(red: R, green: G, blue: B) -> Self {
        let mut rgb_led = Self { red, green, blue };
        rgb_led.off();
        rgb_led
    }

    /// Set the LED color using normalized brightness values (0.0 to 1.0)
    /// Note: Values are inverted because LED is active-low (common anode)
    pub fn set_rgb(&mut self, r: f32, g: f32, b: f32) {
        // Invert because common-anode LED: 1.0 = off, 0.0 = full brightness
        let _ = self.red.set_duty_normalised(1. - r);
        let _ = self.green.set_duty_normalised(1. - g);
        let _ = self.blue.set_duty_normalised(1. - b);
    }

    /// Turn the LED off (all channels to full brightness)
    pub fn off(&mut self) {
        let _ = self.red.set_duty_cycle_fully_on();
        let _ = self.green.set_duty_cycle_fully_on();
        let _ = self.blue.set_duty_cycle_fully_on();
    }

    /// Set the LED to red
    pub fn red(&mut self) {
        self.set_rgb(1.0, 0.0, 0.0);
    }

    /// Set the LED to green
    pub fn green(&mut self) {
        self.set_rgb(0.0, 1.0, 0.0);
    }

    /// Set the LED to blue
    pub fn blue(&mut self) {
        self.set_rgb(0.0, 0.0, 1.0);
    }

    /// Set the LED to yellow (red + green)
    pub fn yellow(&mut self) {
        self.set_rgb(1.0, 1.0, 0.0);
    }

    /// Set the LED to cyan (green + blue)
    pub fn cyan(&mut self) {
        self.set_rgb(0.0, 1.0, 1.0);
    }

    /// Set the LED to magenta (red + blue)
    pub fn magenta(&mut self) {
        self.set_rgb(1.0, 0.0, 1.0);
    }

    /// Set the LED to white (all channels on)
    pub fn white(&mut self) {
        self.set_rgb(1.0, 1.0, 1.0);
    }
}
