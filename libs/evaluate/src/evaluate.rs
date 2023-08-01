
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

    match poker::evaluate::static_lookup::evaluate(&cards[..len]) {
        Ok(eval) => Eval(eval),
        Err(err) => {
            debug_assert!(false, "evaluate Err: {err}");
            Eval::default()
        },
    }
}

fn to_poker_card(card: models::Card) -> poker::Card {
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