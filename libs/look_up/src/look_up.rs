pub mod probability {
    pub type Probability = u8;
    pub const FIFTY_PERCENT: Probability = 0b1000_000;
    pub const SEVENTY_FIVE_PERCENT: Probability = 0b1100_000;
}


pub mod holdem {
    use crate::probability::{FIFTY_PERCENT, Probability};
    use models::holdem::Hand;

    pub fn hand_win_probability(hand: Hand) -> Probability {
        // TODO index into a pre-generated look up table, instead of this incorrect 
        // placeholder.
        FIFTY_PERCENT
    }
}


