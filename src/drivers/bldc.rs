use embedded_hal::pwm::SetDutyCycle;

use crate::utils::SetDutyCycleExtras;

/// Represents a motor phase with low and high-side MOSFET drivers
struct Phase<Low, High>
where
    Low: SetDutyCycle,
    High: SetDutyCycle,
{
    low: Low,
    high: High,
}

impl<Low, High> Phase<Low, High>
where
    Low: SetDutyCycle,
    High: SetDutyCycle,
{
    fn new(low: Low, high: High) -> Self {
        Phase { low, high }
    }

    fn disable(&mut self) {
        let _ = self.low.set_duty_cycle_fully_off();
        let _ = self.high.set_duty_cycle_fully_off();
    }

    fn set_low(&mut self) {
        let _ = self.high.set_duty_cycle_fully_off();
        let _ = self.low.set_duty_cycle_fully_on();
    }

    fn set_high(&mut self, power: f32) {
        // Prevent full power to avoid draining bootstrap capacitor too quickly
        let power = power.clamp(0., 0.9);
        let _ = self.low.set_duty_cycle_fully_off();
        let _ = self.high.set_duty_normalised(power);
    }
}

/// 6-phase commutation state for 3-phase BLDC motor
#[derive(Clone, Copy, Debug)]
enum CommutationState {
    /// Phase A high, Phase B low
    State0,
    /// Phase A high, Phase C low
    State1,
    /// Phase B high, Phase C low
    State2,
    /// Phase B high, Phase A low
    State3,
    /// Phase C high, Phase A low
    State4,
    /// Phase C high, Phase B low
    State5,
}

impl CommutationState {
    /// Get the next commutation state
    fn next(self) -> Self {
        match self {
            CommutationState::State0 => CommutationState::State1,
            CommutationState::State1 => CommutationState::State2,
            CommutationState::State2 => CommutationState::State3,
            CommutationState::State3 => CommutationState::State4,
            CommutationState::State4 => CommutationState::State5,
            CommutationState::State5 => CommutationState::State0,
        }
    }
}

/// BLDC motor controller with 6-phase commutation
pub struct BLDC<ALow, BLow, CLow, AHigh, BHigh, CHigh>
where
    ALow: SetDutyCycle,
    BLow: SetDutyCycle,
    CLow: SetDutyCycle,
    AHigh: SetDutyCycle,
    BHigh: SetDutyCycle,
    CHigh: SetDutyCycle,
{
    a_phase: Phase<ALow, AHigh>,
    b_phase: Phase<BLow, BHigh>,
    c_phase: Phase<CLow, CHigh>,
    state: CommutationState,
    power: f32,
}

impl<ALow, BLow, CLow, AHigh, BHigh, CHigh> BLDC<ALow, BLow, CLow, AHigh, BHigh, CHigh>
where
    ALow: SetDutyCycle,
    BLow: SetDutyCycle,
    CLow: SetDutyCycle,
    AHigh: SetDutyCycle,
    BHigh: SetDutyCycle,
    CHigh: SetDutyCycle,
{
    pub fn new(
        a_low: ALow,
        b_low: BLow,
        c_low: CLow,
        a_high: AHigh,
        b_high: BHigh,
        c_high: CHigh,
    ) -> Self {
        let mut bldc = Self {
            a_phase: Phase::new(a_low, a_high),
            b_phase: Phase::new(b_low, b_high),
            c_phase: Phase::new(c_low, c_high),
            state: CommutationState::State0,
            power: 0.0,
        };

        bldc.disable();
        bldc
    }

    pub fn disable(&mut self) {
        self.a_phase.disable();
        self.b_phase.disable();
        self.c_phase.disable();
    }

    /// Advance to the next commutation state
    pub fn progress(&mut self, power: f32) {
        self.state = self.state.next();
        self.set_power(power);
    }

    /// Apply commutation pattern for the current state
    fn apply_commutation(&mut self, power: f32) {
        match self.state {
            CommutationState::State0 => {
                self.a_phase.set_high(power);
                self.b_phase.set_low();
                self.c_phase.disable();
            }
            CommutationState::State1 => {
                self.a_phase.set_high(power);
                self.b_phase.disable();
                self.c_phase.set_low();
            }
            CommutationState::State2 => {
                self.a_phase.disable();
                self.b_phase.set_high(power);
                self.c_phase.set_low();
            }
            CommutationState::State3 => {
                self.a_phase.set_low();
                self.b_phase.set_high(power);
                self.c_phase.disable();
            }
            CommutationState::State4 => {
                self.a_phase.set_low();
                self.b_phase.disable();
                self.c_phase.set_high(power);
            }
            CommutationState::State5 => {
                self.a_phase.disable();
                self.b_phase.set_low();
                self.c_phase.set_high(power);
            }
        }
    }

    /// Set motor power (0.0 to 1.0)
    pub fn set_power(&mut self, power: f32) {
        let power = power.clamp(0., 0.9);
        self.apply_commutation(power);
    }
}
