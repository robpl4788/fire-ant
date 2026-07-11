use defmt::Format;
use defmt::println;
use embassy_time::Instant;
use embedded_hal::pwm::SetDutyCycle;

use crate::drivers::bldc;
use crate::{
    drivers::bldc::PhaseState::{DISABLED, HIGH, LOW},
    utils::SetDutyCycleExtras,
};

#[derive(PartialEq)]
enum PhaseState {
    HIGH,
    LOW,
    DISABLED,
}

/// Represents a motor phase with low and high-side MOSFET drivers
pub struct Phase<Low, High>
where
    Low: SetDutyCycle,
    High: SetDutyCycle,
{
    low: Low,
    high: High,
    phase_state: PhaseState,
}

impl<Low, High> Phase<Low, High>
where
    Low: SetDutyCycle,
    High: SetDutyCycle,
{
    pub fn new(low: Low, high: High) -> Self {
        let mut new_phase = Phase {
            low,
            high,
            phase_state: DISABLED,
        };

        new_phase.disable();

        new_phase
    }

    fn disable(&mut self) {
        let _ = self.low.set_duty_cycle_fully_on(); // Low is inverted
        let _ = self.high.set_duty_cycle_fully_off();
        self.phase_state = DISABLED;
    }

    fn set_low(&mut self) {
        let _ = self.high.set_duty_normalised(0.0); // Low is inverted
        let _ = self.low.set_duty_normalised(0.0);
        self.phase_state = LOW;
    }

    fn set_high(&mut self, power: f32) {
        // Prevent full power to avoid draining bootstrap capacitor too quickly
        let power = power.clamp(0., 0.9);
        let _ = self.low.set_duty_normalised(power);
        let _ = self.high.set_duty_normalised(power);
        self.phase_state = HIGH;
    }
}

/// 6-phase commutation state for 3-phase BLDC motor
#[derive(Clone, Copy, Debug, PartialEq, Format)]
pub enum CommutationState {
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
    kv: u16,
    vbat: f32,
    commutations_per_rotation: u8,
    last_update_micros: u64,
    last_commutation_micros: u64,
    current_commutation_delay_micros: u64,
    scheduled_commutation_micros: u64,
    bemf_a_sample_valid: bool,
    bemf_a_was_above_common: bool,
    max_rps: u16,
    target_rps: i16,
    prev_v_a: f32,
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
        a_phase: Phase<ALow, AHigh>,
        b_phase: Phase<BLow, BHigh>,
        c_phase: Phase<CLow, CHigh>,
    ) -> Self {
        let mut bldc = Self {
            a_phase,
            b_phase,
            c_phase,
            state: CommutationState::State0,
            power: 0.0,
            kv: 1750,
            vbat: 7.4,
            commutations_per_rotation: 42,
            last_update_micros: 0,
            last_commutation_micros: 0,
            current_commutation_delay_micros: 0,
            scheduled_commutation_micros: 0,
            bemf_a_sample_valid: false,
            bemf_a_was_above_common: false,
            target_rps: 0,
            max_rps: 200,
            prev_v_a: 0.,
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
    pub async fn progress(&mut self, power: f32) {
        self.state = self.state.next();
        self.scheduled_commutation_micros = 0;
        self.bemf_a_sample_valid = false;
        self.set_power(power);
        // Timer::after_micros(50).await;
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

    pub fn get_last_commutation(&self) -> u64 {
        self.last_commutation_micros
    }
    // Run at 20khz?
    pub async fn open_loop(&mut self, micros: u64) -> bool {
        // self.disable();
        // let dt = micros - self.last_update_micros;

        let commutations_per_second: u64 =
            self.commutations_per_rotation as u64 * self.target_rps as u64;
        let commutation_delay_micros: u64;
        if commutations_per_second != 0 {
            commutation_delay_micros = 1_000_000 / commutations_per_second;
            self.current_commutation_delay_micros = commutation_delay_micros;
            let next_commutation = self.last_commutation_micros + commutation_delay_micros;
            // println!(
            //     "next_commutation: {}, micros: {}, commutation_delay_micros: {},",
            //     next_commutation, micros, commutation_delay_micros
            // );
            if next_commutation < micros {
                // println!("micros: {}", &micros);
                self.last_commutation_micros += commutation_delay_micros;

                // Don't let it fall too far behind
                if self.last_commutation_micros < micros {
                    self.last_commutation_micros = micros;
                }
                // self.progress(self.target_rps as f32 / self.max_rps as f32);
                self.progress(0.4).await;
                // println!("commutate");
            }
        }

        self.last_update_micros = micros;

        self.state == CommutationState::State0
    }

    pub fn get_state(&self) -> CommutationState {
        self.state
    }

    pub async fn closed_loop(&mut self, v_a: f32, v_common: f32) {
        let now = Instant::now().as_micros();

        if self.state == CommutationState::State2 {
            // self.set_power(1.);
            let time_since_commutation = now.saturating_sub(self.last_commutation_micros);
            let expected_commutation_delay = self.current_commutation_delay_micros.max(1);

            if time_since_commutation > 60 {
                if v_a < self.prev_v_a {
                    self.progress(0.4).await;
                    self.current_commutation_delay_micros = now - self.last_commutation_micros;
                    println!("{}", &self.current_commutation_delay_micros);
                    self.last_commutation_micros = now;
                    self.prev_v_a = 0.;
                }
            }

            self.prev_v_a = v_a;

            // if time_since_commutation >= expected_commutation_delay {
            //     println!("should commutate");
            //     self.last_commutation_micros = now;
            // }

            let above_common = v_a > v_common;

            // println!("{}", &above_common);
        } else {
            self.open_loop(now).await;
        }
        // else if self.state == CommutationState::State5 {
        //     if v_a < v_common {
        //         self.progress(0.4).await;
        //         let now = Instant::now().as_micros();

        //         self.current_commutation_delay_micros = now - self.last_commutation_micros;
        //         self.last_commutation_micros = now;
        //     }
        // }
        // else {
        //     if self.last_commutation_micros + self.current_commutation_delay_micros
        //         < Instant::now().as_micros()
        //     {
        //         self.progress(0.4).await;
        //         self.last_commutation_micros = Instant::now().as_micros();
        //     }
        // }
    }

    pub fn set_target_rps(&mut self, new_target: i16) {
        self.target_rps = new_target;
    }
}
