pub mod probability {
    pub type Probability = u8;
    pub const FIFTY_PERCENT: Probability = 0b1000_000;
    pub const SEVENTY_FIVE_PERCENT: Probability = 0b1100_000;
}


pub mod holdem {
    use crate::probability::{FIFTY_PERCENT, Probability};
    use models::{ALL_CARDS, DECK_SIZE, holdem::Hand};

    const ALL_SORTED_HANDS_LEN: usize = 1326;
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

    pub fn hand_win_probability(hand: Hand) -> Probability {
        // TODO index into a pre-generated look up table, instead of this incorrect 
        // placeholder.
        FIFTY_PERCENT
    }

    fn hand_to_sorted_hand_index(hand: Hand) -> usize {
        let sorted_hand = if hand[0] > hand[1] {
            [hand[1], hand[0]]
        } else {
            hand
        };

        let s0 = usize::from(sorted_hand[0]);
        let s1 = usize::from(sorted_hand[1]);
        let deck_size = usize::from(DECK_SIZE);

        // TODO? Simplify this formula which was derived through trial and error?
        // Or maybe figure out why it works?
        s0 * (2 * deck_size - s0 - 3)/2 + s1 - 1
    }

    #[test]
    fn hand_to_sorted_hand_index_works_on_all_the_sorted_hands() {
        for hand in ALL_SORTED_HANDS {
            let index = hand_to_sorted_hand_index(hand);
            let sorted_hand = ALL_SORTED_HANDS[index];

            // We don't strictly speaking have to use the same ordering as
            // `ALL_SORTED_HANDS`, but we might as well, since it makes this
            // test easier to write.
            assert!(
                (hand[0] == sorted_hand[0] || hand[0] == sorted_hand[1])
                && (hand[1] == sorted_hand[0] || hand[1] == sorted_hand[1]),
                "{sorted_hand:?} was not a sorted version of {hand:?}.",
            );
        }
    }
}


