/// Represents different types of electricity rate structures
#[derive(Debug, Clone, PartialEq)]
pub enum ElectricityRate {
    /// Fixed rate for all hours
    Fixed {
        /// The rate per unit of electricity
        rate: f64,
    },
    /// Tiered rate structure with different rates for different time periods
    Tiered {
        /// List of rate tiers
        tiers: Vec<RateTier>,
    },
}

/// Represents a single tier in a tiered rate structure
#[derive(Debug, Clone, PartialEq)]
pub struct RateTier {
    /// Name of the tier (e.g., "Peak", "Off-Peak", "Super Off-Peak")
    pub name: String,
    /// Rate per unit of electricity for this tier
    pub rate: f64,
    /// List of hour ranges when this tier applies
    pub hour_ranges: Vec<HourRange>,
}

/// Represents a time range when a rate tier applies
#[derive(Debug, Clone, PartialEq)]
pub struct HourRange {
    /// Starting hour (0-23)
    pub from: u8,
    /// Ending hour (0-23, exclusive)
    pub till: u8,
    /// Type of day this range applies to
    pub weekday_type: WeekdayType,
}

/// Represents the type of day for rate application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeekdayType {
    /// Monday through Friday
    Weekday,
    /// Saturday and Sunday
    Weekend,
}

impl ElectricityRate {
    /// Creates a new fixed electricity rate
    pub fn fixed(rate: f64) -> Self {
        Self::Fixed { rate }
    }

    /// Creates a new tiered electricity rate
    pub fn tiered(tiers: Vec<RateTier>) -> Self {
        Self::Tiered { tiers }
    }

    /// Converts the electricity rate to a vector of hourly rates for a single week
    /// Returns a Vec<f64> with 168 elements (24 hours × 7 days)
    /// The vector is organized as: [Mon 0h, Mon 1h, ..., Mon 23h, Tue 0h, ..., Sun 23h]
    pub fn to_weekly_hourly_rates(&self) -> Vec<f64> {
        let mut weekly_rates = Vec::with_capacity(168);

        // Days of the week: 0=Monday, 1=Tuesday, ..., 6=Sunday
        for day in 0..7 {
            let weekday_type = if day < 5 {
                WeekdayType::Weekday
            } else {
                WeekdayType::Weekend
            };

            for hour in 0..24 {
                let rate = self.get_rate_for_hour(hour, weekday_type);
                weekly_rates.push(rate);
            }
        }

        weekly_rates
    }

    /// Converts the electricity rate to a vector of hourly rates for the whole year
    /// Returns a Vec<f64> with 8760 elements (24 hours × 365 days)
    /// The vector is organized as: [Jan 1 0h, Jan 1 1h, ..., Dec 31 23h]
    pub fn to_yearly_hourly_rates(&self) -> Vec<f64> {
        let mut yearly_rates = Vec::with_capacity(8760);

        // Generate rates for each day of the year
        for day_of_year in 0..365 {
            let weekday_type = self.get_weekday_type_for_day_of_year(day_of_year);

            for hour in 0..24 {
                let rate = self.get_rate_for_hour(hour, weekday_type);
                yearly_rates.push(rate);
            }
        }

        yearly_rates
    }

    /// Gets the rate for a specific hour and day type
    fn get_rate_for_hour(&self, hour: u8, weekday_type: WeekdayType) -> f64 {
        match self {
            ElectricityRate::Fixed { rate } => *rate,
            ElectricityRate::Tiered { tiers } => {
                // Find the first tier that matches this hour and day type
                for tier in tiers {
                    if tier.matches_hour(hour, weekday_type) {
                        return tier.rate;
                    }
                }
                // If no tier matches, return 0.0 (or could panic/return error)
                0.0
            }
        }
    }

    /// Determines the weekday type for a given day of the year
    /// Assumes January 1st is a Monday (day 0)
    fn get_weekday_type_for_day_of_year(&self, day_of_year: u16) -> WeekdayType {
        // January 1st is day 0, which we assume is Monday
        // So day % 7 gives us: 0=Mon, 1=Tue, 2=Wed, 3=Thu, 4=Fri, 5=Sat, 6=Sun
        let day_of_week = day_of_year % 7;
        if day_of_week < 5 {
            WeekdayType::Weekday
        } else {
            WeekdayType::Weekend
        }
    }

    /// Validates that all weekend and weekday hours are covered exactly once
    /// Returns true if the rate structure is valid, false otherwise
    pub fn is_valid(&self) -> bool {
        match self {
            ElectricityRate::Fixed { .. } => {
                // Fixed rates are always valid as they cover all hours
                true
            }
            ElectricityRate::Tiered { tiers } => {
                // Check if all hours (0-23) are covered exactly once for both weekday types
                self.validate_weekday_coverage(tiers) && self.validate_weekend_coverage(tiers)
            }
        }
    }

    /// Validates that all weekday hours (0-23) are covered exactly once
    fn validate_weekday_coverage(&self, tiers: &[RateTier]) -> bool {
        let mut covered_hours = [false; 24];

        for tier in tiers {
            for hour_range in &tier.hour_ranges {
                if hour_range.weekday_type == WeekdayType::Weekday
                    && !self.mark_hours_covered(&mut covered_hours, hour_range)
                {
                    return false; // Overlapping hours detected
                }
            }
        }

        // Check if all hours are covered
        covered_hours.iter().all(|&covered| covered)
    }

    /// Validates that all weekend hours (0-23) are covered exactly once
    fn validate_weekend_coverage(&self, tiers: &[RateTier]) -> bool {
        let mut covered_hours = [false; 24];

        for tier in tiers {
            for hour_range in &tier.hour_ranges {
                if hour_range.weekday_type == WeekdayType::Weekend
                    && !self.mark_hours_covered(&mut covered_hours, hour_range)
                {
                    return false; // Overlapping hours detected
                }
            }
        }

        // Check if all hours are covered
        covered_hours.iter().all(|&covered| covered)
    }

    /// Marks hours as covered in the given array and returns false if any overlap is detected
    fn mark_hours_covered(&self, covered_hours: &mut [bool; 24], hour_range: &HourRange) -> bool {
        if hour_range.from > hour_range.till {
            // Wrapping range (e.g., 22:00 to 06:00)
            for hour in hour_range.from..24 {
                if covered_hours[hour as usize] {
                    return false; // Overlap detected
                }
                covered_hours[hour as usize] = true;
            }
            for hour in 0..hour_range.till {
                if covered_hours[hour as usize] {
                    return false; // Overlap detected
                }
                covered_hours[hour as usize] = true;
            }
        } else {
            // Normal range (e.g., 09:00 to 17:00)
            for hour in hour_range.from..hour_range.till {
                if covered_hours[hour as usize] {
                    return false; // Overlap detected
                }
                covered_hours[hour as usize] = true;
            }
        }
        true
    }
}

impl RateTier {
    /// Creates a new rate tier
    pub fn new(name: String, rate: f64, hour_ranges: Vec<HourRange>) -> Self {
        Self {
            name,
            rate,
            hour_ranges,
        }
    }

    /// Checks if this tier applies to the given hour and day type
    pub fn matches_hour(&self, hour: u8, weekday_type: WeekdayType) -> bool {
        self.hour_ranges
            .iter()
            .any(|range| range.matches_hour(hour, weekday_type))
    }
}

impl HourRange {
    /// Creates a new hour range
    pub fn new(from: u8, till: u8, weekday_type: WeekdayType) -> Self {
        Self {
            from,
            till,
            weekday_type,
        }
    }

    /// Checks if this hour range matches the given hour and day type
    pub fn matches_hour(&self, hour: u8, weekday_type: WeekdayType) -> bool {
        // First check if the weekday type matches
        if self.weekday_type != weekday_type {
            return false;
        }

        // Handle the case where the range wraps around midnight (e.g., 22:00 to 06:00)
        if self.from > self.till {
            // Wrapping range: from > till (e.g., 22:00 to 06:00)
            hour >= self.from || hour < self.till
        } else {
            // Normal range: from <= till (e.g., 09:00 to 17:00)
            hour >= self.from && hour < self.till
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_rate() {
        let rate = ElectricityRate::fixed(0.12);
        match rate {
            ElectricityRate::Fixed { rate: r } => assert_eq!(r, 0.12),
            _ => panic!("Expected Fixed rate"),
        }
    }

    #[test]
    fn test_tiered_rate() {
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![
                HourRange::new(9, 17, WeekdayType::Weekday),
                HourRange::new(10, 16, WeekdayType::Weekend),
            ],
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![
                HourRange::new(17, 9, WeekdayType::Weekday),
                HourRange::new(16, 10, WeekdayType::Weekend),
            ],
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        match rate {
            ElectricityRate::Tiered { tiers } => assert_eq!(tiers.len(), 2),
            _ => panic!("Expected Tiered rate"),
        }
    }

    #[test]
    fn test_fixed_rate_weekly_conversion() {
        let rate = ElectricityRate::fixed(0.15);
        let weekly_rates = rate.to_weekly_hourly_rates();

        // Should have 168 elements (24 hours × 7 days)
        assert_eq!(weekly_rates.len(), 168);

        // All rates should be the same (0.15)
        for &rate_value in &weekly_rates {
            assert_eq!(rate_value, 0.15);
        }
    }

    #[test]
    fn test_fixed_rate_yearly_conversion() {
        let rate = ElectricityRate::fixed(0.15);
        let yearly_rates = rate.to_yearly_hourly_rates();

        // Should have 8760 elements (24 hours × 365 days)
        assert_eq!(yearly_rates.len(), 8760);

        // All rates should be the same (0.15)
        for &rate_value in &yearly_rates {
            assert_eq!(rate_value, 0.15);
        }
    }

    #[test]
    fn test_tiered_rate_weekly_conversion() {
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekday)],
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![
                HourRange::new(17, 9, WeekdayType::Weekday),
                HourRange::new(0, 24, WeekdayType::Weekend),
            ],
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        let weekly_rates = rate.to_weekly_hourly_rates();

        // Should have 168 elements
        assert_eq!(weekly_rates.len(), 168);

        // Check weekday rates (Monday-Friday, indices 0-119)
        for day in 0..5 {
            for hour in 0..24 {
                let index = day * 24 + hour;
                if hour >= 9 && hour < 17 {
                    // Peak hours
                    assert_eq!(weekly_rates[index], 0.25);
                } else {
                    // Off-peak hours
                    assert_eq!(weekly_rates[index], 0.08);
                }
            }
        }

        // Check weekend rates (Saturday-Sunday, indices 120-167)
        for day in 5..7 {
            for hour in 0..24 {
                let index = day * 24 + hour;
                // All weekend hours should be off-peak
                assert_eq!(weekly_rates[index], 0.08);
            }
        }
    }

    #[test]
    fn test_hour_range_matching() {
        // Test normal range (9:00 to 17:00)
        let range = HourRange::new(9, 17, WeekdayType::Weekday);

        // Should match hours 9-16
        for hour in 9..17 {
            assert!(range.matches_hour(hour, WeekdayType::Weekday));
        }

        // Should not match hours outside range
        for hour in 0..9 {
            assert!(!range.matches_hour(hour, WeekdayType::Weekday));
        }
        for hour in 17..24 {
            assert!(!range.matches_hour(hour, WeekdayType::Weekday));
        }

        // Should not match different weekday type
        assert!(!range.matches_hour(10, WeekdayType::Weekend));
    }

    #[test]
    fn test_wrapping_hour_range() {
        // Test wrapping range (22:00 to 06:00)
        let range = HourRange::new(22, 6, WeekdayType::Weekday);

        // Should match hours 22-23 and 0-5
        for hour in 22..24 {
            assert!(range.matches_hour(hour, WeekdayType::Weekday));
        }
        for hour in 0..6 {
            assert!(range.matches_hour(hour, WeekdayType::Weekday));
        }

        // Should not match hours 6-21
        for hour in 6..22 {
            assert!(!range.matches_hour(hour, WeekdayType::Weekday));
        }
    }

    #[test]
    fn test_weekday_type_determination() {
        let rate = ElectricityRate::fixed(0.1);

        // Test first few days of the year (assuming Jan 1 is Monday)
        // Day 0 (Jan 1) should be Monday (Weekday)
        let weekday_type = rate.get_weekday_type_for_day_of_year(0);
        assert_eq!(weekday_type, WeekdayType::Weekday);

        // Day 4 (Jan 5) should be Friday (Weekday)
        let weekday_type = rate.get_weekday_type_for_day_of_year(4);
        assert_eq!(weekday_type, WeekdayType::Weekday);

        // Day 5 (Jan 6) should be Saturday (Weekend)
        let weekday_type = rate.get_weekday_type_for_day_of_year(5);
        assert_eq!(weekday_type, WeekdayType::Weekend);

        // Day 6 (Jan 7) should be Sunday (Weekend)
        let weekday_type = rate.get_weekday_type_for_day_of_year(6);
        assert_eq!(weekday_type, WeekdayType::Weekend);

        // Day 7 (Jan 8) should be Monday (Weekday)
        let weekday_type = rate.get_weekday_type_for_day_of_year(7);
        assert_eq!(weekday_type, WeekdayType::Weekday);
    }

    #[test]
    fn test_fixed_rate_is_valid() {
        let rate = ElectricityRate::fixed(0.12);
        assert!(rate.is_valid());
    }

    #[test]
    fn test_valid_tiered_rate() {
        // Valid tiered rate with complete coverage
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekday)],
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![
                HourRange::new(17, 9, WeekdayType::Weekday), // Wrapping range for weekdays
                HourRange::new(0, 24, WeekdayType::Weekend), // All weekend hours
            ],
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        assert!(rate.is_valid());
    }

    #[test]
    fn test_invalid_tiered_rate_missing_weekday_hours() {
        // Invalid tiered rate missing some weekday hours
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekday)], // Only covers hours 9-16
        );

        let rate = ElectricityRate::tiered(vec![peak_tier]);
        assert!(!rate.is_valid()); // Missing weekday hours 0-8 and 17-23
    }

    #[test]
    fn test_invalid_tiered_rate_missing_weekend_hours() {
        // Invalid tiered rate missing weekend hours
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekend)], // Only covers weekend hours 9-16
        );

        let rate = ElectricityRate::tiered(vec![peak_tier]);
        assert!(!rate.is_valid()); // Missing weekend hours 0-8 and 17-23
    }

    #[test]
    fn test_invalid_tiered_rate_overlapping_hours() {
        // Invalid tiered rate with overlapping hours
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekday)],
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![HourRange::new(15, 20, WeekdayType::Weekday)], // Overlaps with peak tier
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        assert!(!rate.is_valid()); // Hours 15-16 are covered twice
    }

    #[test]
    fn test_valid_tiered_rate_with_wrapping_ranges() {
        // Valid tiered rate using wrapping ranges
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![
                HourRange::new(9, 17, WeekdayType::Weekday),
                HourRange::new(10, 16, WeekdayType::Weekend),
            ],
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![
                HourRange::new(17, 9, WeekdayType::Weekday), // Wrapping: 17-23 and 0-8
                HourRange::new(16, 10, WeekdayType::Weekend), // Wrapping: 16-23 and 0-9
            ],
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        assert!(rate.is_valid());
    }

    #[test]
    fn test_invalid_tiered_rate_wrapping_overlap() {
        // Invalid tiered rate with wrapping ranges that overlap
        let peak_tier = RateTier::new(
            "Peak".to_string(),
            0.25,
            vec![HourRange::new(22, 6, WeekdayType::Weekday)], // Wrapping: 22-23 and 0-5
        );

        let off_peak_tier = RateTier::new(
            "Off-Peak".to_string(),
            0.08,
            vec![HourRange::new(4, 8, WeekdayType::Weekday)], // Overlaps with peak at hours 4-5
        );

        let rate = ElectricityRate::tiered(vec![peak_tier, off_peak_tier]);
        assert!(!rate.is_valid()); // Hours 4-5 are covered twice
    }

    #[test]
    fn test_valid_tiered_rate_separate_weekday_weekend() {
        // Valid tiered rate with completely separate weekday and weekend coverage
        let weekday_peak = RateTier::new(
            "Weekday Peak".to_string(),
            0.25,
            vec![HourRange::new(9, 17, WeekdayType::Weekday)],
        );

        let weekday_off_peak = RateTier::new(
            "Weekday Off-Peak".to_string(),
            0.08,
            vec![HourRange::new(17, 9, WeekdayType::Weekday)],
        );

        let weekend_rate = RateTier::new(
            "Weekend Rate".to_string(),
            0.12,
            vec![HourRange::new(0, 24, WeekdayType::Weekend)],
        );

        let rate = ElectricityRate::tiered(vec![weekday_peak, weekday_off_peak, weekend_rate]);
        assert!(rate.is_valid());
    }
}
