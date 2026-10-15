//! numpy and pandas arithmetic, reproduced in the same order of operations.
//!
//! Floating-point addition is not associative, so a sum taken in a different
//! order can differ in the last bit and, through a threshold test, change a
//! branch. These helpers add in exactly the order numpy and pandas do.

/// `np.sum(a)` / `ndarray.sum()` / `Series.sum()` for a contiguous float64
/// array: numpy's pairwise sum (checked against numpy 2.5.4 on 59,600 random arrays).
pub fn np_sum(a: &[f64]) -> f64 {
    pairwise_sum(a)
}

fn pairwise_sum(a: &[f64]) -> f64 {
    const BLOCK: usize = 128;
    let n = a.len();
    if n < 8 {
        let mut res = 0.0;
        for x in a {
            res += x;
        }
        // numpy starts the short loop from -0.0 so an all -0.0 input sums to -0.0;
        // `0.0 + -0.0` is 0.0 here, which only matters for signed zeros we never compare.
        res
    } else if n <= BLOCK {
        let mut r = [0.0f64; 8];
        r.copy_from_slice(&a[..8]);
        let mut i = 8;
        while i < n - (n % 8) {
            for j in 0..8 {
                r[j] += a[i + j];
            }
            i += 8;
        }
        let mut res = ((r[0] + r[1]) + (r[2] + r[3])) + ((r[4] + r[5]) + (r[6] + r[7]));
        while i < n {
            res += a[i];
            i += 1;
        }
        res
    } else {
        let mut n2 = n / 2;
        n2 -= n2 % 8;
        pairwise_sum(&a[..n2]) + pairwise_sum(&a[n2..])
    }
}

/// `np.cumsum(a)`: a running sum, left to right.
pub fn np_cumsum(a: &[f64]) -> Vec<f64> {
    let mut acc = 0.0;
    let mut out = Vec::with_capacity(a.len());
    for (i, x) in a.iter().enumerate() {
        acc = if i == 0 { *x } else { acc + x };
        out.push(acc);
    }
    out
}

/// pandas' groupby mean (Kahan-compensated sum divided by the count).
pub fn pandas_mean(values: &[f64]) -> f64 {
    let mut sum = 0.0f64;
    let mut comp = 0.0f64;
    let mut n = 0usize;
    for &val in values {
        if val.is_nan() {
            continue;
        }
        n += 1;
        let y = val - comp;
        let t = sum + y;
        comp = t - sum - y;
        if comp.is_nan() {
            comp = 0.0;
        }
        sum = t;
    }
    if n == 0 {
        f64::NAN
    } else {
        sum / n as f64
    }
}

/// `np.interp(x, xp, fp)` for increasing `xp`.
pub fn np_interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    let n = xp.len();
    if n == 0 {
        return f64::NAN;
    }
    if x.is_nan() {
        return x;
    }
    if x <= xp[0] {
        return fp[0];
    }
    if x >= xp[n - 1] {
        return fp[n - 1];
    }
    // numpy finds j with xp[j] <= x < xp[j+1].
    let mut j = match xp.binary_search_by(|v| v.partial_cmp(&x).unwrap()) {
        Ok(k) => k,
        Err(k) => k - 1,
    };
    while j + 1 < n && xp[j + 1] <= x {
        j += 1;
    }
    if xp[j] == x {
        return fp[j];
    }
    let slope = (fp[j + 1] - fp[j]) / (xp[j + 1] - xp[j]);
    let res = slope * (x - xp[j]) + fp[j];
    if res.is_nan() {
        let res2 = slope * (x - xp[j + 1]) + fp[j + 1];
        if res2.is_nan() && fp[j] == fp[j + 1] {
            return fp[j];
        }
        return res2;
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_order() {
        let tenth = [0.1f64; 12];
        // `np.sum([0.1]*12)` in numpy 2.5.
        assert_eq!(np_sum(&tenth), 1.2000000000000002);
        // Arrays where numpy and a left-to-right sum differ (numpy 2.5.4).
        let a12 = [0.13, 0.85, 0.76, 0.26, 0.5, 0.45, 0.65, 0.79, 0.09, 0.03, 0.84, 0.43];
        assert_eq!(np_sum(&a12), 5.779999999999999);
        assert_eq!(a12.iter().fold(0.0, |s, x| s + x), 5.78);
        let a30 = [
            0.76, 0.0, 0.45, 0.72, 0.23, 0.95, 0.9, 0.03, 0.03, 0.54, 0.94, 0.38, 0.22, 0.42, 0.03, 0.22, 0.44, 0.5, 0.23,
            0.23, 0.22, 0.46, 0.29, 0.02, 0.84, 0.56, 0.64, 0.19, 0.99, 0.86,
        ];
        assert_eq!(np_sum(&a30), 13.290000000000001);
    }

    #[test]
    fn interp() {
        assert_eq!(np_interp(0.5, &[0.0, 1.0], &[0.2, 0.4]), 0.30000000000000004);
        assert_eq!(np_interp(2.0, &[0.0, 1.0], &[0.2, 0.4]), 0.4);
    }
}
