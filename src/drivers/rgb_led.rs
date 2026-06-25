use crate::utils::SetDutyCycleExtras;
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
}

#[allow(dead_code)]
impl<R, G, B> RGBLed<R, G, B>
where
    R: SetDutyCycle,
    G: SetDutyCycle,
    B: SetDutyCycle,
{
    /// Create a new RGB LED driver.
    pub fn new(red: R, green: G, blue: B) -> Self {
        let mut rgb_led = Self { red, green, blue };

        rgb_led.off();

        rgb_led
    }

    /// Set the LED color using 0 to 1 brightness values.
    pub fn set_rgb(&mut self, r: f32, g: f32, b: f32) {
        let r = 1. - r;
        let g = 1. - g;
        let b = 1. - b;

        let _ = self.red.set_duty_normalised(r);
        let _ = self.green.set_duty_normalised(g);
        let _ = self.blue.set_duty_normalised(b);
    }

    /// Turn the LED off.
    pub fn off(&mut self) {
        let _ = self.red.set_duty_cycle_fully_on();
        let _ = self.green.set_duty_cycle_fully_on();
        let _ = self.blue.set_duty_cycle_fully_on();
    }

    /// Set the LED Red.
    pub fn red(&mut self) {
        let _ = self.red.set_duty_cycle_fully_off();
        let _ = self.green.set_duty_cycle_fully_on();
        let _ = self.blue.set_duty_cycle_fully_on();
    }

    /// Set the LED Green.
    pub fn green(&mut self) {
        let _ = self.red.set_duty_cycle_fully_on();
        let _ = self.green.set_duty_cycle_fully_off();
        let _ = self.blue.set_duty_cycle_fully_on();
    }

    /// Set the LED Blue.
    pub fn blue(&mut self) {
        let _ = self.red.set_duty_cycle_fully_on();
        let _ = self.green.set_duty_cycle_fully_on();
        let _ = self.blue.set_duty_cycle_fully_off();
    }
}
