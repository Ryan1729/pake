pub mod holdem {
    use probability::{Probability};
    use models::{ALL_CARDS, DECK_SIZE, holdem::Hand};

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

    const WIN_PROBABILITY_LEN: usize = ALL_SORTED_HANDS_LEN;
    const WIN_PROBABILITY: [Probability; WIN_PROBABILITY_LEN] = include!("holdem_win_probability.in");

    #[test]
    fn win_probability_seems_sane() {
        use probability::{SEVENTY_FIVE_PERCENT};
        // Multiple external sources say pocket aces are 85% likely to win.
        // So, let's use 75% as a reasonable lower-bound that should always
        // be achievable.
        const ACES: [models::Card; 4] = [0, 13, 26, 39];
        for a1 in ACES {
            for a2 in ACES {
                if a1 == a2 { continue }
                let hand = [a1, a2];
                let index = hand_to_sorted_hand_index(hand);
                assert!(
                    WIN_PROBABILITY[index]
                    >= SEVENTY_FIVE_PERCENT,
                    "{hand:?} P(win) = {} < {SEVENTY_FIVE_PERCENT} (75%)",
                    WIN_PROBABILITY[index]
                );
            }
        }
    }

    pub const SUITED_WIN_PROBABILITY_LEN: usize = 312;
    pub const SUITED_WIN_PROBABILITY: [Probability; SUITED_WIN_PROBABILITY_LEN] = 
        include!("suited_holdem_win_probability.in");

    pub const UNSUITED_WIN_PROBABILITY_LEN: usize = 1014;
    pub const UNSUITED_WIN_PROBABILITY: [Probability; UNSUITED_WIN_PROBABILITY_LEN] = 
        include!("unsuited_holdem_win_probability.in");

    pub fn hand_win_probability(hand: Hand) -> Probability {
        WIN_PROBABILITY[hand_to_sorted_hand_index(hand)]
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

pub mod five_card {
    use probability::{Probability};
    use models::{ALL_CARDS, DECK_SIZE, Card};

    // 5 cards seems like too many to use a static array of sorted hands.
    // Later games will definitely have too many, so we should have a way
    // to produce a reasonable probability without a static array!

    pub fn hand_win_probability(hand: [Card; 5]) -> Probability {
        todo!("{hand:?}")
    }
}


