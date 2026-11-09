fn display_version(version: &str) -> Result<String, &'static str> {
    let (date, counter) = version
        .split_once('+')
        .ok_or("release counter is missing")?;
    if counter.is_empty() || !counter.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("release counter must be a positive integer");
    }
    let counter = counter
        .parse::<u32>()
        .map_err(|_| "release counter is invalid")?;
    if counter == 0 {
        return Err("release counter must be positive");
    }
    let mut parts = date.split('.').map(str::parse::<u32>);
    let (Some(Ok(year)), Some(Ok(month)), Some(Ok(day)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err("release version must be YYYY.M.D+N");
    };
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 400 == 0 || (year % 4 == 0 && year % 100 != 0) => 29,
        2 => 28,
        _ => return Err("release month is invalid"),
    };
    if day == 0 || day > month_days {
        return Err("release day is invalid");
    }
    Ok(format!("{year:04}.{month:02}.{day:02}.{counter}"))
}

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").expect("Cargo package version is set");
    let display = display_version(&version).expect("Cargo version must encode YYYY.M.D+N");
    println!("cargo:rustc-env=PROSELINT_RS_RELEASE_VERSION={display}");
}

#[cfg(test)]
mod tests {
    use super::display_version;

    #[test]
    fn formats_semver_date_and_build_counter() {
        assert_eq!(
            display_version("2026.11.8+1"),
            Ok("2026.11.08.1".to_owned())
        );
        assert_eq!(
            display_version("2026.11.8+2"),
            Ok("2026.11.08.2".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_date_and_counter() {
        assert!(display_version("2026.2.29+1").is_err());
        assert!(display_version("2026.11.8+0").is_err());
        assert!(display_version("2026.11.8").is_err());
    }
}
