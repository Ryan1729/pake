use models::{ALL_CARDS, Card, holdem::{CommunityCards, Hand}};

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

        pub const ALL_SORTED_HANDS_LEN: u16 = 1326;
        pub const ALL_SORTED_HANDS: [Hand; ALL_SORTED_HANDS_LEN as usize] = {
            let mut all_hands = [[0; 2]; ALL_SORTED_HANDS_LEN as usize];

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

const ALL_SORTED_FLOPS_LEN: u32 = 380204032;

fn sorted_flop(index: usize) -> Flop {
    use std::sync::OnceLock;
    // If this was a const, it takes too much memory to compile. Plus lots of space
    // on disk for the executable!
    static ALL_SORTED_FLOPS: OnceLock<Box<[Flop]>> = OnceLock::new();
    let all_sorted_flops = ALL_SORTED_FLOPS.get_or_init(|| {
        let mut all_flops = vec![[0; 3]; ALL_SORTED_FLOPS_LEN as usize];

        let mut index = 0;
    
        let mut i1 = 0;
        while i1 < ALL_CARDS.len() {
            let mut i2 = i1 + 1;
            while i2 < ALL_CARDS.len() {
                let mut i3 = i1 + 1;
                while i3 < ALL_CARDS.len() {
                    let c1 = ALL_CARDS[i1];
                    let c2 = ALL_CARDS[i2];
                    let c3 = ALL_CARDS[i3];
    
                    all_flops[index] = [c1, c2, c3];
                    index += 1;
                    i3 += 1;
                }
                i2 += 1;
            }
            i1 += 1;
        }
    
        all_flops.into_boxed_slice()
    });

    all_sorted_flops[index]
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

type HandIndex = u16;
type HandCount = u16;
type FlopIndex = u32;
type FlopCount = u32;

#[inline]
fn count_evaluation(
    counts: &mut [EvalCount; ALL_SORTED_HANDS_LEN as usize],
    hand_i_1: HandIndex,
    hand_i_2: HandIndex,
    flop_i: FlopIndex,
) {
    let hand_i_1 = hand_i_1 as usize;
    let hand_i_2 = hand_i_2 as usize;
    let flop_i = flop_i as usize;

    let hand_1 = ALL_SORTED_HANDS[hand_i_1];
    let hand_2 = ALL_SORTED_HANDS[hand_i_2];
    let flop = sorted_flop(flop_i);

    let eval_1 = evaluate::holdem_hand(
        CommunityCards::Flop(flop),
        hand_1,
    );
    let eval_2 = evaluate::holdem_hand(
        CommunityCards::Flop(flop),
        hand_2,
    );

    use core::cmp::Ordering::*;
    match eval_1.cmp(&eval_2) {
        Greater => {
            counts[hand_i_1].win_count += 1;
        },
        Equal => {},
        Less => {
            counts[hand_i_2].win_count += 1;
        },
    }

    counts[hand_i_1].total += 1;

    counts[hand_i_2].total += 1;
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    const WIN_PROBABILTY_OUTPUT_PATH: &str = "../../libs/look_up/src/holdem_win_probability.in";

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(WIN_PROBABILTY_OUTPUT_PATH)?;

    let seed = {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let time = time.as_secs_f64();
    
        unsafe {
            core::mem::transmute::<[f64; 2], [u8; 16]>([time, 1.0 / time])
        }
    };

    println!("{seed:?}");

    let mut rng = xs::from_seed(seed);

    let mut eval_counts: [EvalCount; ALL_SORTED_HANDS_LEN as usize] = [
        EvalCount {
            win_count: 0,
            total: 0,
        };
        ALL_SORTED_HANDS_LEN as usize
    ];

    const SUBSET_SIZE: FlopCount = 1 << 12;//1 << 18;
    // TODO? maybe multiple random subsets would reduce bias?
    let mut subset: [FlopIndex; SUBSET_SIZE as usize] = [0; SUBSET_SIZE as usize];
    
    for output_index in subset.iter_mut() {
        // Pick a (more) normally distributed set by taking the average of N samples.
        const SAMPLE_PO2: u8 = 16;
        for _ in 0..SAMPLE_PO2 {
            *output_index = output_index.saturating_add(xs::range(&mut rng, 0..ALL_SORTED_FLOPS_LEN));
        }
        *output_index >>= u32::from(SAMPLE_PO2);

        assert!(*output_index < ALL_SORTED_FLOPS_LEN);
    }

    // TODO? Measure whether sorting like this meaningfully improves cache locality?
    subset.sort();

    for hand_i_1 in 0..ALL_SORTED_HANDS_LEN {
        println!("{hand_i_1}/{ALL_SORTED_HANDS_LEN}");
        for hand_i_2 in 0..ALL_SORTED_HANDS_LEN {
            //println!("    {hand_i_2}/{ALL_SORTED_HANDS_LEN}");
            for flop_i in subset {
                count_evaluation(
                    &mut eval_counts,
                    hand_i_1,
                    hand_i_2,
                    flop_i,
                );
            }
        }
    }
    println!("{ALL_SORTED_HANDS_LEN}/{ALL_SORTED_HANDS_LEN}");

    writeln!(file, "// Seed used was: {seed:?}")?;
    write!(file, "[")?;

    for (i, count) in eval_counts.iter().enumerate() {
        write!(file, "{},", count.probability())?;
    }

    write!(file, "]")?;

    // Actually flush to disk before printing the success message.
    file.flush()?;
    drop(file);

    println!("wrote win_probabilty to {WIN_PROBABILTY_OUTPUT_PATH}");

    Ok(())
}
