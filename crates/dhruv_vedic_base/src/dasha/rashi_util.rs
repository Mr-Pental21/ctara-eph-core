//! Rashi sign-type, parity, and counting utilities for dasha calculations.

/// Classification of rashis by quality (cardinal/fixed/mutable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignType {
    /// Chara (cardinal): Mesha, Karka, Tula, Makara.
    Chara,
    /// Sthira (fixed): Vrishabha, Simha, Vrischika, Kumbha.
    Sthira,
    /// Dvisvabhava (dual/mutable): Mithuna, Kanya, Dhanu, Meena.
    Dvisvabhava,
}

/// Get the sign type of a rashi by 0-based index.
pub fn sign_type(rashi_index: u8) -> SignType {
    match rashi_index % 3 {
        0 => SignType::Chara,
        1 => SignType::Sthira,
        _ => SignType::Dvisvabhava,
    }
}

/// Check if a rashi is odd-signed (1-indexed: 1,3,5,7,9,11 = Mesha..Kumbha).
/// Odd signs: Mesha(0), Mithuna(2), Simha(4), Tula(6), Dhanu(8), Kumbha(10).
pub fn is_odd_sign(rashi_index: u8) -> bool {
    rashi_index.is_multiple_of(2)
}

/// Count signs forward from `from` to `to` (inclusive of `to`, exclusive of `from`).
/// Result is 1-12: same sign = 1 if `from == to`.
///
/// Note: returns the distance counting forward through the zodiac.
pub fn count_signs_forward(from: u8, to: u8) -> u8 {
    let f = from % 12;
    let t = to % 12;
    if t >= f { t - f + 1 } else { 12 - f + t + 1 }
}

/// Count signs in reverse from `from` to `to`.
/// Result is 1-12: same sign = 1 if `from == to`.
pub fn count_signs_reverse(from: u8, to: u8) -> u8 {
    let f = from % 12;
    let t = to % 12;
    if f >= t { f - t + 1 } else { 12 - t + f + 1 }
}

/// Next rashi in zodiacal order (+1, wrapping).
pub fn next_rashi(rashi_index: u8) -> u8 {
    (rashi_index + 1) % 12
}

/// Jump rashi by offset (can be negative via wrapping).
pub fn jump_rashi(rashi_index: u8, offset: i8) -> u8 {
    ((rashi_index as i16 + offset as i16).rem_euclid(12)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_types() {
        // Mesha=0 (Chara), Vrishabha=1 (Sthira), Mithuna=2 (Dvi)
        assert_eq!(sign_type(0), SignType::Chara);
        assert_eq!(sign_type(1), SignType::Sthira);
        assert_eq!(sign_type(2), SignType::Dvisvabhava);
        // Karka=3 (Chara)
        assert_eq!(sign_type(3), SignType::Chara);
    }

    #[test]
    fn odd_signs() {
        assert!(is_odd_sign(0)); // Mesha
        assert!(!is_odd_sign(1)); // Vrishabha
        assert!(is_odd_sign(2)); // Mithuna
        assert!(is_odd_sign(10)); // Kumbha
        assert!(!is_odd_sign(11)); // Meena
    }

    #[test]
    fn count_forward_same() {
        assert_eq!(count_signs_forward(0, 0), 1);
    }

    #[test]
    fn count_forward_next() {
        assert_eq!(count_signs_forward(0, 1), 2);
    }

    #[test]
    fn count_forward_wrap() {
        assert_eq!(count_signs_forward(11, 0), 2);
    }

    #[test]
    fn count_reverse_same() {
        assert_eq!(count_signs_reverse(0, 0), 1);
    }

    #[test]
    fn count_reverse_wrap() {
        assert_eq!(count_signs_reverse(0, 11), 2);
    }

    #[test]
    fn next_rashi_wrap() {
        assert_eq!(next_rashi(11), 0);
        assert_eq!(next_rashi(0), 1);
    }

    #[test]
    fn jump_rashi_positive() {
        assert_eq!(jump_rashi(0, 3), 3);
    }

    #[test]
    fn jump_rashi_negative() {
        assert_eq!(jump_rashi(0, -3), 9);
    }
}
