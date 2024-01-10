use chrono::Local;
use hl7_parser::LocationQuery;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct ValueMapFrom(pub LocationQuery);

impl std::ops::Deref for ValueMapFrom {
    type Target = LocationQuery;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum ValueMapTo {
    Auto,
    Now,
    Explicit(String),
}

#[derive(Debug, Clone)]
pub struct ValueMap {
    pub from: ValueMapFrom,
    pub to: ValueMapTo,
}

impl FromStr for ValueMap {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '=');
        let from = parts.next().ok_or_else(|| "missing from".to_string())?;
        let to = parts.next().ok_or_else(|| "missing to".to_string())?;
        let from = LocationQuery::from_str(from)?;
        let to = match to {
            "<auto>" => ValueMapTo::Auto,
            "<now>" => ValueMapTo::Now,
            s => ValueMapTo::Explicit(s.to_string()),
        };
        Ok(ValueMap {
            from: ValueMapFrom(from),
            to,
        })
    }
}

impl ValueMapTo {
    pub fn reify(&self, location: &LocationQuery) -> String {
        match self {
            ValueMapTo::Auto => {
                // TODO: generate based on defintion of field
                if *location == LocationQuery::new_field_repeat("MSH", 7, 1).unwrap() {
                    // message time
                    let now = Local::now();
                    now.format("%Y%m%d%H%M%S").to_string()
                } else if *location == LocationQuery::new_field_repeat("MSH", 10, 1).unwrap() {
                    // control ID
                    use rand::distributions::{Alphanumeric, DistString};
                    Alphanumeric.sample_string(&mut rand::thread_rng(), 20)
                } else {
                    use rand::distributions::{Alphanumeric, DistString};
                    Alphanumeric.sample_string(&mut rand::thread_rng(), 8)
                }
            }
            ValueMapTo::Now => {
                let now = Local::now();
                now.format("%Y%m%d%H%M%S").to_string()
            }
            ValueMapTo::Explicit(s) => s.clone(),
        }
    }
}

impl std::fmt::Display for ValueMapFrom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for ValueMapTo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueMapTo::Auto => write!(f, "<auto>"),
            ValueMapTo::Now => write!(f, "<now>"),
            ValueMapTo::Explicit(s) => write!(f, "{}", s),
        }
    }
}

impl std::fmt::Display for ValueMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{from}={to}", from = self.from, to = self.to)
    }
}
