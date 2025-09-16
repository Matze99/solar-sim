use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Represents monthly energy demand in kWh
#[derive(Debug, Clone)]
pub struct MonthlyDemand {
    pub january: f64,
    pub february: f64,
    pub march: f64,
    pub april: f64,
    pub may: f64,
    pub june: f64,
    pub july: f64,
    pub august: f64,
    pub september: f64,
    pub october: f64,
    pub november: f64,
    pub december: f64,
}

impl MonthlyDemand {
    pub fn get_monthly_demand(&self, month: u32) -> f64 {
        match month {
            1 => self.january,
            2 => self.february,
            3 => self.march,
            4 => self.april,
            5 => self.may,
            6 => self.june,
            7 => self.july,
            8 => self.august,
            9 => self.september,
            10 => self.october,
            11 => self.november,
            12 => self.december,
            _ => panic!("Invalid month: {}", month),
        }
    }
}

/// Loads hourly energy demand data from CSV file
///
/// # Arguments
/// * `file_path` - Path to the CSV file containing hourly energy demand in Wh
///
/// # Returns
/// * Vector of hourly energy demand values in kWh
pub fn load_hourly_demand(file_path: &str) -> Result<Vec<f64>> {
    let file =
        File::open(file_path).with_context(|| format!("Failed to open file: {}", file_path))?;

    let reader = BufReader::new(file);
    let mut hourly_demand = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Failed to read line {}", line_num + 1))?;
        let trimmed = line.trim();
        // Remove any non-numeric, non-decimal, non-minus characters
        let cleaned: String = trimmed
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == ',')
            .collect();
        if line_num < 5 {
            println!(
                "Line {}: '{}' (cleaned: '{}', length: {})",
                line_num + 1,
                trimmed,
                cleaned,
                cleaned.len()
            );
        }
        let value: f64 = if cleaned.contains(',') {
            cleaned.replace(',', ".").parse::<f64>().with_context(|| {
                format!(
                    "Failed to parse value on line {}: '{}' (cleaned: '{}')",
                    line_num + 1,
                    trimmed,
                    cleaned
                )
            })?
        } else {
            cleaned.parse::<f64>().with_context(|| {
                format!(
                    "Failed to parse value on line {}: '{}' (cleaned: '{}')",
                    line_num + 1,
                    trimmed,
                    cleaned
                )
            })?
        };
        hourly_demand.push(value / 1000.0);
    }

    Ok(hourly_demand)
}

/// Generates a scaled hourly load curve based on monthly demand totals
///
/// # Arguments
/// * `monthly_demand` - HashMap with month (1-12) as key and total monthly demand in kWh as value
/// * `base_hourly_demand` - Vector of hourly energy demand values in kWh (8760 hours for a year)
///
/// # Returns
/// * Vector of scaled hourly energy demand values in kWh
pub fn generate_scaled_load_curve(
    monthly_demand: &MonthlyDemand,
    base_hourly_demand: &[f64],
) -> Result<Vec<f64>> {
    if base_hourly_demand.len() != 8760 {
        return Err(anyhow::anyhow!(
            "Base hourly demand must contain exactly 8760 hours (one year), got {}",
            base_hourly_demand.len()
        ));
    }

    // Calculate total energy in base hourly demand (for potential future use)
    let _base_total_energy: f64 = base_hourly_demand.iter().sum();

    // Define hours per month (assuming non-leap year)
    let hours_per_month = [744, 672, 744, 720, 744, 720, 744, 744, 720, 744, 720, 744];

    let mut scaled_demand = Vec::with_capacity(8760);
    let mut hour_index = 0;

    for month in 1..=12 {
        let target_monthly_energy = monthly_demand.get_monthly_demand(month);

        let month_hours = hours_per_month[month as usize - 1];
        let month_start = hour_index;
        let month_end = month_start + month_hours;

        // Calculate total energy for this month in base data
        let base_monthly_energy: f64 = base_hourly_demand[month_start..month_end].iter().sum();

        // Calculate scaling factor for this month
        let scaling_factor = if base_monthly_energy > 0.0 {
            target_monthly_energy / base_monthly_energy
        } else {
            0.0
        };

        // Scale each hour in this month
        for &hourly_value in &base_hourly_demand[month_start..month_end] {
            scaled_demand.push(hourly_value * scaling_factor);
        }

        hour_index += month_hours;
    }

    Ok(scaled_demand)
}

/// Convenience function that loads the base hourly demand from CSV and generates scaled load curve
///
/// # Arguments
/// * `monthly_demand` - HashMap with month (1-12) as key and total monthly demand in kWh as value
/// * `csv_file_path` - Path to the CSV file containing base hourly energy demand in Wh
///
/// # Returns
/// * Vector of scaled hourly energy demand values in kWh
pub fn create_scaled_load_curve_from_csv(
    monthly_demand: &MonthlyDemand,
    csv_file_path: &str,
) -> Result<Vec<f64>> {
    let base_hourly_demand = load_hourly_demand(csv_file_path)?;
    generate_scaled_load_curve(monthly_demand, &base_hourly_demand)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_hourly_demand() {
        // Create a temporary test file
        let test_data = "1000\n2000\n1500\n";
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(&temp_file, test_data).unwrap();

        let result = load_hourly_demand(temp_file.path().to_str().unwrap());
        assert!(result.is_ok());

        let hourly_demand = result.unwrap();
        assert_eq!(hourly_demand, vec![1.0, 2.0, 1.5]); // Converted from Wh to kWh
    }

    #[test]
    fn test_generate_scaled_load_curve() {
        // Create test monthly demand
        let monthly_demand = MonthlyDemand {
            january: 1000.0,
            february: 800.0,
            march: 1200.0,
            april: 1500.0,
            may: 1800.0,
            june: 2100.0,
            july: 2400.0,
            august: 2700.0,
            september: 3000.0,
            october: 3300.0,
            november: 3600.0,
            december: 3900.0,
        };

        // Create test base hourly demand (simplified for testing)
        let base_hourly_demand = vec![1.0; 8760]; // 1 kWh per hour for all hours

        let result = generate_scaled_load_curve(&monthly_demand, &base_hourly_demand);
        assert!(result.is_ok());

        let scaled_demand = result.unwrap();
        assert_eq!(scaled_demand.len(), 8760);

        // Check that January (744 hours) has the correct scaling
        let january_hours: f64 = scaled_demand[0..744].iter().sum();
        assert!((january_hours - 1000.0).abs() < 0.01);

        // Check that February (672 hours) has the correct scaling
        let february_hours: f64 = scaled_demand[744..1416].iter().sum();
        assert!((february_hours - 800.0).abs() < 0.01);
    }
}
