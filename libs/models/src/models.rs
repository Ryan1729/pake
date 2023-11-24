use xs::Xs;
use core::num::NonZeroU32;

macro_rules! compile_time_assert {
    ($assertion: expr) => (
        #[allow(unknown_lints, clippy::eq_op)]
        // Based on the const_assert macro from static_assertions;
        const _: [(); 0 - !{$assertion} as usize] = [];
    )
}

pub const RANK_COUNT: u8 = 13;
pub const SUIT_COUNT: u8 = 4;
pub const DECK_SIZE: u8 = RANK_COUNT * SUIT_COUNT;

pub type Card = u8;

pub const ALL_CARDS: [Card; DECK_SIZE as usize] = {
    let mut all_cards = [0; DECK_SIZE as usize];

    let mut c = 0;
    while c < DECK_SIZE {
        all_cards[c as usize] = c;
        c += 1;
    }

    all_cards
};

pub fn card_text_byte(card: Card) -> u8 {
    match get_rank(card) {
        0 => b'a',
        1 => b'2',
        2 => b'3',
        3 => b'4',
        4 => b'5',
        5 => b'6',
        6 => b'7',
        7 => b'8',
        8 => b'9',
        9 => b't',
        10 => b'j',
        11 => b'q',
        12 => b'k',
        _ => b'?',
    }
}

#[cfg(any())]
pub fn gen_card(rng: &mut Xs) -> Card {
    xs::range(rng, 0..DECK_SIZE as _) as Card
}

pub type Suit = u8;

pub mod suits {
    use super::*;

    pub const CLUBS: Suit = 0;
    pub const DIAMONDS: Suit = 1;
    pub const HEARTS: Suit = 2;
    pub const SPADES: Suit = 3;
}

pub const fn get_suit(card: Card) -> Suit {
    card / RANK_COUNT
}

pub type Rank = u8;

pub mod ranks {
    use super::*;

    pub const ACE: Rank = 0;
    pub const HIGH_ACE: Rank = 13;
}

pub const fn get_rank(card: Card) -> Rank {
    card % RANK_COUNT
}

type CardAmount = u8;
type CardIndex = u8;

#[derive(Clone, Debug)]
pub struct Deck {
    cards: [Card; DECK_SIZE as usize],
    index: CardIndex,
}

impl Default for Deck {
    fn default() -> Self {
        Self {
            cards: [0; DECK_SIZE as usize],
            index: 0,
        }
    }
}

impl Deck {
    pub fn draw(&mut self) -> Option<Card> {
        if self.index >= DECK_SIZE {
            None
        } else {
            let output = Some(self.cards[self.index as usize]);

            self.index += 1;

            output
        }
    }

    pub fn burn(&mut self) {
        self.draw();
    }
}

pub fn gen_deck(rng: &mut Xs) -> Deck {
    let mut output = Deck::default();
    for i in 1..DECK_SIZE {
        output.cards[i as usize] = i;
    }
    xs::shuffle(rng, &mut output.cards);

    output
}

type CardBits = u64;
#[derive(Clone, Copy, Debug, Default)]
pub struct CardBitset(CardBits);

compile_time_assert!{
    CardBits::BITS >= DECK_SIZE as u32
}

impl CardBitset {
    pub fn full() -> Self {
        Self((1 << DECK_SIZE as CardBits) - 1)
    }

    pub fn len(&self) -> CardAmount {
        self.0.count_ones() as CardAmount
    }

    pub fn set(&mut self, card: Card){
        if card > DECK_SIZE { return }
        self.0 |= 1 << CardBits::from(card);
    }

    pub fn remove(&mut self, card: Card) {
        if card > DECK_SIZE { return }
        self.0 &= !(1 << CardBits::from(card));
    }

    pub fn iter(self) -> CardBitsetIter {
        CardBitsetIter {
            set: self,
            index: 0,
        }
    }
}

#[test]
fn full_is_full() {
    let full = CardBitset::full();

    assert_eq!(full.len(), DECK_SIZE);

    assert_eq!(full.iter().count(), DECK_SIZE as _);
}

pub struct CardBitsetIter {
    set: CardBitset,
    index: CardIndex,
}

impl Iterator for CardBitsetIter {
    type Item = Card;

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < DECK_SIZE {
            if (self.set.0 & 1 << self.index) != 0 {
                let output = self.index;

                self.index += 1;

                return Some(output);
            }

            self.index += 1;
        }

        None
    }
}

pub fn split_among(
    mut remaining: MoneyInner,
    targets: &mut [MoneyInner],
    remainder_goes_to: usize
) {
    if targets.is_empty() {
        debug_assert!(false, "split_among called with empty slice!");
        return;
    }

    debug_assert!(remaining % MIN_MONEY_UNIT.get() == 0);

    let len = targets.len();

    // TODO? More efficient version of this?
    // Will this actually ever be a bottleneck?
    let mut i = remainder_goes_to;
    if i >= len {
        i = 0;
    }
    while remaining > 0 {
        remaining = remaining.saturating_sub(MIN_MONEY_UNIT.get());
        targets[i] = targets[i].saturating_add(MIN_MONEY_UNIT.get());

        i += 1;
        if i >= len {
            i = 0;
        }
    }
}

#[test]
fn split_among_works_on_these_examples() {
    macro_rules! a {
        ($start_with: literal $targets: expr, $remainder_goes_to: literal => $expected: expr) => ({
            let mut targets = $targets;

            for el in &mut targets {
                *el *= MIN_MONEY_UNIT.get();
            }

            let mut expected = $expected;
            for el in &mut expected {
                *el *= MIN_MONEY_UNIT.get();
            }

            let start_with = $start_with * MIN_MONEY_UNIT.get();

            split_among(start_with, &mut targets[..], $remainder_goes_to);

            assert_eq!(targets, expected);
        })
    }
    a!(10 [0], 0 => [10]);
    a!(10 [0, 0, 0], 0 => [4, 3, 3]);
    a!(10 [0, 0, 0], 1 => [3, 4, 3]);
    a!(10 [0, 0, 0], 2 => [3, 3, 4]);
    a!(10 [0, 0, 0], 3 => [4, 3, 3]);
    a!(10 [0, 0, 0], 99 => [4, 3, 3]);

    a!(10 [5], 0 => [15]);
    // [1 + 4, 2 + 3, 3 + 3]
    a!(10 [1, 2, 3], 0 => [5, 5, 6]);
    // [1 + 3, 2 + 4, 3 + 3]
    a!(10 [1, 2, 3], 1 => [4, 6, 6]);
    // [1 + 3, 2 + 3, 3 + 4]
    a!(10 [1, 2, 3], 2 => [4, 5, 7]);
    a!(10 [1, 2, 3], 3 => [5, 5, 6]);
    a!(10 [1, 2, 3], 99 => [5, 5, 6]);
}

// TODO? Switch to a representation that has MIN_MONEY_UNIT as 1, but scales up for
//       display only?
pub const MIN_MONEY_UNIT: NonZeroMoneyInner = 
    NonZeroMoneyInner::MIN.saturating_add(5 - 1);
pub const INITIAL_ANTE_AMOUNT: NonZeroMoneyInner = 
    MIN_MONEY_UNIT.saturating_mul(MIN_MONEY_UNIT);

mod money {
    use super::*;

    use core::cmp::Ordering;

    pub type MoneyInner = u32;
    pub type NonZeroMoneyInner = NonZeroU32;
    
    /// We intentionally avoid implementing Copy because money should be conserved over
    /// the lifetime of a game, once it has been initialized.
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct Money(MoneyInner);

    impl PartialEq<MoneyInner> for Money {
        fn eq(&self, other: &MoneyInner) -> bool {
            self.0 == *other
        }
    }

    impl PartialEq<Money> for MoneyInner {
        fn eq(&self, other: &Money) -> bool {
            *self == other.0
        }
    }

    impl PartialOrd<MoneyInner> for Money {
        fn partial_cmp(&self, other: &MoneyInner) -> Option<Ordering> {
            self.0.partial_cmp(other)
        }
    }

    impl PartialOrd<Money> for MoneyInner {
        fn partial_cmp(&self, other: &Money) -> Option<Ordering> {
            self.partial_cmp(&other.0)
        }
    }

    impl core::fmt::Display for Money {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::iter::Sum<Money> for MoneyInner {
        fn sum<I>(iter: I) -> Self
           where I: Iterator<Item = Money> {
            MoneyInner::sum(iter.map(|m| m.0))
        }
    }
    
    impl <'iter> std::iter::Sum<&'iter Money> for MoneyInner {
        fn sum<I>(iter: I) -> Self
           where I: Iterator<Item = &'iter Money> {
            MoneyInner::sum(iter.map(|m| m.0))
        }
    }
    
    impl Money {
        // Is this actually needed? Seems to undermine making Money non-Copy
        // const MAX: Self = Self(MoneyInner::MAX);
        pub const ZERO: Self = Self(0);
    
        pub fn as_inner(&self) -> MoneyInner {
            self.0
        }
    
        pub fn array_from_inner_array<const N: usize>(
            array: [MoneyInner; N]
        ) -> [Money; N] {
            array.map(Money)
        }

        pub fn take(&mut self, to_take: MoneyInner) -> Money {
            self.take_all_but(self.0.saturating_sub(to_take))
        }
    
        pub fn take_all(&mut self) -> Money {
            self.take_all_but(0)
        }
    
        pub fn take_all_but(&mut self, to_leave: MoneyInner) -> Money {
            let output = Money(self.0.saturating_sub(to_leave));

            self.0 = core::cmp::min(self.0, to_leave);

            output
        }

        pub fn split_among(&mut self, targets: &mut [Money], remainder_goes_to: usize) {
            // TODO? refactor to avoid needing dynamic allocation?
            let mut amounts = vec![0; targets.len()];

            crate::split_among(
                self.as_inner(),
                &mut amounts[..],
                remainder_goes_to
            );

            for i in 0..targets.len() {
                let possibly_zero_amount = amounts[i];
                let Some(amount) = NonZeroMoneyInner::new(possibly_zero_amount)
                    else { continue };
                MoneyMove {
                    from: self,
                    to: &mut targets[i],
                    amount,
                }.perform();
            }

            assert_eq!(*self, 0);
        }
    }

    #[test]
    fn take_all_but_works_for_these_examples() {
        macro_rules! a {
            ($start_with: literal $to_leave: expr => $end_with: literal $taken: literal) => ({
                let mut m = Money($start_with);
    
                let taken = m.take_all_but($to_leave);
    
                assert_eq!(m.0, $end_with);
                assert_eq!(taken.0, $taken);
            })
        }
        a!(10 0 => 0 10);
        a!(10 4 => 4 6);
        a!(10 10 => 10 0);
        a!(10 99 => 10 0);
        a!(10 MoneyInner::MAX => 10 0);

        a!(0 0 => 0 0);
        a!(0 4 => 0 0);
        a!(0 10 => 0 0);
        a!(0 99 => 0 0);
        a!(0 MoneyInner::MAX => 0 0);
    }

    pub struct MoneyMove<'from, 'to> {
        pub from: &'from mut Money,
        pub to: &'to mut Money,
        pub amount: NonZeroMoneyInner,
    }

    impl <'from, 'to> MoneyMove<'from, 'to> {
        pub fn perform(self) {
            let amount = self.amount.get();
            let taken = self.from.take(amount);
            self.to.0 = self.to.0.saturating_add(taken.0);
        }
    }

    #[test]
    fn perform_works_for_these_examples() {
        macro_rules! a {
            (($from_before: literal $to_before: literal) $amount: expr => $from_after: literal $to_after: literal) => ({
                let mut from = Money($from_before);
                let mut to = Money($to_before);

                MoneyMove {
                    from: &mut from,
                    to: &mut to,
                    amount: NonZeroMoneyInner::new($amount).unwrap(),
                }.perform();
    
                assert_eq!(from.0, $from_after);
                assert_eq!(to.0, $to_after);
            })
        }
        
        a!((10 10) 4 => 6 14);
        a!((10 10) 10 => 0 20);
        a!((10 10) 99 => 0 20);
        a!((10 10) MoneyInner::MAX => 0 20);

        a!((10 0) 4 => 6 4);
        a!((10 0) 10 => 0 10);
        a!((10 0) 99 => 0 10);
        a!((10 0) MoneyInner::MAX => 0 10);

        a!((0 10) 4 => 0 10);
        a!((0 10) 10 => 0 10);
        a!((0 10) 99 => 0 10);
        a!((0 10) MoneyInner::MAX => 0 10);

        a!((0 0) 4 => 0 0);
        a!((0 0) 10 => 0 0);
        a!((0 0) 99 => 0 0);
        a!((0 0) MoneyInner::MAX => 0 0);
    }
    
    /// We intentionally avoid implementing Copy because money should be conserved over
    /// the lifetime of a game, once it has been initialized.
    #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
    pub struct NonZeroMoney(NonZeroMoneyInner);

    impl PartialEq<NonZeroMoneyInner> for NonZeroMoney {
        fn eq(&self, other: &NonZeroMoneyInner) -> bool {
            self.0 == *other
        }
    }

    impl PartialEq<NonZeroMoney> for NonZeroMoneyInner {
        fn eq(&self, other: &NonZeroMoney) -> bool {
            *self == other.0
        }
    }

    impl PartialOrd<NonZeroMoneyInner> for NonZeroMoney {
        fn partial_cmp(&self, other: &NonZeroMoneyInner) -> Option<Ordering> {
            self.0.partial_cmp(other)
        }
    }

    impl PartialOrd<NonZeroMoney> for NonZeroMoneyInner {
        fn partial_cmp(&self, other: &NonZeroMoney) -> Option<Ordering> {
            self.partial_cmp(&other.0)
        }
    }

    impl core::fmt::Display for NonZeroMoney {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

pub use money::{Money, MoneyInner, MoneyMove, NonZeroMoney, NonZeroMoneyInner};

pub mod holdem {
    use super::*;

    pub type Hand = [Card; 2];

    pub fn short_hand_text(mut hand: Hand) -> [u8; 2] {
        let rank_0 = get_rank(hand[0]);
        let rank_1 = get_rank(hand[1]);
        // High card first looks nicer
        // != 0 to put ace first
        if rank_0 != 0 && rank_0 < rank_1 {
            let temp = hand[0];
            hand[0] = hand[1];
            hand[1] = temp;
        }
        [card_text_byte(hand[0]), card_text_byte(hand[1])]
    }

    #[derive(Clone, Debug, Default)]
    pub enum Action {
        #[default]
        Fold,
        Call,
        Raise(MoneyInner)
    }

    #[derive(Debug)]
    pub struct ActionSpec {
        pub one_past_max_money: NonZeroMoneyInner,
        pub min_money_unit: NonZeroMoneyInner,
        pub minimum_raise_total: MoneyInner,
    }

    pub fn gen_action(
        rng: &mut Xs,
        ActionSpec {
            one_past_max_money,
            min_money_unit,
            minimum_raise_total
        }: ActionSpec
    ) -> Action {
        use Action::*;

        match xs::range(rng, 0..3) {
            0 => Fold,
            1 => Call,
            _ => {
                // TODO? Maybe just take max_money as a param?
                let max_money = one_past_max_money.get() - 1;

                if minimum_raise_total > max_money {
                    // Go all in
                    Call
                } else {
                    let max_in_units: MoneyInner = max_money/min_money_unit.get();
                    let min_in_units: MoneyInner = minimum_raise_total/min_money_unit.get();
                    let output_in_units = xs::range(rng, min_in_units..core::cmp::max(min_in_units, max_in_units).saturating_add(1)) as MoneyInner;
                    let output_in_money = output_in_units.saturating_mul(min_money_unit.get());

                    Raise(output_in_money)
                }
            }
        }
    }

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub enum ActionKind {
        #[default]
        Fold,
        Call,
        Raise,
    }

    impl ActionKind {
        pub fn text(self) -> &'static [u8] {
            use ActionKind::*;
            match self {
                Fold => b"fold",
                Call => b"call",
                Raise => b"raise",
            }
        }
    }

    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub enum AllowedKindMode {
        #[default]
        All,
        NoFolding,
        AllIn,
    }

    impl ActionKind {
        pub fn next_up(self, mode: AllowedKindMode) -> Self {
            use ActionKind::*;
            use AllowedKindMode::*;
            match mode {
                All => match self {
                    Fold => Call,
                    Call => Raise,
                    Raise => Fold,
                },
                NoFolding => match self {
                    Fold => Call,
                    Call => Raise,
                    Raise => Call,
                },
                AllIn => Call,
            }
        }

        pub fn next_down(self, mode: AllowedKindMode) -> Self {
            use ActionKind::*;
            use AllowedKindMode::*;
            match mode {
                All => match self {
                    Fold => Raise,
                    Call => Fold,
                    Raise => Call,
                },
                NoFolding => match self {
                    Fold => Raise,
                    Call => Raise,
                    Raise => Call,
                },
                AllIn => Call,
            }
        }
    }

    #[derive(Copy, Clone, Default)]
    pub enum Facing {
        #[default]
        Down,
        Up(Hand),
    }

    pub type PlayerIndex = u8;
    /// Does not necessarily contain a valid number of players for a round.
    /// For a type with that guarentee see `HandLen`.
    pub type PlayerAmount = u8;

    pub const MIN_PLAYERS: PlayerAmount = 2;
    /// With 52 cards, and 5 community cards, and 3 burn cards,
    /// that leaves 44 cards left over so the maximum amount of
    /// possible hands is 22.
    pub const MAX_PLAYERS: PlayerAmount = 22;
    // TODO? Is the amount of possible pots MAX_PLAYERS - 1? Or even lower?
    pub const MAX_POTS: u8 = MAX_PLAYERS;

    pub type PerPlayer<A> = [A; MAX_PLAYERS as usize];

    type PerPlayerBits = u32;
    #[derive(Clone, Copy, Debug, Default)]
    pub struct PerPlayerBitset(PerPlayerBits);

    compile_time_assert!{
        PerPlayerBits::BITS >= MAX_PLAYERS as u32
    }

    impl PerPlayerBitset {
        pub fn len(&self) -> PlayerAmount {
            self.0.count_ones() as PlayerAmount
        }

        pub fn set(&mut self, index: HandIndex){
            if index > MAX_PLAYERS { return }
            self.0 |= 1 << PerPlayerBits::from(index);
        }

        pub fn iter(self) -> PerPlayerBitsetIter {
            PerPlayerBitsetIter {
                set: self,
                index: 0,
            }
        }
    }

    pub struct PerPlayerBitsetIter {
        set: PerPlayerBitset,
        index: HandIndex,
    }

    impl Iterator for PerPlayerBitsetIter {
        type Item = HandIndex;

        fn next(&mut self) -> Option<Self::Item> {
            while self.index < MAX_PLAYERS {
                if (self.set.0 & 1 << self.index) != 0 {
                    let output = self.index;

                    self.index += 1;

                    return Some(output);
                }

                self.index += 1;
            }

            None
        }
    }

    pub type HandIndex = u8;
    pub const MAX_HAND_INDEX: u8 = MAX_PLAYERS - 1;

    pub fn gen_hand_index(rng: &mut Xs, player_count: HandLen) -> HandIndex {
        xs::range(rng, 0..player_count.u8() as _) as HandIndex
    }

    #[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
    pub enum HandLen {
        #[default]
        Two,
        Three,
        Four,
        Five,
        Six,
        Seven,
        Eight,
        Nine,
        Ten,
        Eleven,
        Twelve,
        Thirteen,
        Fourteen,
        Fifteen,
        Sixteen,
        Seventeen,
        Eightteen,
        Nineteen,
        Twenty,
        TwentyOne,
        TwentyTwo,
    }

    impl HandLen {
        pub fn saturating_add_1(self) -> Self {
            use HandLen::*;
            match self {
                Two => Three,
                Three => Four,
                Four => Five,
                Five => Six,
                Six => Seven,
                Seven => Eight,
                Eight => Nine,
                Nine => Ten,
                Ten => Eleven,
                Eleven => Twelve,
                Twelve => Thirteen,
                Thirteen => Fourteen,
                Fourteen => Fifteen,
                Fifteen => Sixteen,
                Sixteen => Seventeen,
                Seventeen => Eightteen,
                Eightteen => Nineteen,
                Nineteen => Twenty,
                Twenty => TwentyOne,
                TwentyOne
                | TwentyTwo => TwentyTwo,
            }
        }

        pub fn saturating_sub_1(self) -> Self {
            use HandLen::*;
            match self {
                Two
                | Three => Two,
                Four => Three,
                Five => Four,
                Six => Five,
                Seven => Six,
                Eight => Seven,
                Nine => Eight,
                Ten => Nine,
                Eleven => Ten,
                Twelve => Eleven,
                Thirteen => Twelve,
                Fourteen => Thirteen,
                Fifteen => Fourteen,
                Sixteen => Fifteen,
                Seventeen => Sixteen,
                Eightteen => Seventeen,
                Nineteen => Eightteen,
                Twenty => Nineteen,
                TwentyOne => Twenty,
                TwentyTwo => TwentyOne,
            }
        }

        pub fn text(self) -> &'static str {
            use HandLen::*;
            match self {
                Two => "2",
                Three => "3",
                Four => "4",
                Five => "5",
                Six => "6",
                Seven => "7",
                Eight => "8",
                Nine => "9",
                Ten => "10",
                Eleven => "11",
                Twelve => "12",
                Thirteen => "13",
                Fourteen => "14",
                Fifteen => "15",
                Sixteen => "16",
                Seventeen => "17",
                Eightteen => "18",
                Nineteen => "19",
                Twenty => "20",
                TwentyOne => "21",
                TwentyTwo => "22",
            }
        }

        pub fn u8(self) -> u8 {
            use HandLen::*;
            match self {
                Two => 2,
                Three => 3,
                Four => 4,
                Five => 5,
                Six => 6,
                Seven => 7,
                Eight => 8,
                Nine => 9,
                Ten => 10,
                Eleven => 11,
                Twelve => 12,
                Thirteen => 13,
                Fourteen => 14,
                Fifteen => 15,
                Sixteen => 16,
                Seventeen => 17,
                Eightteen => 18,
                Nineteen => 19,
                Twenty => 20,
                TwentyOne => 21,
                TwentyTwo => 22,
            }
        }

        pub fn amount(self) -> PlayerAmount {
            self.u8()
        }

        pub fn usize(self) -> usize {
            usize::from(self.u8())
        }
    }

    impl TryFrom<u8> for HandLen {
        type Error = ();

        fn try_from(byte: u8) -> Result<Self, Self::Error> {
            use HandLen::*;
            match byte {
                2 => Ok(Two),
                3 => Ok(Three),
                4 => Ok(Four),
                5 => Ok(Five),
                6 => Ok(Six),
                7 => Ok(Seven),
                8 => Ok(Eight),
                9 => Ok(Nine),
                10 => Ok(Ten),
                11 => Ok(Eleven),
                12 => Ok(Twelve),
                13 => Ok(Thirteen),
                14 => Ok(Fourteen),
                15 => Ok(Fifteen),
                16 => Ok(Sixteen),
                17 => Ok(Seventeen),
                18 => Ok(Eightteen),
                19 => Ok(Nineteen),
                20 => Ok(Twenty),
                21 => Ok(TwentyOne),
                22 => Ok(TwentyTwo),
                _ => Err(())
            }
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct Hands {
        hands: PerPlayer<Hand>,
        len: HandLen,
    }

    impl Hands {
        pub fn iter(&self) -> impl Iterator<Item = Hand> {
            self.hands.into_iter().take(self.len.usize())
        }

        pub fn len(&self) -> HandLen {
            self.len
        }

        pub fn get(&self, index: HandIndex) -> Option<&Hand> {
            self.hands.get(usize::from(index))
        }
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub enum PotAction {
        #[default]
        Fold,
        Bet(Money)
    }

    #[derive(Clone, Debug)]
    pub struct Pot {
        // TODO? Is there a way to get a firm upper bound for the number of actions
        // per round? Maybe we could impose a (generous) raise limit then calculate
        // an upper bound based on that?
        // Assuming any heap allocations still make sense, since this will be append
        // only and then dropped all at once, an arena could work here to reduce the
        // number of allocations. Without any known speed concerns, or another use
        // case for an arena, bringing in that dependency doesn't currently seem
        // worth it.
        actions: PerPlayer<Vec<PotAction>>,
        player_count: HandLen,
        has_gone_this_round: PerPlayerBitset,
    }

    #[cfg(test)]
    impl Default for Pot {
        fn default() -> Pot {
            Pot {
                actions: <_>::default(),
                player_count: HandLen::Two,
                has_gone_this_round: <_>::default(),
            }
        }
    }

    impl Pot {
        pub fn with_capacity(player_count: HandLen, capacity: usize) -> Self {
            let mut output = Pot{
                actions: <_>::default(),
                player_count,
                has_gone_this_round: <_>::default(),
            };

            for vec in &mut output.actions {
                *vec = Vec::with_capacity(capacity);
            }

            output
        }

        pub fn push_bet(&mut self, index: HandIndex, bet: PotAction) {
            self.has_gone_this_round.set(index);
            self.actions[usize::from(index)].push(bet);
        }

        pub fn reset_for_new_round(&mut self) {
            self.has_gone_this_round = <_>::default();
        }

        pub fn has_folded(&self, index: HandIndex) -> bool {
            self.has_folded_i(usize::from(index))
        }

        fn has_folded_i(&self, index: usize) -> bool {
            self.actions[index].iter().any(|a| *a == PotAction::Fold)
        }
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum RoundOutcome {
        Undetermined,
        AdvanceToNext,
        /// This happens if everyone but one player folds.
        AwardNow(HandIndex),
    }

    impl Pot {
        pub fn round_outcome(&self, current_money: &PerPlayer<Money>) -> RoundOutcome {
            use RoundOutcome::*;

            if self.has_gone_this_round.len() < self.player_count.amount() {
                // Rounds aren't complete until everyone has had a chance to move.
                return Undetermined;
            }

            let amounts = self.amounts();
            let mut previous_amount = None;
            for i in 0..amounts.len() {
                // A player is all in or not playing, if they have 0 money left.
                if current_money[i] == Money::ZERO {
                    continue;
                }

                if self.has_folded_i(i) {
                    continue;
                }

                if let Some(previous) = previous_amount {
                    if previous != amounts[i] {
                        return Undetermined;
                    }
                } else {
                    previous_amount = Some(amounts[i]);
                }
            }

            let call_amount = self.call_amount();
            let is_complete = previous_amount
                .map(|amount| amount >= call_amount)
                // If everyone is all-in, then the round is done.
                .unwrap_or(true);

            if is_complete {
                // TODO? avoid this seemingly extra loop?
                let mut unfolded_count = 0;
                let mut last_unfolded_index = 0;

                for index in 0..MAX_HAND_INDEX {
                    let i = usize::from(index);

                    if amounts[i] == 0
                    && current_money[i] == Money::ZERO {
                        // All in with nothing? Must not be playing.
                        continue;
                    }

                    if !self.has_folded(index) {
                        unfolded_count += 1;
                        last_unfolded_index = index;
                    }
                }

                if unfolded_count == 1 {
                    AwardNow(last_unfolded_index)
                } else {
                    AdvanceToNext
                }
            } else {
                Undetermined
            }
        }

        pub fn total(&self) -> MoneyInner {
            self.amounts()
                .iter()
                .sum()
        }

        /// Returns the total that a player must have bet, in order to call
        /// including previous bets. That is, the maxiumum amount bet by any player.
        pub fn call_amount(&self) -> MoneyInner {
            self.amounts()
                .into_iter()
                .max()
                .unwrap_or_default()
        }

        pub fn eligibilities(
            &self,
            current_money: &PerPlayer<Money>,
        ) -> impl Iterator<Item = (PerPlayerBitset, MoneyInner)> {
            // A side pot exists if there is a higher amount than someone who is
            // still in, has bet. (TODO? filter out in-progress bets?)

            let mut amounts = self.amounts();

            let mut current_money_inner = [0; MAX_PLAYERS as usize];
            for (i, m) in current_money.iter().enumerate() {
                current_money_inner[i] = m.as_inner();
            }

            std::iter::from_fn(move || {
                loop {
                    if amounts == [0; MAX_PLAYERS as usize] {
                        return None
                    }

                    let mut min_all_in = MoneyInner::MAX;
                    for i in 0..amounts.len() {
                        // A player is all in if they have 0 money left,
                        // and actually bet something.
                        if current_money_inner[i] == 0 {
                            if amounts[i] > 0 && amounts[i] < min_all_in {
                                min_all_in = amounts[i];
                            }
                        }
                    }

                    let mut contributors = PerPlayerBitset::default();
                    let mut output: MoneyInner = 0;
                    for index in 0..MAX_PLAYERS {
                        let i = usize::from(index);
                        if amounts[i] > 0 {
                            contributors.set(index);
                            match amounts[i].checked_sub(min_all_in) {
                                Some(new_amount) => {
                                    output = output.saturating_add(min_all_in);
                                    amounts[i] = new_amount;
                                },
                                None => {
                                    output = output.saturating_add(amounts[i]);
                                    amounts[i] = 0;
                                }
                            }
                        }
                    }

                    if contributors.len() > 0 && output != 0 {
                        return Some((contributors, output))
                    }
                }
            })
        }

        pub fn individual_pots(
            &self,
            current_money: &PerPlayer<Money>,
        ) -> impl Iterator<Item = MoneyInner> {
            self.eligibilities(current_money).filter(
                |(contributors, money)| {
                    // Side pots with one player in them are "trivial" and not
                    // desired to be returned
                    contributors.len() > 1 && *money != 0
                }
            )
            .map(|(_, money)| money)
        }

        pub fn amount_for(&self, index: HandIndex) -> MoneyInner {
            // TODO? Avoid calculating the other players' amounts here?
            self.amounts()[usize::from(index)]
        }

        fn amounts(&self) -> PerPlayer<MoneyInner> {
            let mut outputs: PerPlayer<MoneyInner> = [0; MAX_PLAYERS as usize];
            for i in 0..MAX_PLAYERS as usize {
                let output = &mut outputs[i];
                for action in &self.actions[i] {
                    match action {
                        PotAction::Fold => break,
                        PotAction::Bet(bet) => {
                            *output = output.saturating_add(bet.as_inner());
                        }
                    }
                }
            }
            outputs
        }

        pub fn award(&mut self, winner: &mut Money) {
            for i in 0..MAX_PLAYERS as usize {
                for action in &mut self.actions[i] {
                    match action {
                        PotAction::Fold => break,
                        PotAction::Bet(ref mut bet) => {
                            MoneyMove {
                                from: bet,
                                to: winner,
                                amount: NonZeroMoneyInner::MAX
                            }.perform();
                        }
                    }
                }
            }
        }

        pub fn award_multiple(
            &mut self,
            moneys: &mut PerPlayer<Money>,
            iter: impl Iterator<Item = (PlayerIndex, MoneyInner)>
        ) {
            // Collect all the money into one pile
            let mut pile: Money = Money::ZERO;
            for i in 0..MAX_PLAYERS as usize {
                for action in &mut self.actions[i] {
                    match action {
                        PotAction::Fold => break,
                        PotAction::Bet(ref mut bet) => {
                            MoneyMove {
                                from: bet,
                                to: &mut pile,
                                amount: NonZeroMoneyInner::MAX
                            }.perform();
                        }
                    }
                }
            }

            // Pull out the amounts from the pile
            for (i, possibly_zero_amount) in iter {
                let Some(amount) = NonZeroMoneyInner::new(possibly_zero_amount)
                    else { continue };
                MoneyMove {
                    from: &mut pile,
                    to: &mut moneys[usize::from(i)],
                    amount,
                }.perform();
            }

            assert_eq!(pile.as_inner(), 0);
        }
    }

    // We delibrately don't want to make this operation convenient outside of tests,
    // because we want the total amount of money in a given game to remain constant.
    // This is why `Money` is a struct that doesn't implement `Copy` in the first 
    // place.
    #[cfg(test)]
    fn test_money_inner_to_money(inner: MoneyInner) -> Money {
        let mut arr = Money::array_from_inner_array([inner]);

        arr[0].take_all()
    }

    #[cfg(test)]
    mod call_amount_works {
        use super::*;
        #[derive(Debug)]
        struct Spec {
            bet: Money,
            is_all_in: bool,
        }

        fn bet(bet: MoneyInner) -> Spec {
            Spec {
                bet: test_money_inner_to_money(bet),
                is_all_in: false,
            }
        }

        fn all_in(bet: MoneyInner) -> Spec {
            Spec {
                bet: test_money_inner_to_money(bet),
                is_all_in: true,
            }
        }

        // Short for assert
        macro_rules! a {
            ($specs: expr, $expected: expr) => {
                let specs = $specs;
                let expected = $expected;

                let mut pot = Pot::default();

                let mut moneys = [0; MAX_PLAYERS as usize];

                for (i, spec) in specs.into_iter().enumerate() {
                    pot.push_bet(
                        HandIndex::try_from(i).unwrap(),
                        PotAction::Bet(spec.bet),
                    );

                    moneys[i] = if spec.is_all_in {
                        0
                    } else {
                        1
                    };
                }

                let actual: MoneyInner = pot.call_amount();

                assert_eq!(actual, expected);
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], 10);
            a!([all_in(300), all_in(500)], 500);
            a!([all_in(300), all_in(500), all_in(800)], 800);
            a!([all_in(800), all_in(500), all_in(300)], 800);
            a!([all_in(500), all_in(300), all_in(800)], 800);
            a!([all_in(300), all_in(500), bet(800)], 800);
            a!([all_in(300), all_in(500), bet(800), bet(800)], 800);
            a!([all_in(300), all_in(500), bet(900), bet(900)], 900);
        }
    }

    #[cfg(test)]
    mod individual_pots_works {
        use super::*;
        #[derive(Debug)]
        struct Spec {
            action: PotAction,
            is_all_in: bool,
        }

        fn bet(bet: MoneyInner) -> Spec {
            Spec {
                action: PotAction::Bet(test_money_inner_to_money(bet)),
                is_all_in: false,
            }
        }

        fn all_in(bet: MoneyInner) -> Spec {
            Spec {
                action: PotAction::Bet(test_money_inner_to_money(bet)),
                is_all_in: true,
            }
        }

        fn fold() -> Spec {
            Spec {
                action: PotAction::Fold,
                is_all_in: false,
            }
        }

        // Short for assert
        macro_rules! a {
            ($specs: expr, $expected: expr) => {
                let specs = $specs;
                let expected = $expected;

                let mut pot = Pot::default();

                let mut moneys = [0; MAX_PLAYERS as usize];

                for (i, spec) in specs.into_iter().enumerate() {
                    pot.push_bet(
                        HandIndex::try_from(i).unwrap(),
                        spec.action,
                    );

                    moneys[i] = if spec.is_all_in {
                        0
                    } else {
                        1
                    };
                }

                let moneys = Money::array_from_inner_array(moneys);

                let actual: Vec<MoneyInner> = pot.individual_pots(&moneys).collect();

                let expected: Vec<MoneyInner> = expected.into_iter().collect();

                assert_eq!(actual, expected);
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], [15]);
            a!([all_in(300), all_in(500)], [600]);
            a!([all_in(300), all_in(500), all_in(800)], [900, 400]);
            a!([all_in(800), all_in(500), all_in(300)], [900, 400]);
            a!([all_in(500), all_in(300), all_in(800)], [900, 400]);
            a!([all_in(300), all_in(500), bet(800)], [900, 400]);
            a!([all_in(300), all_in(500), bet(800), bet(800)], [300 * 4, 200 * 3, 300 * 2]);
            a!([all_in(300), all_in(500), bet(900), bet(900)], [300 * 4, 200 * 3, 400 * 2]);
            a!([bet(5), bet(10), fold()], [15]);
            a!([all_in(300), fold(), all_in(500), fold(), bet(800)], [900, 400]);
        }
    }

    #[cfg(test)]
    mod is_round_complete_works {
        use super::{*, RoundOutcome::*};

        #[derive(Debug)]
        struct Spec {
            action: PotAction,
            is_all_in: bool,
        }

        fn bet(bet: MoneyInner) -> Spec {
            Spec {
                action: PotAction::Bet(test_money_inner_to_money(bet)),
                is_all_in: false,
            }
        }

        fn all_in(bet: MoneyInner) -> Spec {
            Spec {
                action: PotAction::Bet(test_money_inner_to_money(bet)),
                is_all_in: true,
            }
        }

        fn fold() -> Spec {
            Spec {
                action: PotAction::Fold,
                is_all_in: false,
            }
        }

        // Short for assert
        macro_rules! a {
            ($specs: expr, $expected: expr) => {
                let specs = $specs;
                let expected = $expected;

                let specs_string = format!("{specs:?}");

                let mut pot = Pot::default();

                let mut moneys = [0; MAX_PLAYERS as usize];

                for (i, spec) in specs.into_iter().enumerate() {
                    pot.push_bet(
                        HandIndex::try_from(i).unwrap(),
                        spec.action,
                    );

                    moneys[i] = if spec.is_all_in {
                        0
                    } else {
                        1
                    };
                }

                let moneys = Money::array_from_inner_array(moneys);

                let actual = pot.round_outcome(&moneys);

                assert_eq!(actual, expected, "{specs_string:?}");
            }
        }

        #[test]
        fn on_these_examples() {
            a!([bet(5), bet(10)], Undetermined);
            a!([all_in(300), all_in(500)], AdvanceToNext);
            a!([all_in(300), all_in(500), all_in(800)], AdvanceToNext);
            a!([all_in(800), all_in(500), all_in(300)], AdvanceToNext);
            a!([all_in(500), all_in(300), all_in(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(300)], Undetermined);
            a!([all_in(300), all_in(500), bet(800), bet(800)], AdvanceToNext);
            a!([all_in(300), all_in(500), bet(900), bet(900)], AdvanceToNext);
            a!([bet(5), bet(10), fold()], Undetermined);
            a!([all_in(300), fold(), all_in(500), fold(), bet(800)], AdvanceToNext);
            a!([all_in(300), fold(), fold()], AwardNow(0));
            a!([fold(), all_in(300), fold()], AwardNow(1));
            a!([fold(), fold(), all_in(300)], AwardNow(2));
        }
    }


    pub fn deal(
        rng: &mut Xs,
        player_count: HandLen,
    ) -> (Hands, Deck) {
        let mut deck = gen_deck(rng);

        let mut hands = Hands::default();

        let count = player_count.usize();

        for hand in (&mut hands.hands[0..count]).iter_mut() {
            let (Some(card1), Some(card2)) = (deck.draw(), deck.draw())
                else { continue };
            *hand = [card1, card2];
        }

        hands.len = player_count;

        (hands, deck)
    }

    pub type Flop = [Card; 3];

    #[derive(Clone, Copy)]
    pub enum CommunityCards {
        Flop(Flop),
        Turn(Flop, Card),
        River(Flop, Card, Card),
    }

    impl Default for CommunityCards {
        fn default() -> Self {
            Self::Flop(<_>::default())
        }
    }

    impl CommunityCards {
        pub fn contains(&self, card: Card) -> bool {
            match *self {
                Self::Flop([c1, c2, c3]) =>
                    c1 == card
                    || c2 == card
                    || c3 == card,
                Self::Turn([c1, c2, c3], c4) =>
                    c1 == card
                    || c2 == card
                    || c3 == card
                    || c4 == card,
                Self::River([c1, c2, c3], c4, c5) =>
                    c1 == card
                    || c2 == card
                    || c3 == card
                    || c4 == card
                    || c5 == card,
            }
        }
    }

    pub fn deal_community_cards(deck: &mut Deck) -> Option<CommunityCards> {
        deck.burn();
        let [Some(card1), Some(card2), Some(card3)] =
            [deck.draw(), deck.draw(), deck.draw()]
            else {
                return None
            };
        Some(CommunityCards::Flop([card1, card2, card3]))
    }

    pub fn deal_to_community_cards(
        deck: &mut Deck,
        community_cards: &mut CommunityCards
    ) {
        match *community_cards {
            CommunityCards::Flop(flop) => {
                deck.burn();
                if let Some(turn) = deck.draw() {
                    *community_cards = CommunityCards::Turn(flop, turn);
                } else {
                    debug_assert!(false, "Ran out of cards for turn!");
                }
            },
            CommunityCards::Turn(flop, turn) => {
                deck.burn();
                if let Some(river) = deck.draw() {
                    *community_cards = CommunityCards::River(flop, turn, river);
                } else {
                    debug_assert!(false, "Ran out of cards for river!");
                }
            }
            CommunityCards::River(..) => {
                // Nothing left to deal out.
            }
        }
    }

    pub type FullBoard = [Card; 5];

    impl From<FullBoard> for CommunityCards {
        fn from(full_board: FullBoard) -> Self {
            Self::River(
                [
                    full_board[0],
                    full_board[1],
                    full_board[2],
                ],
                full_board[3],
                full_board[4],
            )
        }
    }
}

