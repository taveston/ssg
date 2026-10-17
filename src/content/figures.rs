//! Figures written to a number of significant figures, shared by the
//! generator's star facts and the view's readouts. It lives in content so
//! content never depends on the view; `view::readout` re-exports it beside
//! the unit formats.

/// `value` written to `figures` significant figures, with no exponent:
/// 0.000123, 0.523, 1.00, 12.3, 457.
pub fn significant(value: f64, figures: i32) -> String {
    if value == 0.0 {
        return "0".to_string();
    }
    let magnitude = value.abs().log10().floor() as i32;
    let decimals = (figures - 1 - magnitude).max(0) as usize;
    let text = format!("{value:.decimals$}");
    // Rounding up can add a digit (9.996 → "10.00"): write it again at
    // the next magnitude.
    let digits = text.chars().filter(char::is_ascii_digit).count();
    let leading_zeros = text
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take_while(|&c| c == '0')
        .count();
    if decimals > 0 && digits - leading_zeros > figures as usize {
        return format!("{value:.prec$}", prec = decimals - 1);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Three significant figures, whatever the magnitude (SYSG-10).
    #[test]
    fn figures_keep_three_significant_figures() {
        assert_eq!(significant(1.0, 3), "1.00");
        assert_eq!(significant(0.000123456, 3), "0.000123");
        assert_eq!(significant(12.345, 3), "12.3");
        assert_eq!(significant(457.2, 3), "457");
        assert_eq!(significant(9.996, 3), "10.0");
        assert_eq!(significant(0.09996, 3), "0.100");
        assert_eq!(significant(999.7, 3), "1000");
    }
}
