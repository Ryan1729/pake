
use models::holdem;

pub struct Eval(poker::Eval);

impl Default for Eval {
    fn default() -> Self {
        Self(poker::Eval::WORST)
    }
}

impl core::fmt::Display for Eval {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Eval {}", self.0)
    }
}

pub fn holdem_hand(community_cards: holdem::CommunityCards, hand: holdem::Hand) -> Eval {
    use holdem::CommunityCards::*;

    let mut cards = [to_poker_card(0); 7];

    let len = match community_cards {
        Flop(flop) => {
            cards[0] = to_poker_card(flop[0]);
            cards[1] = to_poker_card(flop[1]);
            cards[2] = to_poker_card(flop[2]);
            cards[3] = to_poker_card(hand[0]);
            cards[4] = to_poker_card(hand[1]);

            5
        },
        Turn(flop, turn) => {
            cards[0] = to_poker_card(flop[0]);
            cards[1] = to_poker_card(flop[1]);
            cards[2] = to_poker_card(flop[2]);
            cards[3] = to_poker_card(turn);
            cards[4] = to_poker_card(hand[0]);
            cards[5] = to_poker_card(hand[1]);

            6
        },
        River(flop, turn, river) => {
            cards[0] = to_poker_card(flop[0]);
            cards[1] = to_poker_card(flop[1]);
            cards[2] = to_poker_card(flop[2]);
            cards[3] = to_poker_card(turn);
            cards[4] = to_poker_card(river);
            cards[5] = to_poker_card(hand[0]);
            cards[6] = to_poker_card(hand[1]);

            7
        },
    };

    // Measuring time {
    
    let all_other_cards: Vec<_> = ALL_CARDS
        .iter()
        .filter(|&&card| {
            card != to_poker_card(hand[0]) && card != to_poker_card(hand[1])
        })
        .collect();
    const OTHER_CARDS_LEN: usize = ALL_CARDS.len() - 2;
    assert_eq!(all_other_cards.len(), OTHER_CARDS_LEN);
    let mut mask: u64 = 0;
    let mut selected: Vec<poker::Card> = Vec::with_capacity(3);
    while mask < 1 << OTHER_CARDS_LEN {
        if mask.count_ones() == 3 {
            //dbg!(mask);
            selected.clear();
            for c in all_other_cards.iter()
                .enumerate()
                .filter(|(index, _)| (mask >> index) & 1 == 0)
                .map(|(_, e)| **e) {
                selected.push(c);
            }
            match poker::evaluate::static_lookup::evaluate(&selected) {
                Err(err) => {
                    debug_assert!(false, "evaluate Err: {err}");
                    return Eval::default()
                },
                _ => {}
            }
        }
        mask += 1;
    }
    // }

    match poker::evaluate::static_lookup::evaluate(&cards[..len]) {
        Ok(eval) => Eval(eval),
        Err(err) => {
            debug_assert!(false, "evaluate Err: {err}");
            Eval::default()
        },
    }
}

const fn to_poker_card(card: models::Card) -> poker::Card {
    let rank = models::get_rank(card);

    let suit = models::get_suit(card);

    poker::Card::new(
        match rank {
            0 => poker::Rank::Ace,
            // 1 maps to Two
            2 => poker::Rank::Three,
            3 => poker::Rank::Four,
            4 => poker::Rank::Five,
            5 => poker::Rank::Six,
            6 => poker::Rank::Seven,
            7 => poker::Rank::Eight,
            8 => poker::Rank::Nine,
            9 => poker::Rank::Ten,
            10 => poker::Rank::Jack,
            11 => poker::Rank::Queen,
            12 => poker::Rank::King,
            _ => poker::Rank::Two,
        },
        match suit {
            models::suits::DIAMONDS => poker::Suit::Diamonds,
            models::suits::HEARTS => poker::Suit::Hearts,
            models::suits::SPADES => poker::Suit::Spades,
            _ => poker::Suit::Clubs,
        },
    )
}

const ALL_CARDS: [poker::Card; 52] = [
    poker::Card::new(poker::Rank::Two, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Two, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Two, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Two, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Three, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Three, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Three, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Three, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Four, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Four, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Four, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Four, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Five, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Five, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Five, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Five, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Six, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Six, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Six, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Six, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Seven, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Seven, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Seven, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Seven, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Eight, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Eight, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Eight, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Eight, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Nine, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Nine, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Nine, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Nine, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Ten, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Ten, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Ten, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Ten, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Jack, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Jack, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Jack, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Jack, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Queen, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Queen, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Queen, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Queen, poker::Suit::Spades),
    poker::Card::new(poker::Rank::King, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::King, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::King, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::King, poker::Suit::Spades),
    poker::Card::new(poker::Rank::Ace, poker::Suit::Clubs),
    poker::Card::new(poker::Rank::Ace, poker::Suit::Diamonds),
    poker::Card::new(poker::Rank::Ace, poker::Suit::Hearts),
    poker::Card::new(poker::Rank::Ace, poker::Suit::Spades),
];