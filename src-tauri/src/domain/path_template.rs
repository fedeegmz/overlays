/// Local date/time captured once per expansion so every variable in a string is consistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateParts {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

/// Expands OBS-style variables (`%CCYY %YY %MM %DD %hh %mm %ss %%`).
/// Unknown sequences and a trailing lone `%` are left unchanged.
pub fn expand(template: &str, at: &DateParts) -> String {
    let variables: [(&str, String); 8] = [
        ("CCYY", format!("{:04}", at.year)),
        ("YY", format!("{:02}", at.year.rem_euclid(100))),
        ("MM", format!("{:02}", at.month)),
        ("DD", format!("{:02}", at.day)),
        ("hh", format!("{:02}", at.hour)),
        ("mm", format!("{:02}", at.minute)),
        ("ss", format!("{:02}", at.second)),
        ("%", "%".to_string()),
    ];

    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(pos) = rest.find('%') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        match variables.iter().find(|(name, _)| after.starts_with(name)) {
            Some((name, value)) => {
                out.push_str(value);
                rest = &after[name.len()..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const AT: DateParts = DateParts {
        year: 2026,
        month: 3,
        day: 7,
        hour: 9,
        minute: 5,
        second: 2,
    };

    #[test]
    fn expands_full_timestamp() {
        assert_eq!(
            expand("%CCYY-%MM-%DD_%hh-%mm-%ss", &AT),
            "2026-03-07_09-05-02"
        );
    }

    #[test]
    fn expands_two_digit_year() {
        assert_eq!(expand("%YY", &AT), "26");
    }

    #[test]
    fn two_digit_year_is_zero_padded() {
        let at = DateParts { year: 2005, ..AT };
        assert_eq!(expand("%YY", &at), "05");
    }

    #[test]
    fn variables_are_case_sensitive() {
        let at = DateParts {
            hour: 14,
            minute: 30,
            ..AT
        };
        assert_eq!(expand("%hh%mm", &at), "1430");
        assert_eq!(expand("%HH%MM", &at), "%HH03");
    }

    #[test]
    fn escaped_percent_is_literal() {
        assert_eq!(expand("100%%", &AT), "100%");
    }

    #[test]
    fn escape_prevents_expansion() {
        assert_eq!(expand("%%CCYY", &AT), "%CCYY");
    }

    #[test]
    fn unknown_variable_is_left_as_is() {
        assert_eq!(expand("x-%Q-y", &AT), "x-%Q-y");
    }

    #[test]
    fn trailing_percent_is_left_as_is() {
        assert_eq!(expand("abc%", &AT), "abc%");
    }

    #[test]
    fn locale_variables_are_unsupported() {
        assert_eq!(expand("%A", &AT), "%A");
        assert_eq!(expand("%b%p", &AT), "%b%p");
    }

    #[test]
    fn longest_match_wins() {
        assert_eq!(expand("%CCYYx", &AT), "2026x");
    }

    #[test]
    fn handles_non_ascii_text() {
        assert_eq!(expand("ñ-%DD-é%", &AT), "ñ-07-é%");
    }
}
