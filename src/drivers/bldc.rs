use defmt::{info, println};
use embassy_rp::adc::Channel;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embedded_hal::pwm::SetDutyCycle;

use crate::{
    ADCMutex,
    drivers::{
        bldc::PhaseState::{DISABLED, HIGH, LOW},
        logger::LOGGER,
    },
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
    adc_mutex: &'static ADCMutex,
    adc_pin: Option<Channel<'static>>,
    phase_state: PhaseState,
}

impl<Low, High> Phase<Low, High>
where
    Low: SetDutyCycle,
    High: SetDutyCycle,
{
    pub fn new(
        low: Low,
        high: High,
        adc_mutex: &'static ADCMutex,
        adc_pin: Option<Channel<'static>>,
    ) -> Self {
        let mut new_phase = Phase {
            low,
            high,
            adc_mutex,
            adc_pin,
            phase_state: DISABLED,
        };

        new_phase.disable();

        new_phase
    }

    fn disable(&mut self) {
        let _ = self.low.set_duty_cycle_fully_off();
        let _ = self.high.set_duty_cycle_fully_off();
        self.phase_state = DISABLED;
    }

    fn set_low(&mut self) {
        let _ = self.high.set_duty_cycle_fully_off();
        let _ = self.low.set_duty_cycle_fully_on();
        self.phase_state = LOW;
    }

    fn set_high(&mut self, power: f32) {
        // Prevent full power to avoid draining bootstrap capacitor too quickly
        let power = power.clamp(0., 0.9);
        let _ = self.low.set_duty_cycle_fully_off();
        let _ = self.high.set_duty_normalised(power);
        self.phase_state = HIGH;
    }

    fn has_adc(&self) -> bool {
        self.adc_pin.is_some()
    }

    async fn get_phase_voltage(&mut self) -> Option<f32> {
        if self.phase_state == DISABLED {
            if let Some(ref mut adc_pin) = self.adc_pin {
                let mut adc = self.adc_mutex.lock().await;
                let raw_voltage = adc.read(adc_pin).await.ok();
                println!("{}", raw_voltage);
                if let Some(voltage) = raw_voltage {
                    let voltage = voltage as f32 / 4096.0 * 3.3 * 3.0;
                    Some(voltage)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// 6-phase commutation state for 3-phase BLDC motor
#[derive(Clone, Copy, Debug, PartialEq)]
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
    adc: &'static ADCMutex,
    current_sense_pin: Channel<'static>,
    back_emf_common_pin: Channel<'static>,
    kv: u16,
    vbat: f32,
    commutations_per_rotation: u8,
    last_update_micros: u64,
    last_commutation_micros: u64,
    max_rps: u16,
    target_rps: i16,
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
        adc: &'static ADCMutex,
        current_sense_pin: Channel<'static>,
        back_emf_common_pin: Channel<'static>,
    ) -> Self {
        let mut bldc = Self {
            a_phase,
            b_phase,
            c_phase,
            adc,
            current_sense_pin,
            state: CommutationState::State0,
            power: 0.0,
            back_emf_common_pin,
            kv: 1750,
            vbat: 7.4,
            commutations_per_rotation: 42,
            last_update_micros: 0,
            last_commutation_micros: 0,
            target_rps: 0,
            max_rps: 200,
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

    async fn get_back_emf_common_voltage(&mut self) -> f32 {
        let voltage: f32;
        let voltage_raw: u16;
        {
            let mut adc_raw = self.adc.lock().await;
            voltage_raw = adc_raw.read(&mut self.current_sense_pin).await.unwrap();
        }
        voltage = voltage_raw as f32 / 4096.0 * 3.0;

        voltage
    }

    pub async fn get_current_amps(&mut self) -> f32 {
        let current_amps: f32;
        let current_raw: u16;
        {
            let mut adc_raw = self.adc.lock().await;
            current_raw = adc_raw.read(&mut self.current_sense_pin).await.unwrap();
        }
        current_amps = (current_raw as f32 - 2048.0) / 2048.0 * 30.0;

        current_amps
    }

    // Run at 20khz?
    pub fn update(&mut self, micros: u64) {
        // self.disable();
        let dt = micros - self.last_update_micros;

        // println!("dt: {}", dt);

        let commutations_per_second: u64 =
            self.commutations_per_rotation as u64 * self.target_rps as u64;
        let commutation_delay_micros: u64;
        if commutations_per_second != 0 {
            commutation_delay_micros = 1_000_000 / commutations_per_second;

            let next_commutation = self.last_commutation_micros + commutation_delay_micros;
            // println!(
            //     "next_commutation: {}, micros: {}, commutation_delay_micros: {},",
            //     next_commutation, micros, commutation_delay_micros
            // );
            if (next_commutation < micros) {
                // println!("micros: {}", &micros);
                self.last_commutation_micros += commutation_delay_micros;

                // Don't let it fall too far behind
                if self.last_commutation_micros < micros {
                    self.last_commutation_micros = micros;
                }
                // self.progress(self.target_rps as f32 / self.max_rps as f32);
                self.progress(0.5);
                // println!("commutate");
            }
        }

        // if self.state == CommutationState::State2 {
        //     let phase_a_voltage = self
        //         .a_phase
        //         .get_phase_voltage()
        //         .await
        //         .expect("Phase A voltage is important");

        //     let common_voltage = self.get_back_emf_common_voltage().await;

        //     println!(
        //         "{}, {}, {}",
        //         phase_a_voltage < common_voltage,
        //         phase_a_voltage,
        //         common_voltage
        //     )
        // }
        self.last_update_micros = micros;
    }

    pub fn set_target_rps(&mut self, new_target: i16) {
        self.target_rps = new_target;
    }
}
