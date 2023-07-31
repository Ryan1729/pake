
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
    let cards: &[poker::Card] = match community_cards {
        Flop(flop) => {
            todo!()
        },
        Turn(flop, turn) => {
            todo!()
        },
        River(flop, turn, river) => {
            todo!()
        },
    };

    match poker::evaluate::static_lookup::evaluate(cards) {
        Ok(eval) => Eval(eval),
        Err(err) => {
            debug_assert!(false, "evaluate Err: {err}");
            Eval::default()
        },
    }
}