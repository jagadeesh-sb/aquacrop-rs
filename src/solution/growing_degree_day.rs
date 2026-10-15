//! solution/growing_degree_day.py

#![allow(non_snake_case)]

/// Growing degree days for one day (method 1, 2 or 3 of AquaCrop).
pub fn growing_degree_day(GDDmethod: f64, Tupp: f64, Tbase: f64, temp_max: f64, temp_min: f64) -> f64 {
    let mut temp_max = temp_max;
    let mut temp_min = temp_min;
    let gdd;
    if GDDmethod == 1.0 {
        let mut Tmean = (temp_max + temp_min) / 2.0;
        Tmean = Tmean.min(Tupp);
        Tmean = Tmean.max(Tbase);
        gdd = Tmean - Tbase;
    } else if GDDmethod == 2.0 {
        temp_max = temp_max.min(Tupp);
        temp_max = temp_max.max(Tbase);
        temp_min = temp_min.min(Tupp);
        temp_min = temp_min.max(Tbase);
        let Tmean = (temp_max + temp_min) / 2.0;
        gdd = Tmean - Tbase;
    } else {
        // GDDmethod == 3.
        // Deviation: any other method leaves gdd unbound in Python (UnboundLocalError); read as 3 here.
        temp_max = temp_max.min(Tupp);
        temp_max = temp_max.max(Tbase);
        temp_min = temp_min.min(Tupp);
        let mut Tmean = (temp_max + temp_min) / 2.0;
        Tmean = Tmean.max(Tbase);
        gdd = Tmean - Tbase;
    }
    gdd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods() {
        // Method 1 clips the mean; method 2 clips each temperature first.
        assert_eq!(growing_degree_day(1.0, 30.0, 8.0, 34.0, 4.0), 11.0);
        assert_eq!(growing_degree_day(2.0, 30.0, 8.0, 34.0, 4.0), 11.0);
        assert_eq!(growing_degree_day(2.0, 30.0, 8.0, 34.0, 14.0), 14.0);
        assert_eq!(growing_degree_day(3.0, 30.0, 8.0, 10.0, 2.0), 0.0);
        assert_eq!(growing_degree_day(1.0, 30.0, 8.0, 10.0, 2.0), 0.0);
    }
}
