use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Nutrition {
    #[serde(default)]
    pub calories_kcal: f64,
    #[serde(default)]
    pub protein_g: f64,
    #[serde(default)]
    pub carbs_g: f64,
    #[serde(default)]
    pub fat_g: f64,
    #[serde(default)]
    pub fiber_g: f64,
}

impl Default for Nutrition {
    fn default() -> Self {
        Self {
            calories_kcal: 0.0,
            protein_g: 0.0,
            carbs_g: 0.0,
            fat_g: 0.0,
            fiber_g: 0.0,
        }
    }
}

impl Nutrition {
    pub fn add_scaled(&mut self, other: &Self, factor: f64) {
        self.calories_kcal += other.calories_kcal * factor;
        self.protein_g += other.protein_g * factor;
        self.carbs_g += other.carbs_g * factor;
        self.fat_g += other.fat_g * factor;
        self.fiber_g += other.fiber_g * factor;
    }

    pub fn divided_by(&self, divisor: f64) -> Self {
        if divisor <= 0.0 {
            return self.clone();
        }
        Self {
            calories_kcal: self.calories_kcal / divisor,
            protein_g: self.protein_g / divisor,
            carbs_g: self.carbs_g / divisor,
            fat_g: self.fat_g / divisor,
            fiber_g: self.fiber_g / divisor,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NutritionTargets {
    pub calories_kcal: Option<f64>,
    pub protein_g: Option<f64>,
    pub carbs_g: Option<f64>,
    pub fat_g: Option<f64>,
    pub fiber_g: Option<f64>,
}

impl Default for NutritionTargets {
    fn default() -> Self {
        Self {
            calories_kcal: None,
            protein_g: None,
            carbs_g: None,
            fat_g: None,
            fiber_g: Some(35.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub locale: String,
    pub timezone: String,
    pub household_size: i64,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    pub birth_date: Option<String>,
    pub sex_for_energy: Option<String>,
    pub activity_level: String,
    pub calorie_target_mode: String,
    pub manual_calorie_target_kcal: Option<f64>,
    pub dietary_style: Option<String>,
    pub nutrition_targets: NutritionTargets,
    pub weekday_max_minutes: i64,
    pub weekend_max_minutes: i64,
    pub default_servings: f64,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfilePatch {
    pub name: Option<String>,
    pub locale: Option<String>,
    pub timezone: Option<String>,
    pub household_size: Option<i64>,
    pub height_cm: Option<Option<f64>>,
    pub weight_kg: Option<Option<f64>>,
    pub birth_date: Option<Option<String>>,
    pub sex_for_energy: Option<Option<String>>,
    pub activity_level: Option<String>,
    pub calorie_target_mode: Option<String>,
    pub manual_calorie_target_kcal: Option<Option<f64>>,
    pub dietary_style: Option<Option<String>>,
    pub nutrition_targets: Option<NutritionTargets>,
    pub weekday_max_minutes: Option<i64>,
    pub weekend_max_minutes: Option<i64>,
    pub default_servings: Option<f64>,
    pub notes: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Equipment {
    pub id: String,
    pub name: String,
    pub category: String,
    pub available: bool,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeIngredient {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub recipe_id: String,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
    #[serde(default)]
    pub aisle: String,
    pub preparation: Option<String>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub nutrition: Nutrition,
    #[serde(default)]
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeStep {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub recipe_id: String,
    #[serde(default)]
    pub position: i64,
    pub instruction: String,
    pub timer_minutes: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSource {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub recipe_id: String,
    pub title: String,
    pub url: Option<String>,
    pub publisher: Option<String>,
    #[serde(default)]
    pub source_type: String,
    pub accessed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeImage {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub recipe_id: String,
    pub url: String,
    #[serde(default)]
    pub kind: String,
    pub alt_text: Option<String>,
    pub attribution: Option<String>,
    #[serde(default)]
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    #[serde(default)]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub summary: String,
    pub servings: f64,
    pub prep_minutes: i64,
    pub cook_minutes: i64,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub cuisine: String,
    #[serde(default)]
    pub meal_types: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub source_kind: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub ingredients: Vec<RecipeIngredient>,
    #[serde(default)]
    pub steps: Vec<RecipeStep>,
    #[serde(default)]
    pub sources: Vec<RecipeSource>,
    #[serde(default)]
    pub images: Vec<RecipeImage>,
    #[serde(default)]
    pub nutrition_total: Nutrition,
    #[serde(default)]
    pub nutrition_per_serving: Nutrition,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_confidence() -> f64 {
    0.7
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSummary {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub servings: f64,
    pub total_minutes: i64,
    pub cuisine: String,
    pub meal_types: Vec<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub archived: bool,
    pub image_url: Option<String>,
    pub average_rating: Option<f64>,
    pub last_cooked_at: Option<String>,
    pub nutrition_per_serving: Nutrition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRating {
    #[serde(default)]
    pub id: String,
    pub recipe_id: String,
    pub score: i64,
    pub comment: Option<String>,
    pub cooked_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    #[serde(default)]
    pub id: String,
    pub date: NaiveDate,
    pub slot: String,
    pub recipe_id: Option<String>,
    pub title_override: Option<String>,
    #[serde(default = "default_servings")]
    pub servings: f64,
    #[serde(default = "default_plan_status")]
    pub status: String,
    pub notes: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_servings() -> f64 {
    1.0
}

fn default_plan_status() -> String {
    "planned".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DayPlan {
    pub date: NaiveDate,
    pub entries: Vec<PlanEntry>,
    pub nutrition: Nutrition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WeekPlan {
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub days: Vec<DayPlan>,
    pub nutrition: Nutrition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingList {
    pub id: String,
    pub range_start: NaiveDate,
    pub range_end: NaiveDate,
    pub items: Vec<ShoppingItem>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingItem {
    pub id: String,
    pub list_id: String,
    pub name: String,
    pub quantity: f64,
    pub unit: String,
    pub aisle: String,
    pub checked: bool,
    pub manual: bool,
    #[serde(default)]
    pub recipe_ids: Vec<String>,
    pub note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PantryItem {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub exclude_from_shopping: bool,
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    #[serde(default)]
    pub id: String,
    pub kind: String,
    pub content: String,
    #[serde(default = "default_confidence")]
    pub confidence: f64,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default = "default_memory_status")]
    pub status: String,
    pub preference_score: Option<f64>,
    pub source: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    pub last_used_at: Option<String>,
}

fn default_memory_status() -> String {
    "proposed".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub codex_thread_id: Option<String>,
    pub title: String,
    pub status: String,
    pub metadata: Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSummary {
    pub session: AgentSession,
    pub message_count: usize,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub item_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UndoRecord {
    pub id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub action: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
    pub undone: bool,
    pub created_at: String,
    pub undone_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DynamicToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapData {
    pub profile: UserProfile,
    pub equipment: Vec<Equipment>,
    pub recipes: Vec<RecipeSummary>,
    pub current_week: WeekPlan,
    pub memories: Vec<Memory>,
    pub shopping_list: Option<ShoppingList>,
}
