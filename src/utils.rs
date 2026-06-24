use embedded_hal::pwm::SetDutyCycle;

pub trait SetDutyCycleExtras: SetDutyCycle {
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
