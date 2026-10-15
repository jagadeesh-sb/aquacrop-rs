//! The two roundings AquaCrop-OSPy uses, which differ at decimal ties.
//!
//! Python's `round(x, n)` on a Python float rounds the exact binary value
//! correctly, ties to even. `round(x, n)` on a numpy float64, and `np.round`,
//! compute `rint(x * 10**n) / 10**n`. The port uses whichever the Python code
//! used at each site, so the two implementations agree to the last bit.

/// `round(x)` with no digits, Python and numpy alike: ties to even.
pub fn round0(x: f64) -> f64 {
    x.round_ties_even()
}

/// numpy's `np.round(x, n)` for `n >= 0`.
pub fn np_round(x: f64, n: i32) -> f64 {
    let f = 10f64.powi(n);
    (x * f).round_ties_even() / f
}

/// Python's `round(x, n)` on a float: the exact value rounded to `n` decimals,
/// ties to even.
pub fn py_round(x: f64, n: usize) -> f64 {
    if !x.is_finite() {
        return x;
    }
    // Rust prints the exact binary value when asked for enough digits; 1100 covers every f64.
    let exact = format!("{:.1100}", x.abs());
    let (int_part, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let keep = &frac[..n.min(frac.len())];
    let rest = &frac[n.min(frac.len())..];
    let first = rest.as_bytes().first().copied().unwrap_or(b'0');
    let tail_nonzero = rest.bytes().skip(1).any(|b| b != b'0');
    let mut digits: Vec<u8> = int_part.bytes().chain(keep.bytes()).map(|b| b - b'0').collect();
    let round_up = match first {
        b'0'..=b'4' => false,
        b'6'..=b'9' => true,
        _ => tail_nonzero || digits.last().copied().unwrap_or(0) % 2 == 1,
    };
    if round_up {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, 1);
                break;
            }
            i -= 1;
            if digits[i] == 9 {
                digits[i] = 0;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let split = digits.len() - keep.len();
    let mut s = String::with_capacity(digits.len() + 2);
    if x.is_sign_negative() {
        s.push('-');
    }
    for (i, d) in digits.iter().enumerate() {
        if i == split {
            s.push('.');
        }
        s.push((b'0' + d) as char);
    }
    s.parse().unwrap_or(x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_rounding() {
        // Values checked against CPython 3.12.
        assert_eq!(py_round(2.675, 2), 2.67);
        assert_eq!(py_round(0.125, 2), 0.12);
        assert_eq!(py_round(0.375, 2), 0.38);
        assert_eq!(py_round(1.005, 2), 1.0);
        assert_eq!(py_round(0.30000000000000004, 2), 0.3);
        assert_eq!(py_round(-0.125, 2), -0.12);
        assert_eq!(py_round(9.995, 2), 9.99);
        assert_eq!(py_round(99.995, 2), 100.0);
        assert_eq!(py_round(0.0866 * 500f64.powf(0.35), 2), 0.76);
    }

    #[test]
    fn numpy_rounding() {
        assert_eq!(np_round(2.675, 2), 2.68); // 2.675 * 100 is exactly 267.5 in binary
        assert_eq!(np_round(0.125, 2), 0.12);
        assert_eq!(np_round(1.005, 2), 1.0);
        assert_eq!(round0(2.5), 2.0);
        assert_eq!(round0(3.5), 4.0);
    }
}
