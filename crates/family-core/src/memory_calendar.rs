use chrono::{Datelike, Local, NaiveDate};
use serde_json::Value;

/// Returns whether a capture belongs in today's local-calendar memory tray.
/// February 29 captures surface on February 28 during non-leap years.
pub fn is_earlier_anniversary(captured_at: i64, today: NaiveDate) -> bool {
    let Some(captured) = chrono::DateTime::<chrono::Utc>::from_timestamp(captured_at, 0)
        .map(|date| date.with_timezone(&Local).date_naive())
    else {
        return false;
    };
    if captured.year() >= today.year() {
        return false;
    }

    let same_day = captured.month() == today.month() && captured.day() == today.day();
    let leap_day_on_february_28 = today.month() == 2
        && today.day() == 28
        && NaiveDate::from_ymd_opt(today.year(), 2, 29).is_none()
        && captured.month() == 2
        && captured.day() == 29;
    same_day || leap_day_on_february_28
}

pub fn asset_ids(assets: &[Value], today: NaiveDate) -> Vec<String> {
    let mut memories: Vec<_> = assets
        .iter()
        .filter_map(|asset| {
            if !matches!(asset["kind"].as_str(), Some("photo" | "video"))
                || asset["deleted"].as_bool().unwrap_or(false)
            {
                return None;
            }
            let captured_at = asset["captured_at"].as_i64()?;
            if !is_earlier_anniversary(captured_at, today) {
                return None;
            }
            Some((captured_at, asset["id"].as_str()?.to_owned()))
        })
        .collect();
    memories.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    memories.into_iter().map(|(_, asset_id)| asset_id).collect()
}

#[cfg(test)]
mod tests {
    use super::is_earlier_anniversary;
    use chrono::{Local, NaiveDate, TimeZone};
    use serde_json::json;

    fn local_noon(year: i32, month: u32, day: u32) -> i64 {
        Local
            .with_ymd_and_hms(year, month, day, 12, 0, 0)
            .single()
            .expect("valid local capture date")
            .timestamp()
    }

    #[test]
    fn anniversaries_use_local_dates_and_include_leap_day_on_february_28() {
        let ordinary = local_noon(2023, 2, 28);
        let leap = local_noon(2024, 2, 29);
        let later_day = local_noon(2023, 3, 1);
        assert!(is_earlier_anniversary(
            ordinary,
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        ));
        assert!(is_earlier_anniversary(
            leap,
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        ));
        assert!(!is_earlier_anniversary(
            leap,
            NaiveDate::from_ymd_opt(2024, 2, 28).unwrap()
        ));
        assert!(!is_earlier_anniversary(
            later_day,
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        ));
        assert!(!is_earlier_anniversary(
            ordinary,
            NaiveDate::from_ymd_opt(2023, 2, 28).unwrap()
        ));
    }

    #[test]
    fn on_this_day_asset_list_omits_deleted_non_media_and_current_year_items() {
        let older = local_noon(2023, 5, 14);
        let newer = local_noon(2024, 5, 14);
        let today_timestamp = local_noon(2025, 5, 14);
        let assets = vec![
            json!({"id":"older","kind":"photo","captured_at":older,"deleted":false}),
            json!({"id":"newer","kind":"video","captured_at":newer,"deleted":false}),
            json!({"id":"deleted","kind":"photo","captured_at":newer,"deleted":true}),
            json!({"id":"audio","kind":"audio","captured_at":newer,"deleted":false}),
            json!({"id":"today","kind":"photo","captured_at":today_timestamp,"deleted":false}),
        ];
        assert_eq!(
            super::asset_ids(&assets, NaiveDate::from_ymd_opt(2025, 5, 14).unwrap()),
            ["newer", "older"]
        );
    }
}
