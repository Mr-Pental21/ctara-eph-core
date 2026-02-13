//! Chebyshev polynomial evaluation via Clenshaw recurrence.
//!
//! Algorithms from "Numerical Recipes" and the Chebyshev polynomial
//! mathematical definition. Implementation is original.

/// Evaluate a Chebyshev expansion using the Clenshaw recurrence.
///
/// Given coefficients `[c_0, c_1, ..., c_{n-1}]` and normalised time
/// `s` in `[-1, 1]`, computes `sum(c_k * T_k(s))`.
pub fn clenshaw(coeffs: &[f64], s: f64) -> f64 {
    let n = coeffs.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return coeffs[0];
    }

    let mut b_k1 = 0.0; // b_{k+1}
    let mut b_k2 = 0.0; // b_{k+2}
    let two_s = 2.0 * s;

    for k in (1..n).rev() {
        let b_k = two_s * b_k1 - b_k2 + coeffs[k];
        b_k2 = b_k1;
        b_k1 = b_k;
    }

    // Final step: position = s * b_1 - b_2 + c_0
    s * b_k1 - b_k2 + coeffs[0]
}

/// Evaluate the derivative of a Chebyshev expansion.
///
/// Given coefficients `[c_0, c_1, ..., c_{n-1}]` and normalised time
/// `s` in `[-1, 1]`, computes `sum(c_k * T_k'(s))`.
///
/// Uses the forward recurrence for Chebyshev derivatives:
/// ```text
/// T_0'(s) = 0
/// T_1'(s) = 1
/// T_k'(s) = 2 * T_{k-1}(s) + 2 * s * T_{k-1}'(s) - T_{k-2}'(s)
/// ```
///
/// This also needs T_k(s) values, computed via the standard recurrence.
pub fn clenshaw_derivative(coeffs: &[f64], s: f64) -> f64 {
    let n = coeffs.len();
    if n <= 1 {
        return 0.0;
    }

    // Forward recurrence tracking both T_k(s) and T_k'(s).
    let two_s = 2.0 * s;

    // k = 0
    let mut t_prev2 = 1.0; // T_0(s)
    let mut dt_prev2 = 0.0; // T_0'(s)

    // k = 1
    let mut t_prev1 = s; // T_1(s)
    let mut dt_prev1 = 1.0; // T_1'(s)

    let mut result = coeffs[1]; // c_1 * T_1'(s) = c_1 * 1

    for &c_k in &coeffs[2..n] {
        let t_k = two_s * t_prev1 - t_prev2;
        let dt_k = 2.0 * t_prev1 + two_s * dt_prev1 - dt_prev2;

        result += c_k * dt_k;

        t_prev2 = t_prev1;
        t_prev1 = t_k;
        dt_prev2 = dt_prev1;
        dt_prev1 = dt_k;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-14;

    #[test]
    fn constant_polynomial() {
        // T_0(s) = 1 for all s, so [7.0] → 7.0
        assert!((clenshaw(&[7.0], 0.5) - 7.0).abs() < EPS);
        assert!(clenshaw_derivative(&[7.0], 0.5).abs() < EPS);
    }

    #[test]
    fn linear_polynomial() {
        // [a, b] → a * T_0(s) + b * T_1(s) = a + b*s
        let a = 3.0;
        let b = 5.0;
        let s = 0.7;
        let expected = a + b * s;
        assert!((clenshaw(&[a, b], s) - expected).abs() < EPS);

        // Derivative: b * T_1'(s) = b * 1 = b
        assert!((clenshaw_derivative(&[a, b], s) - b).abs() < EPS);
    }

    #[test]
    fn quadratic_polynomial() {
        // [a, b, c] → a*T_0 + b*T_1 + c*T_2
        // T_2(s) = 2s^2 - 1
        // T_2'(s) = 4s
        let a = 1.0;
        let b = 2.0;
        let c = 3.0;
        let s = 0.4;
        let t0 = 1.0;
        let t1 = s;
        let t2 = 2.0 * s * s - 1.0;
        let expected = a * t0 + b * t1 + c * t2;
        assert!((clenshaw(&[a, b, c], s) - expected).abs() < EPS);

        // Derivative: b*1 + c*4s
        let expected_d = b + c * 4.0 * s;
        assert!((clenshaw_derivative(&[a, b, c], s) - expected_d).abs() < EPS);
    }

    #[test]
    fn cubic_polynomial() {
        // T_3(s) = 4s^3 - 3s
        // T_3'(s) = 12s^2 - 3
        let coeffs = [1.0, 0.0, 0.0, 1.0]; // just T_0 + T_3
        let s = 0.6;
        let t3 = 4.0 * s * s * s - 3.0 * s;
        let expected = 1.0 + t3;
        assert!((clenshaw(&coeffs, s) - expected).abs() < EPS);

        let dt3 = 12.0 * s * s - 3.0;
        assert!((clenshaw_derivative(&coeffs, s) - dt3).abs() < EPS);
    }

    #[test]
    fn empty_coefficients() {
        assert_eq!(clenshaw(&[], 0.5), 0.0);
        assert_eq!(clenshaw_derivative(&[], 0.5), 0.0);
    }

    #[test]
    fn evaluation_at_boundaries() {
        // T_k(1) = 1 for all k, T_k(-1) = (-1)^k
        let coeffs = [2.0, 3.0, 5.0];
        let at_one = 2.0 * 1.0 + 3.0 * 1.0 + 5.0 * 1.0;
        assert!((clenshaw(&coeffs, 1.0) - at_one).abs() < EPS);

        let at_neg_one = 2.0 - 3.0 + 5.0;
        assert!((clenshaw(&coeffs, -1.0) - at_neg_one).abs() < EPS);
    }
}
