use std::collections::HashMap;

/// heating demand in kWh/m2/year
pub struct HeatingNeed {
    pub national_minimum_requirement: f64,
    pub improved_standard: f64,
    pub ambitious_standard: f64,
}

impl HeatingNeed {
    pub fn new(
        national_minimum_requirement: f64,
        improved_standard: f64,
        ambitious_standard: f64,
    ) -> Self {
        Self {
            national_minimum_requirement,
            improved_standard,
            ambitious_standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuildingTypeEnum {
    SingleFamily,
    Terraced,
    MultiFamily,
    Apartment,
}

pub struct BuildingTypeMapping {
    pub mapping: HashMap<BuildingTypeEnum, HeatingNeed>,
}

impl BuildingTypeMapping {
    pub fn new() -> Self {
        let mapping = HashMap::new();
        Self { mapping }
    }

    pub fn get(&self, building_type: BuildingTypeEnum) -> Option<&HeatingNeed> {
        self.mapping.get(&building_type)
    }

    pub fn insert(&mut self, building_type: BuildingTypeEnum, heating_need: HeatingNeed) {
        self.mapping.insert(building_type, heating_need);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YearCategoryESEnum {
    Before1900,
    Between1901and1936,
    Between1937and1959,
    Between1960and1979,
    Between1980and2006,
    After2007,
}

pub struct YearCategoryESMapping {
    pub mapping: HashMap<YearCategoryESEnum, BuildingTypeMapping>,
}

impl YearCategoryESMapping {
    pub fn new() -> Self {
        let mapping = HashMap::new();
        Self { mapping }
    }

    pub fn get(&self, year_category: YearCategoryESEnum) -> Option<&BuildingTypeMapping> {
        self.mapping.get(&year_category)
    }

    pub fn insert(
        &mut self,
        year_category: YearCategoryESEnum,
        building_type_mapping: BuildingTypeMapping,
    ) {
        self.mapping.insert(year_category, building_type_mapping);
    }
}

impl Default for YearCategoryESMapping {
    fn default() -> Self {
        let mut mapping = HashMap::new();

        // Before1900
        let mut before1900 = BuildingTypeMapping::new();
        before1900.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(10.6, 10.7, 11.0),
        );
        before1900.insert(BuildingTypeEnum::Terraced, HeatingNeed::new(7.1, 4.0, 3.4));
        before1900.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(11.8, 6.1, 6.1),
        );
        before1900.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(7.8, 5.9, 5.6));
        mapping.insert(YearCategoryESEnum::Before1900, before1900);

        // Between1901and1936
        let mut between1901and1936 = BuildingTypeMapping::new();
        between1901and1936.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(14.8, 8.0, 7.1),
        );
        between1901and1936.insert(
            BuildingTypeEnum::Terraced,
            HeatingNeed::new(17.9, 11.7, 11.5),
        );
        between1901and1936.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(7.7, 4.9, 5.6),
        );
        between1901and1936.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(8.5, 4.5, 6.1));
        mapping.insert(YearCategoryESEnum::Between1901and1936, between1901and1936);

        // Between1937and1959
        let mut between1937and1959 = BuildingTypeMapping::new();
        between1937and1959.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(8.1, 4.1, 3.4),
        );
        between1937and1959.insert(
            BuildingTypeEnum::Terraced,
            HeatingNeed::new(20.7, 15.2, 15.2),
        );
        between1937and1959.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(11.3, 5.5, 5.1),
        );
        between1937and1959.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(7.4, 3.6, 3.1));
        mapping.insert(YearCategoryESEnum::Between1937and1959, between1937and1959);

        // Between1960and1979
        let mut between1960and1979 = BuildingTypeMapping::new();
        between1960and1979.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(12.4, 10.2, 9.1),
        );
        between1960and1979.insert(BuildingTypeEnum::Terraced, HeatingNeed::new(7.6, 5.0, 6.6));
        between1960and1979.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(9.8, 6.3, 6.0),
        );
        between1960and1979.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(4.3, 2.3, 2.3));
        mapping.insert(YearCategoryESEnum::Between1960and1979, between1960and1979);

        // Between1980and2006
        let mut between1980and2006 = BuildingTypeMapping::new();
        between1980and2006.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(5.8, 4.7, 5.7),
        );
        between1980and2006.insert(BuildingTypeEnum::Terraced, HeatingNeed::new(5.8, 5.4, 6.7));
        between1980and2006.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(3.9, 3.3, 2.8),
        );
        between1980and2006.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(2.3, 1.9, 3.5));
        mapping.insert(YearCategoryESEnum::Between1980and2006, between1980and2006);

        // After2007
        let mut after2007 = BuildingTypeMapping::new();
        after2007.insert(
            BuildingTypeEnum::SingleFamily,
            HeatingNeed::new(6.4, 2.9, 2.4),
        );
        after2007.insert(BuildingTypeEnum::Terraced, HeatingNeed::new(2.5, 2.2, 1.9));
        after2007.insert(
            BuildingTypeEnum::MultiFamily,
            HeatingNeed::new(3.5, 1.9, 1.5),
        );
        after2007.insert(BuildingTypeEnum::Apartment, HeatingNeed::new(2.4, 1.5, 1.2));
        mapping.insert(YearCategoryESEnum::After2007, after2007);

        Self { mapping }
    }
}

pub struct YearCategory {
    pub es: YearCategoryESMapping,
}
