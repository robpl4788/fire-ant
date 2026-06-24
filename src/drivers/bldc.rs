use embedded_hal::pwm::SetDutyCycle;

use crate::utils::SetDutyCycleExtras;

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
    pub fn new(low: Low, high: High) -> Self {
        Phase { low, high }
    }

    pub fn disable(&mut self) {
        self.low.set_duty_cycle_fully_off();
        self.high.set_duty_cycle_fully_off();
    }

    pub fn set_low(&mut self, power: f32) {
        self.high.set_duty_cycle_fully_off();
        self.low.set_duty_normalised(power);
    }

    pub fn set_high(&mut self) {
        self.low.set_duty_cycle_fully_off();
        self.high.set_duty_cycle_fully_on();
    }
}
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

    phase_state: u8,
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
        Self {
            a_phase: Phase::new(a_low, a_high),
            b_phase: Phase::new(b_low, b_high),
            c_phase: Phase::new(c_low, c_high),

            phase_state: 0,
        }
    }

    pub fn disable(&mut self) {
        self.a_phase.disable();
        self.b_phase.disable();
        self.c_phase.disable();
    }

    pub fn set_state(&mut self, state: u8, power: f32) {
        let power = power.clamp(0., 1.);

        self.phase_state = state;
        match state {
            0 => {
                self.a_phase.set_high();
                self.b_phase.set_low(power);
                self.c_phase.disable();
            }
            1 => {
                self.a_phase.set_high();
                self.b_phase.disable();
                self.c_phase.set_low(power);
            }
            2 => {
                self.a_phase.disable();
                self.b_phase.set_high();
                self.c_phase.set_low(power);
            }
            3 => {
                self.a_phase.set_low(power);
                self.b_phase.set_high();
                self.c_phase.disable();
            }
            4 => {
                self.a_phase.set_low(power);
                self.b_phase.disable();
                self.c_phase.set_high();
            }
            5 => {
                self.a_phase.disable();
                self.b_phase.set_low(power);
                self.c_phase.set_high();
            }
            _ => panic!(),
        }
    }
}
