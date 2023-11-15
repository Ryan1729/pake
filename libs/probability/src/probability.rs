pub type Probability = u8;

pub const TWENTY_FIVE_PERCENT: Probability = 0b0100_0000;
pub const FIFTY_PERCENT: Probability = 0b1000_0000;
pub const SEVENTY_FIVE_PERCENT: Probability = 0b1100_0000;
pub const EIGHTY_SEVEN_POINT_FIVE_PERCENT: Probability = 0b1110_0000;
pub const ONE: Probability = 0b1111_1111;

pub type Count = u32;

#[derive(Clone, Copy, Debug)]
pub struct EvalCount {
    pub win_count: Count,
    pub total: Count,
}

impl EvalCount {
    pub fn probability(self) -> Probability {
        assert!(self.total > 0);
        let frac = f64::from(self.win_count) / f64::from(self.total);

        ((frac * 256.) + 0.5) as Probability
    }
}

#[test]
fn probability_works_in_these_cases() {
    macro_rules! a {
        ($numerator: expr , $denomenator: expr => $expected: expr) => ({
            let eval_count = EvalCount {
                win_count: $numerator,
                total: $denomenator,
            };
            assert_eq!(eval_count.probability(), $expected);
        })
    }

    a!(1, 1 => ONE);
    a!(0, 1 => 0);
    a!(1, 2 => FIFTY_PERCENT);
    a!(3, 4 => SEVENTY_FIVE_PERCENT);
    for x in 0..256 {
        a!(x, 256 => u8::try_from(x).unwrap());
    }
}