use models::{ALL_CARDS, Card, get_suit, holdem::{CommunityCards, Hand}};

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

const ALL_SORTED_FLOPS_LEN: u32 = 22_100;

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
                let mut i3 = i2 + 1;
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
        assert_eq!(all_flops.len(), ALL_SORTED_FLOPS_LEN as usize);
        all_flops.into_boxed_slice()
    });

    all_sorted_flops[index]
}

#[test]
fn sorted_flop_has_the_expected_last_flop() {
    assert_eq!(sorted_flop((ALL_SORTED_FLOPS_LEN - 1) as usize), [49, 50, 51]);
}

type Count = u32;

#[derive(Clone, Copy, Debug)]
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
    assert_ne!(hand_i_1, hand_i_2);

    let hand_i_1 = hand_i_1 as usize;
    let hand_i_2 = hand_i_2 as usize;
    let flop_i = flop_i as usize;

    let hand_1 = ALL_SORTED_HANDS[hand_i_1];
    let hand_2 = ALL_SORTED_HANDS[hand_i_2];
    let flop = sorted_flop(flop_i);

    {
        let mut seen = std::collections::HashSet::new();

        let cards = [
            hand_1[0],
            hand_1[1],
            hand_2[0],
            hand_2[1],
            flop[0],
            flop[1],
            flop[2],
        ];

        let all_unique = cards
            .into_iter()
            .all(move |x| seen.insert(x));

        if !all_unique { return }
        //assert!(all_unique, "{cards:?} had duplicates");
    }

    let eval_1 = evaluate::holdem_hand(
        CommunityCards::Flop(flop),
        hand_1,
    );
    let eval_2 = evaluate::holdem_hand(
        CommunityCards::Flop(flop),
        hand_2,
    );
//dbg!(hand_1, hand_2, eval_1, eval_2);
    use core::cmp::Ordering::*;
    match eval_1.cmp(&eval_2) {
        Greater => {
            //dbg!(Greater, hand_i_1);
            let count = &mut (counts[hand_i_1].win_count);
            *count = count.saturating_add(1);
        },
        Equal => {},
        Less => {
            //dbg!(Less, hand_i_2);
            let count = &mut (counts[hand_i_2].win_count);
            *count = count.saturating_add(1);
        },
    }

    counts[hand_i_1].total += 1;

    counts[hand_i_2].total += 1;
}

#[test]
fn count_evaluation_works_on_these_few_flops() {
    let mut eval_counts: [EvalCount; ALL_SORTED_HANDS_LEN as usize] = [
        EvalCount {
            win_count: 0,
            total: 0,
        };
        ALL_SORTED_HANDS_LEN as usize
    ];

    const FLOPS_TO_CHECK: FlopCount = 4;

    for hand_i_1 in 0..ALL_SORTED_HANDS_LEN {
        for hand_i_2 in (hand_i_1 + 1)..ALL_SORTED_HANDS_LEN {
            for flop_i in 0..4 {
                count_evaluation(
                    &mut eval_counts,
                    hand_i_1,
                    hand_i_2,
                    flop_i,
                );
            }
        }
    }

    let eval_counts = &eval_counts[1000..1008];

    assert!(
        !eval_counts.iter().any(|count| count.total <= FLOPS_TO_CHECK),
        "a total was <= {FLOPS_TO_CHECK}\n {eval_counts:?}"
    );

    assert!(
        eval_counts.iter().any(|count| count.probability() != 0),
        "all propabilities were == 0\n {eval_counts:?}"
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;

    const WIN_PROBABILTY_OUTPUT_PATH: &str = "../../libs/look_up/src/holdem_win_probability.in";
    const SUITED_WIN_PROBABILTY_OUTPUT_PATH: &str = "../../libs/look_up/src/suited_holdem_win_probability.in";
    const UNSUITED_WIN_PROBABILTY_OUTPUT_PATH: &str = "../../libs/look_up/src/unsuited_holdem_win_probability.in";

    let mut plain_win_prob_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(WIN_PROBABILTY_OUTPUT_PATH)?;

    let mut suited_win_prob_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(SUITED_WIN_PROBABILTY_OUTPUT_PATH)?;

    let mut unsuited_win_prob_file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(false)
        .open(UNSUITED_WIN_PROBABILTY_OUTPUT_PATH)?;

    let seed = {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let time = time.as_secs_f64();
    
        unsafe {
            core::mem::transmute::<[f64; 2], [u8; 16]>([time, 1.0 / time])
        }
    };
    let seed = [173, 113, 107, 104, 41, 63, 217, 65, 113, 10, 152, 191, 164, 71, 4, 62];

    println!("{seed:?}");

    let mut rng = xs::from_seed(seed);

    let mut eval_counts: [EvalCount; ALL_SORTED_HANDS_LEN as usize] = [
        EvalCount {
            win_count: 0,
            total: 0,
        };
        ALL_SORTED_HANDS_LEN as usize
    ];

    let used_size;
    #[cfg(any())] // if 0
    {
        let flops;

        used_size = ALL_SORTED_FLOPS_LEN;
        let mut all_flops = [0; ALL_SORTED_FLOPS_LEN as usize];
        let mut index = 0;
        for flop in all_flops.iter_mut() {
            *flop = index;
            index += 1;
        }
        flops = all_flops;

        println!("Using {used_size}/{ALL_SORTED_FLOPS_LEN} flops");
    
        for hand_i_1 in 0..ALL_SORTED_HANDS_LEN {
            println!("{hand_i_1}/{ALL_SORTED_HANDS_LEN}");
            for hand_i_2 in (hand_i_1 + 1)..ALL_SORTED_HANDS_LEN {
                //println!("    {hand_i_2}/{ALL_SORTED_HANDS_LEN}");
                for flop_i in flops {
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
    }
    #[cfg(all())] // if 1
    {
        used_size = ALL_SORTED_FLOPS_LEN;
        const WIN_PROBABILITY: [Probability; ALL_SORTED_HANDS_LEN as usize] = include!("../../../libs/look_up/src/holdem_win_probability.in");
        for i in 0..ALL_SORTED_HANDS_LEN as usize {
            eval_counts[i] = EvalCount {
                win_count: u32::from(WIN_PROBABILITY[i]),
                total: 256,
            };
        }
    }

    {
        writeln!(plain_win_prob_file, "// Seed used was: {seed:?}. Used {used_size}/{ALL_SORTED_FLOPS_LEN} flops")?;
        write!(plain_win_prob_file, "[")?;
    
        for (i, count) in eval_counts.iter().enumerate() {
            write!(plain_win_prob_file, "{},", count.probability())?;
        }
    
        writeln!(plain_win_prob_file, "]")?;
    
        // Actually flush to disk before printing the success message.
        plain_win_prob_file.flush()?;
        drop(plain_win_prob_file);

        println!("wrote win_probabilty to {WIN_PROBABILTY_OUTPUT_PATH}");
    }
    {
        writeln!(suited_win_prob_file, "// Seed used was: {seed:?}. Used {used_size}/{ALL_SORTED_FLOPS_LEN} flops")?;
        write!(suited_win_prob_file, "[")?;
    
        // TODO put in good order for chart
        for (i, count) in eval_counts.iter().enumerate() {
            let hand = ALL_SORTED_HANDS[i];
            if get_suit(hand[0]) == get_suit(hand[1]) {
                write!(suited_win_prob_file, "{},", count.probability())?;
            }
        }
    
        writeln!(suited_win_prob_file, "]")?;
    
        // Actually flush to disk before printing the success message.
        suited_win_prob_file.flush()?;
        drop(suited_win_prob_file);

        println!("wrote suited_win_probabilty to {SUITED_WIN_PROBABILTY_OUTPUT_PATH}");
    }
    {
        writeln!(unsuited_win_prob_file, "// Seed used was: {seed:?}. Used {used_size}/{ALL_SORTED_FLOPS_LEN} flops")?;
        write!(unsuited_win_prob_file, "[")?;
    
        // TODO put in good order for chart
        for (i, count) in eval_counts.iter().enumerate() {
            let hand = ALL_SORTED_HANDS[i];
            if get_suit(hand[0]) != get_suit(hand[1]) {
                write!(unsuited_win_prob_file, "{},", count.probability())?;
            }
        }
    
        writeln!(unsuited_win_prob_file, "]")?;
    
        // Actually flush to disk before printing the success message.
        unsuited_win_prob_file.flush()?;
        drop(unsuited_win_prob_file);

        println!("wrote unsuited_win_probabilty to {UNSUITED_WIN_PROBABILTY_OUTPUT_PATH}");
    }

    // We'd rather have a bad version on disk that we can examine, than abort
    // early, given we're looking at things we can't know until the expensive
    // calculation is done.
    {   
        let mut show_eval_counts = false;
        if eval_counts.iter().any(|count| count.total <= 10) {
            println!("WARNING: a total was <= 10");
            show_eval_counts = true;
        }

        if eval_counts.iter().any(|count| count.probability() == 0) {
            println!("WARNING: a propability was == 0");
            show_eval_counts = true;
        }

        if show_eval_counts {
            println!("{eval_counts:?}");
        }
    }

    Ok(())
}
