//! World state management - the central structure holding all game data.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::{Character, EntityId};

/// Unique identifier for locations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LocationId(pub Uuid);

impl LocationId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn nil() -> Self {
        Self(Uuid::nil())
    }
}

impl Default for LocationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LocationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for quests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuestId(pub Uuid);

impl QuestId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for QuestId {
    fn default() -> Self {
        Self::new()
    }
}

/// World time tracking.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct WorldTime {
    pub day: u32,
    pub hour: u8,
    pub minute: u8,
    pub season: Season,
}

impl WorldTime {
    /// Create a new world time.
    pub fn new(day: u32, hour: u8, minute: u8, season: Season) -> Self {
        Self {
            day,
            hour,
            minute,
            season,
        }
    }
}

/// Seasons of the year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Season {
    #[default]
    Spring,
    Summer,
    Autumn,
    Winter,
}

/// Weather conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Weather {
    #[default]
    Clear,
    Cloudy,
    Rainy,
    Stormy,
    Snowy,
    Foggy,
}

/// Visibility levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Visibility {
    Bright,
    #[default]
    Normal,
    Dim,
    Dark,
    MagicalDarkness,
}

/// Environment state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentState {
    pub weather: Weather,
    /// Temperature in Celsius.
    pub temperature: i32,
    pub visibility: Visibility,
    /// Ambient danger level from 0.0 to 1.0.
    pub ambient_danger_level: f32,
}

impl Default for EnvironmentState {
    fn default() -> Self {
        Self {
            weather: Weather::Clear,
            temperature: 20,
            visibility: Visibility::Normal,
            ambient_danger_level: 0.0,
        }
    }
}

/// Location types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationType {
    Wilderness,
    Town,
    Dungeon,
    Building,
    Special,
}

/// A location in the game world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub name: String,
    pub description: String,
    pub location_type: LocationType,
    pub connected_locations: Vec<LocationId>,
    /// Tags for knowledge base integration.
    pub ambient_tags: Vec<String>,
}

/// Flag value types for global state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FlagValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// The complete state of the game world at any point in time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorldState {
    /// Global time tracking.
    pub time: WorldTime,

    /// Current weather and environmental conditions.
    pub environment: EnvironmentState,

    /// All characters in the world (including player).
    pub characters: HashMap<EntityId, Character>,

    /// Location data.
    pub locations: HashMap<LocationId, Location>,

    /// Current location of each entity.
    pub entity_locations: HashMap<EntityId, LocationId>,

    /// Global flags and variables.
    pub global_flags: HashMap<String, FlagValue>,
}

impl WorldState {
    /// Create a new empty world state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get all entities at a specific location.
    pub fn entities_at_location(&self, location_id: LocationId) -> Vec<EntityId> {
        self.entity_locations
            .iter()
            .filter(|(_, loc)| **loc == location_id)
            .map(|(entity, _)| *entity)
            .collect()
    }

    /// Get character by ID.
    pub fn get_character(&self, id: EntityId) -> Option<&Character> {
        self.characters.get(&id)
    }

    /// Get mutable character by ID.
    pub fn get_character_mut(&mut self, id: EntityId) -> Option<&mut Character> {
        self.characters.get_mut(&id)
    }

    /// Check if it's currently night.
    pub fn is_night(&self) -> bool {
        self.time.hour < 6 || self.time.hour >= 20
    }

    /// Get the danger level at current time/weather.
    pub fn current_danger_level(&self) -> f32 {
        let base_danger = self.environment.ambient_danger_level;

        // Increase danger at night
        let time_modifier = if self.is_night() { 0.2 } else { 0.0 };

        // Increase danger in bad weather
        let weather_modifier = match self.environment.weather {
            Weather::Stormy => 0.15,
            Weather::Foggy => 0.1,
            _ => 0.0,
        };

        (base_danger + time_modifier + weather_modifier).min(1.0)
    }

    /// Advance time by given minutes.
    pub fn advance_time(&mut self, minutes: u32) {
        let total_minutes = self.time.minute as u32 + minutes;
        self.time.minute = (total_minutes % 60) as u8;

        let hours_passed = total_minutes / 60;
        let total_hours = self.time.hour as u32 + hours_passed;
        self.time.hour = (total_hours % 24) as u8;

        let days_passed = total_hours / 24;
        self.time.day += days_passed;

        // Update season every 90 days
        let season_day = self.time.day % 360;
        self.time.season = match season_day {
            0..=89 => Season::Spring,
            90..=179 => Season::Summer,
            180..=269 => Season::Autumn,
            _ => Season::Winter,
        };
    }

    /// Add a character to the world.
    pub fn add_character(&mut self, character: Character) -> EntityId {
        let id = character.id;
        self.characters.insert(id, character);
        id
    }

    /// Add a location to the world.
    pub fn add_location(&mut self, location: Location) -> LocationId {
        let id = location.id;
        self.locations.insert(id, location);
        id
    }

    /// Set entity location.
    pub fn set_entity_location(&mut self, entity_id: EntityId, location_id: LocationId) {
        self.entity_locations.insert(entity_id, location_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_time_is_night() {
        let mut state = WorldState::new();

        state.time.hour = 14;
        assert!(!state.is_night());

        state.time.hour = 22;
        assert!(state.is_night());

        state.time.hour = 4;
        assert!(state.is_night());
    }

    #[test]
    fn test_advance_time() {
        let mut state = WorldState::new();
        state.time = WorldTime::new(1, 23, 30, Season::Spring);

        state.advance_time(60); // Advance 1 hour

        assert_eq!(state.time.hour, 0);
        assert_eq!(state.time.minute, 30);
        assert_eq!(state.time.day, 2);
    }

    #[test]
    fn test_danger_level() {
        let mut state = WorldState::new();
        state.environment.ambient_danger_level = 0.3;

        // Day time, clear weather
        state.time.hour = 12;
        state.environment.weather = Weather::Clear;
        assert!((state.current_danger_level() - 0.3).abs() < 0.01);

        // Night time
        state.time.hour = 22;
        assert!((state.current_danger_level() - 0.5).abs() < 0.01);

        // Stormy weather at night
        state.environment.weather = Weather::Stormy;
        assert!((state.current_danger_level() - 0.65).abs() < 0.01);
    }

    #[test]
    fn test_add_character() {
        let mut state = WorldState::new();
        let character = Character::new("Test Hero");
        let id = character.id;

        state.add_character(character);

        assert!(state.get_character(id).is_some());
        assert_eq!(state.get_character(id).unwrap().name, "Test Hero");
    }
}
