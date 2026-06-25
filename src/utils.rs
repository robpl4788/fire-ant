use embedded_hal::pwm::SetDutyCycle;

/// Extension trait for PWM duty cycle control with normalized values
/// Provides convenience methods for setting duty cycle as a normalized 0.0-1.0 value
pub trait SetDutyCycleExtras: SetDutyCycle {
    /// Set the PWM duty cycle using a normalized value (0.0 to 1.0)
    /// 
    /// # Arguments
    /// * `value` - Normalized duty cycle (0.0 = fully off, 1.0 = fully on)
    ///             Values outside this range are clamped
    fn set_duty_normalised(&mut self, value: f32) -> Result<(), Self::Error>;
}

impl<T> SetDutyCycleExtras for T
where
    T: SetDutyCycle,
{
    fn set_duty_normalised(&mut self, value: f32) -> Result<(), Self::Error> {
        let value = value.clamp(0., 1.);
        self.set_duty_cycle((value * self.max_duty_cycle() as f32) as u16)
    }
}
