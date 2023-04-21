pub mod dates {
    use chrono::{Duration, NaiveDate};
    use regex::Regex;

    pub fn parse(s: &str) -> Result<NaiveDate, String> {
        let re = Regex::new("[0-9]+").unwrap();
        let parts: Vec<u32> = re
            .find_iter(s)
            .filter_map(|digits| digits.as_str().parse::<u32>().ok())
            .collect();
        return match parts.as_slice() {
            [month, day, year] => {
                return Ok(NaiveDate::from_ymd(
                    i32::try_from(*year).unwrap(),
                    *month,
                    *day,
                ));
            }
            _ => {
                println!("Could not parse a date from {}", s);
                Err(format!("Could not parse a date from {}", s))
            }
        };
    }
}