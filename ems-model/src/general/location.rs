use serde::{Deserialize, Serialize};

/// Supported countries for the EMS system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Country {
    Germany,
    Spain,
    Portugal,
}

impl Country {
    /// Get the ISO 3166-1 alpha-2 country code
    pub fn code(&self) -> &'static str {
        match self {
            Country::Germany => "DE",
            Country::Spain => "ES",
            Country::Portugal => "PT",
        }
    }

    /// Get the country name in English
    pub fn name(&self) -> &'static str {
        match self {
            Country::Germany => "Germany",
            Country::Spain => "Spain",
            Country::Portugal => "Portugal",
        }
    }

    /// Get the local name of the country
    pub fn local_name(&self) -> &'static str {
        match self {
            Country::Germany => "Deutschland",
            Country::Spain => "España",
            Country::Portugal => "Portugal",
        }
    }
}

/// Geographic coordinates for a location
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Coordinates {
    /// Latitude in decimal degrees (-90 to 90)
    pub latitude: f64,
    /// Longitude in decimal degrees (-180 to 180)
    pub longitude: f64,
}

impl Coordinates {
    /// Create new coordinates with validation
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(format!(
                "Invalid latitude: {}. Must be between -90 and 90",
                latitude
            ));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(format!(
                "Invalid longitude: {}. Must be between -180 and 180",
                longitude
            ));
        }

        Ok(Coordinates {
            latitude,
            longitude,
        })
    }
}

/// Address information for a location
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Address {
    /// Street name and number
    pub street: String,
    /// City or municipality
    pub city: String,
    /// State, province, or region
    pub region: Option<String>,
    /// Postal code
    pub postal_code: String,
    /// Additional address information
    pub additional_info: Option<String>,
}

impl Address {
    /// Create a new address
    pub fn new(
        street: String,
        city: String,
        region: Option<String>,
        postal_code: String,
        additional_info: Option<String>,
    ) -> Self {
        Address {
            street,
            city,
            region,
            postal_code,
            additional_info,
        }
    }

    /// Get formatted address string
    pub fn formatted(&self) -> String {
        let mut parts = vec![self.street.clone(), self.city.clone()];

        if let Some(ref region) = self.region {
            parts.push(region.clone());
        }

        parts.push(self.postal_code.clone());

        if let Some(ref additional) = self.additional_info {
            parts.push(additional.clone());
        }

        parts.join(", ")
    }
}

/// Complete location information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    /// Human-readable name for the location
    pub name: String,
    /// Country where the location is situated
    pub country: Country,
    /// Physical address
    pub address: Address,
    /// Geographic coordinates (serves as the unique identifier)
    pub coordinates: Coordinates,
}

impl Location {
    /// Create a new location
    pub fn new(name: String, country: Country, address: Address, coordinates: Coordinates) -> Self {
        Location {
            name,
            country,
            address,
            coordinates,
        }
    }

    /// Create a location with minimal information
    pub fn minimal(
        name: String,
        country: Country,
        city: String,
        postal_code: String,
        coordinates: Coordinates,
    ) -> Self {
        let address = Address::new(String::new(), city, None, postal_code, None);

        Location {
            name,
            country,
            address,
            coordinates,
        }
    }

    /// Check if location is in the same country as another location
    pub fn same_country(&self, other: &Location) -> bool {
        self.country == other.country
    }

    /// Get full display string for the location
    pub fn display(&self) -> String {
        format!(
            "{}, {}, {}",
            self.name,
            self.address.city,
            self.country.name()
        )
    }
}
