use models::{Card, holdem::{Hand}};

use std::fs::OpenOptions;

// TODO? Import this look_up stuff in some way that doesn't produce a dependency
// cycle?
mod look_up {
    pub mod probability {
        pub type Probability = u8;
        pub const FIFTY_PERCENT: Probability = 0b1000_0000;
        pub const SEVENTY_FIVE_PERCENT: Probability = 0b1100_0000;
        pub const ONE: Probability = 0b1111_1111;
    }

    pub mod holdem {
        use models::{ALL_CARDS, holdem::{Hand}};

        pub const ALL_SORTED_HANDS_LEN: usize = 1326;
        pub const ALL_SORTED_HANDS: [Hand; ALL_SORTED_HANDS_LEN] = {
            let mut all_hands = [[0; 2]; ALL_SORTED_HANDS_LEN];

            let mut index = 0;

            let mut i1 = 0;
            while i1 < ALL_CARDS.len() {
                let mut i2 = i1 + 1;
                while i2 < ALL_CARDS.len() {
                    let c1 = ALL_CARDS[i1];
                    let c2 = ALL_CARDS[i2];

                    all_hands[index] = [c1, c2];
                    index += 1;
                    i2 += 1;
                }
                i1 += 1;
            }

            all_hands
        };
    }
}

use look_up::{
    holdem::{
        ALL_SORTED_HANDS_LEN,
        ALL_SORTED_HANDS,
    },
    probability::{Probability, FIFTY_PERCENT},
};

type Flop = [Card; 3];

fn probability_for_hand(hand: Hand) -> Probability {
    //const ALL_SORTED_FLOPS_LEN: usize = 380204032;
    //const ALL_SORTED_FLOPS: [Hand; ALL_SORTED_FLOPS_LEN] = {
        //let mut all_flops = [[0; 3]; ALL_SORTED_FLOPS_LEN];
    //
        //let mut index = 0;
    //
        //let mut i1 = 0;
        //while i1 < ALL_CARDS.len() {
            //let mut i2 = i1 + 1;
            //while i2 < ALL_CARDS.len() {
                //let mut i3 = i1 + 1;
                //while i3 < ALL_CARDS.len() {
                    //let c1 = ALL_CARDS[i1];
                    //let c2 = ALL_CARDS[i2];
                    //let c3 = ALL_CARDS[i3];
    //
                    //all_flops[index] = [c1, c2];
                    //index += 1;
                    //i3 += 1;
                //}
                //i2 += 1;
            //}
            //i1 += 1;
        //}
    //
        //all_flops
    //};
//
    FIFTY_PERCENT
}

type Count = u32;

#[derive(Clone, Copy)]
struct EvalCount {
    win_count: Count,
    total: Count,
}

impl EvalCount {
    fn probability(self) -> Probability {
        assert!(self.total > 0);
        let frac = f64::from(self.win_count) / f64::from(self.total);

        ((frac * 256.) + 0.5) as Probability
    }
}

#[test]
fn probability_works_in_these_cases() {
    use look_up::{probability::{FIFTY_PERCENT, SEVENTY_FIVE_PERCENT, ONE}};

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    const WIN_PROBABILTY_OUTPUT_PATH: &str = "../../libs/look_up/src/holdem_win_probability.in";

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(WIN_PROBABILTY_OUTPUT_PATH)?;

    let mut eval_counts: [EvalCount; ALL_SORTED_HANDS_LEN] = [
        EvalCount {
            win_count: 0,
            total: 0,
        };
        ALL_SORTED_HANDS_LEN
    ];

    // TODO remove the need for this
    for count in eval_counts.iter_mut() {
        count.win_count = 1;
        count.total = 2;
    }

    write!(file, "[")?;

    for (i, count) in eval_counts.iter().enumerate() {
        write!(file, "{},", count.probability())?;
    }

    write!(file, "]")?;

    Ok(())
}
