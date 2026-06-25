use rp235x_hal::pwm::{A, B, Channel, FreeRunning, Pwm0, Pwm1, Pwm5, Pwm6, Pwm7, Slice};

pub type RGBRed = Channel<Slice<Pwm0, FreeRunning>, A>;
pub type RGBGreen = Channel<Slice<Pwm0, FreeRunning>, B>;
pub type RGBBlue = Channel<Slice<Pwm1, FreeRunning>, A>;

pub type RGBLed = crate::drivers::rgb_led::RGBLed<RGBRed, RGBGreen, RGBBlue>;

pub type BLDCLowC = Channel<Slice<Pwm5, FreeRunning>, A>;
pub type BLDCLowB = Channel<Slice<Pwm5, FreeRunning>, B>;
pub type BLDCLowA = Channel<Slice<Pwm6, FreeRunning>, A>;

pub type BLDCHighC = Channel<Slice<Pwm6, FreeRunning>, B>;
pub type BLDCHighB = Channel<Slice<Pwm7, FreeRunning>, A>;
pub type BLDCHighA = Channel<Slice<Pwm7, FreeRunning>, B>;

pub type BLDC =
    crate::drivers::bldc::BLDC<BLDCLowA, BLDCLowB, BLDCLowC, BLDCHighA, BLDCHighB, BLDCHighC>;
