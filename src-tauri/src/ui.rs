use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::{
    AgentMessage, Equipment, Memory, Nutrition, PlanEntry, Recipe, RecipeImage, RecipeIngredient,
    RecipeRating, RecipeSource, RecipeStep, ShoppingItem, UserProfile,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiNutrition {
    #[serde(default)]
    pub calories: f64,
    #[serde(default)]
    pub protein: f64,
    #[serde(default)]
    pub carbs: f64,
    #[serde(default)]
    pub fat: f64,
    #[serde(default)]
    pub fiber: f64,
}

impl From<&Nutrition> for UiNutrition {
    fn from(value: &Nutrition) -> Self {
        Self {
            calories: value.calories_kcal,
            protein: value.protein_g,
            carbs: value.carbs_g,
            fat: value.fat_g,
            fiber: value.fiber_g,
        }
    }
}

impl From<&UiNutrition> for Nutrition {
    fn from(value: &UiNutrition) -> Self {
        Self {
            calories_kcal: value.calories,
            protein_g: value.protein,
            carbs_g: value.carbs,
            fat_g: value.fat,
            fiber_g: value.fiber,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiIngredient {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiRecipe {
    #[serde(default)]
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub image_url: Option<String>,
    #[serde(default)]
    pub meal_types: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prep_minutes: i64,
    #[serde(default)]
    pub cook_minutes: i64,
    #[serde(default = "one")]
    pub servings: f64,
    #[serde(default)]
    pub nutrition: UiNutrition,
    #[serde(default)]
    pub ingredients: Vec<UiIngredient>,
    #[serde(default)]
    pub steps: Vec<String>,
    pub rating: Option<f64>,
    pub rating_comment: Option<String>,
    pub source_url: Option<String>,
    pub source_name: Option<String>,
    pub last_cooked_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub favorite: bool,
}

fn one() -> f64 {
    1.0
}

impl UiRecipe {
    pub fn from_domain(recipe: &Recipe, ratings: &[RecipeRating]) -> Self {
        let latest = ratings.first();
        let average = if ratings.is_empty() {
            None
        } else {
            Some(
                ratings
                    .iter()
                    .map(|rating| rating.score as f64)
                    .sum::<f64>()
                    / ratings.len() as f64,
            )
        };
        Self {
            id: recipe.id.clone(),
            title: recipe.title.clone(),
            description: recipe.summary.clone(),
            image_url: recipe.images.first().map(|image| image.url.clone()),
            meal_types: recipe
                .meal_types
                .iter()
                .map(|value| slot_to_meal_type(value).to_string())
                .collect(),
            tags: recipe.tags.clone(),
            prep_minutes: recipe.prep_minutes,
            cook_minutes: recipe.cook_minutes,
            servings: recipe.servings,
            nutrition: UiNutrition::from(&recipe.nutrition_per_serving),
            ingredients: recipe
                .ingredients
                .iter()
                .map(|ingredient| UiIngredient {
                    id: ingredient.id.clone(),
                    name: ingredient.name.clone(),
                    amount: ingredient.quantity,
                    unit: ingredient.unit.clone(),
                    category: ingredient.aisle.clone(),
                    optional: ingredient.optional,
                })
                .collect(),
            steps: recipe
                .steps
                .iter()
                .map(|step| step.instruction.clone())
                .collect(),
            rating: average,
            rating_comment: latest.and_then(|rating| rating.comment.clone()),
            source_url: recipe.sources.first().and_then(|source| source.url.clone()),
            source_name: recipe.sources.first().map(|source| source.title.clone()),
            last_cooked_at: ratings
                .iter()
                .filter_map(|rating| rating.cooked_at.clone())
                .max(),
            created_at: recipe.created_at.clone(),
            updated_at: recipe.updated_at.clone(),
            favorite: recipe.favorite,
        }
    }

    pub fn into_domain(self, existing: Option<Recipe>) -> Recipe {
        let old = existing.as_ref();
        let total: Nutrition = (&self.nutrition).into();
        let ingredients = self
            .ingredients
            .into_iter()
            .enumerate()
            .map(|(index, ingredient)| RecipeIngredient {
                id: ingredient.id,
                recipe_id: self.id.clone(),
                name: ingredient.name,
                quantity: ingredient.amount,
                unit: ingredient.unit,
                aisle: ingredient.category,
                preparation: None,
                optional: ingredient.optional,
                nutrition: Nutrition::default(),
                position: index as i64,
            })
            .collect();
        let steps = self
            .steps
            .into_iter()
            .enumerate()
            .map(|(index, instruction)| RecipeStep {
                id: String::new(),
                recipe_id: self.id.clone(),
                position: index as i64,
                instruction,
                timer_minutes: None,
            })
            .collect();
        let sources = if self.source_url.is_some() || self.source_name.is_some() {
            vec![RecipeSource {
                id: String::new(),
                recipe_id: self.id.clone(),
                title: self.source_name.clone().unwrap_or_else(|| "Quelle".into()),
                url: self.source_url.clone(),
                publisher: None,
                source_type: "web".into(),
                accessed_at: None,
            }]
        } else {
            Vec::new()
        };
        let images = self.image_url.map_or_else(Vec::new, |url| {
            vec![RecipeImage {
                id: String::new(),
                recipe_id: self.id.clone(),
                url,
                kind: "source".into(),
                alt_text: Some(self.title.clone()),
                attribution: None,
                position: 0,
            }]
        });
        let created_at = old
            .map(|recipe| recipe.created_at.clone())
            .unwrap_or(self.created_at);
        Recipe {
            id: self.id,
            title: self.title,
            summary: self.description,
            servings: self.servings,
            prep_minutes: self.prep_minutes,
            cook_minutes: self.cook_minutes,
            difficulty: old
                .map(|recipe| recipe.difficulty.clone())
                .unwrap_or_else(|| "einfach".into()),
            cuisine: old
                .map(|recipe| recipe.cuisine.clone())
                .unwrap_or_else(|| "Alltag".into()),
            meal_types: self
                .meal_types
                .iter()
                .map(|value| meal_type_to_slot(value).to_string())
                .collect(),
            tags: self.tags,
            favorite: self.favorite,
            archived: old.is_some_and(|recipe| recipe.archived),
            source_kind: old
                .map(|recipe| recipe.source_kind.clone())
                .unwrap_or_else(|| "manual".into()),
            confidence: old.map_or(1.0, |recipe| recipe.confidence),
            ingredients,
            steps,
            sources,
            images,
            nutrition_total: total.clone(),
            nutrition_per_serving: total,
            created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPlanItem {
    #[serde(default)]
    pub id: String,
    pub date: String,
    pub meal_type: String,
    pub recipe_id: Option<String>,
    pub recipe: Option<UiRecipe>,
    #[serde(default)]
    pub title_override: Option<String>,
    #[serde(default = "one")]
    pub servings: f64,
    #[serde(default = "planned")]
    pub status: String,
    pub note: Option<String>,
}

fn planned() -> String {
    "planned".into()
}

impl UiPlanItem {
    pub fn from_domain(entry: &PlanEntry, recipe: Option<UiRecipe>) -> Self {
        Self {
            id: entry.id.clone(),
            date: entry.date.to_string(),
            meal_type: slot_to_meal_type(&entry.slot).into(),
            recipe_id: entry.recipe_id.clone(),
            recipe,
            title_override: entry.title_override.clone(),
            servings: entry.servings,
            status: entry.status.clone(),
            note: entry.notes.clone(),
        }
    }

    pub fn into_domain(self) -> Result<PlanEntry, String> {
        Ok(PlanEntry {
            id: self.id,
            date: NaiveDate::parse_from_str(&self.date, "%Y-%m-%d")
                .map_err(|_| "Ungültiges Datum".to_string())?,
            slot: meal_type_to_slot(&self.meal_type).into(),
            recipe_id: self.recipe_id,
            title_override: self
                .title_override
                .or_else(|| self.recipe.map(|recipe| recipe.title)),
            servings: self.servings,
            status: self.status,
            notes: self.note,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        })
    }
}

fn slot_to_meal_type(slot: &str) -> &str {
    match slot {
        "breakfast" => "fruehstueck",
        "lunch" => "mittagessen",
        "dinner" => "abendessen",
        "shake" => "shake",
        "dessert" => "dessert",
        "other" => "sonstiges",
        _ => "snack",
    }
}

fn meal_type_to_slot(meal_type: &str) -> &str {
    match meal_type {
        "fruehstueck" => "breakfast",
        "mittagessen" => "lunch",
        "abendessen" => "dinner",
        "shake" => "shake",
        "dessert" => "dessert",
        "sonstiges" => "other",
        _ => "snack",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiShoppingItem {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub amount: f64,
    #[serde(default)]
    pub unit: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub manual: bool,
    #[serde(default)]
    pub recipe_ids: Vec<String>,
}

impl From<&ShoppingItem> for UiShoppingItem {
    fn from(value: &ShoppingItem) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            amount: value.quantity,
            unit: value.unit.clone(),
            category: value.aisle.clone(),
            checked: value.checked,
            manual: value.manual,
            recipe_ids: value.recipe_ids.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiMemory {
    #[serde(default)]
    pub id: String,
    #[serde(default = "preference")]
    pub kind: String,
    pub title: String,
    pub content: String,
    #[serde(default = "one")]
    pub confidence: f64,
    #[serde(default = "explicit")]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preference_score: Option<f64>,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

fn preference() -> String {
    "preference".into()
}

fn explicit() -> String {
    "explicit".into()
}

fn yes() -> bool {
    true
}

impl UiMemory {
    pub fn from_domain(value: &Memory) -> Self {
        let title = value
            .evidence
            .iter()
            .find_map(|evidence| evidence.strip_prefix("title:"))
            .map(str::to_string)
            .unwrap_or_else(|| value.content.chars().take(54).collect());
        Self {
            id: value.id.clone(),
            kind: value.kind.clone(),
            title,
            content: value.content.clone(),
            confidence: value.confidence,
            source: value.source.clone().unwrap_or_else(|| "inferred".into()),
            preference_score: value.preference_score,
            // Proposed memories are intentionally visible and usable. They
            // are not a hidden or paused record; only an explicit dismissal
            // makes a memory inactive in the UI.
            active: value.status != "dismissed",
            created_at: value.created_at.clone(),
            updated_at: value.updated_at.clone(),
        }
    }

    pub fn into_domain(self) -> Memory {
        Memory {
            id: self.id,
            kind: self.kind,
            content: self.content,
            confidence: self.confidence,
            evidence: vec![format!("title:{}", self.title)],
            status: if self.active {
                "confirmed"
            } else {
                "dismissed"
            }
            .into(),
            preference_score: self.preference_score,
            source: Some(self.source),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_used_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiEquipment {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default = "yes")]
    pub enabled: bool,
}

impl From<&Equipment> for UiEquipment {
    fn from(value: &Equipment) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            enabled: value.available,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiProfile {
    pub name: String,
    pub height_cm: Option<f64>,
    pub weight_kg: Option<f64>,
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub sex_for_energy: Option<String>,
    #[serde(default = "low_active")]
    pub activity_level: String,
    #[serde(default = "manual")]
    pub calorie_target_mode: String,
    #[serde(default)]
    pub calculated_calorie_target: Option<f64>,
    #[serde(default)]
    pub manual_calorie_target: Option<f64>,
    #[serde(default)]
    pub calorie_target: f64,
    #[serde(default)]
    pub protein_target: f64,
    #[serde(default)]
    pub fiber_target: f64,
    #[serde(default = "balanced")]
    pub budget_preference: String,
    #[serde(default = "weekday_minutes")]
    pub weekday_max_minutes: i64,
    #[serde(default = "weekend_minutes")]
    pub weekend_max_minutes: i64,
    #[serde(default)]
    pub cooking_style: String,
    #[serde(default)]
    pub dislikes: Vec<String>,
    #[serde(default)]
    pub favorites: Vec<String>,
    #[serde(default)]
    pub equipment: Vec<UiEquipment>,
    #[serde(default = "agent_name")]
    pub agent_name: String,
    #[serde(default = "agent_personality")]
    pub agent_personality: String,
    #[serde(default = "balanced")]
    pub autonomy: String,
}

fn balanced() -> String {
    "ausgewogen".into()
}

fn low_active() -> String {
    "low_active".into()
}

fn manual() -> String {
    "manual".into()
}

fn weekday_minutes() -> i64 {
    45
}

fn weekend_minutes() -> i64 {
    90
}

fn agent_name() -> String {
    "Mila".into()
}

fn agent_personality() -> String {
    "Direkt, aufmerksam, warm und pragmatisch.".into()
}

impl UiProfile {
    pub fn from_domain(
        profile: &UserProfile,
        equipment: &[Equipment],
        memories: &[Memory],
        metadata: &serde_json::Value,
    ) -> Self {
        let favorites = memories
            .iter()
            .filter(|memory| {
                memory.status == "confirmed"
                    && memory.kind == "preference"
                    && memory.preference_score.unwrap_or(5.0) >= 7.0
            })
            .map(|memory| memory.content.clone())
            .collect();
        let dislikes = memories
            .iter()
            .filter(|memory| {
                memory.status == "confirmed"
                    && memory.kind == "preference"
                    && memory.preference_score.unwrap_or(5.0) <= 3.0
            })
            .map(|memory| memory.content.clone())
            .collect();
        Self {
            name: profile.name.clone(),
            height_cm: profile.height_cm,
            weight_kg: profile.weight_kg,
            birth_date: profile.birth_date.clone(),
            sex_for_energy: profile.sex_for_energy.clone(),
            activity_level: profile.activity_level.clone(),
            calorie_target_mode: profile.calorie_target_mode.clone(),
            calculated_calorie_target: (profile.calorie_target_mode == "calculated")
                .then_some(profile.nutrition_targets.calories_kcal)
                .flatten(),
            manual_calorie_target: profile.manual_calorie_target_kcal,
            calorie_target: profile.nutrition_targets.calories_kcal.unwrap_or(2400.0),
            protein_target: profile
                .nutrition_targets
                .protein_g
                .unwrap_or_else(|| profile.weight_kg.unwrap_or(85.0) * 2.0),
            fiber_target: profile.nutrition_targets.fiber_g.unwrap_or(35.0),
            budget_preference: metadata
                .get("budgetPreference")
                .and_then(|value| value.as_str())
                .unwrap_or("ausgewogen")
                .into(),
            weekday_max_minutes: profile.weekday_max_minutes,
            weekend_max_minutes: profile.weekend_max_minutes,
            cooking_style: profile.notes.clone().unwrap_or_default(),
            dislikes,
            favorites,
            equipment: equipment.iter().map(UiEquipment::from).collect(),
            agent_name: metadata
                .get("agentName")
                .and_then(|value| value.as_str())
                .unwrap_or("Mila")
                .into(),
            agent_personality: metadata
                .get("agentPersonality")
                .and_then(|value| value.as_str())
                .unwrap_or("Direkt, aufmerksam, warm und pragmatisch.")
                .into(),
            autonomy: metadata
                .get("autonomy")
                .and_then(|value| value.as_str())
                .unwrap_or("ausgewogen")
                .into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiToolActivity {
    pub id: String,
    pub name: String,
    pub label: String,
    pub status: String,
    pub detail: Option<String>,
    #[serde(default)]
    pub recipe_id: Option<String>,
    #[serde(default)]
    pub recipe_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAgentMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub streaming: bool,
    pub tools: Option<Vec<UiToolActivity>>,
}

impl From<&AgentMessage> for UiAgentMessage {
    fn from(value: &AgentMessage) -> Self {
        let payload = value.tool_payload.as_ref();
        let tools = payload
            .and_then(|payload| payload.get("tools"))
            .and_then(|tools| serde_json::from_value::<Vec<UiToolActivity>>(tools.clone()).ok())
            .or_else(|| {
                value.tool_name.as_ref().map(|name| {
                    vec![UiToolActivity {
                        id: value.item_id.clone().unwrap_or_else(|| value.id.clone()),
                        name: name.clone(),
                        label: tool_label(name).into(),
                        status: payload
                            .and_then(|value| value.get("status"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("success")
                            .into(),
                        detail: payload
                            .and_then(|value| value.get("detail"))
                            .and_then(|value| value.as_str())
                            .map(str::to_owned),
                        recipe_id: payload
                            .and_then(|value| value.get("recipeId"))
                            .and_then(|value| value.as_str())
                            .map(str::to_owned),
                        recipe_title: payload
                            .and_then(|value| value.get("recipeTitle"))
                            .and_then(|value| value.as_str())
                            .map(str::to_owned),
                    }]
                })
            });
        Self {
            id: value.item_id.clone().unwrap_or_else(|| value.id.clone()),
            role: value.role.clone(),
            content: value.content.clone(),
            created_at: value.created_at.clone(),
            streaming: false,
            tools,
        }
    }
}

pub fn tool_label(name: &str) -> &str {
    match name {
        "profile_get_context" => "Profil gelesen",
        "memory_recall" => "Erinnerungen gelesen",
        "memory_propose" => "Erinnerung vorgeschlagen",
        "recipes_search" => "Rezepte durchsucht",
        "recipes_get" => "Rezept geöffnet",
        "recipes_save" | "recipes_create_draft" => "Rezept gespeichert",
        "recipes_set_image" => "Rezeptbild gespeichert",
        "web_search" | "webSearch" => "Webrecherche",
        "image_generation" | "imageGeneration" => "Bild generiert",
        "reasoning" => "Planung wird vorbereitet",
        "plan_get_week" => "Wochenplan geprüft",
        "plan_set_meal" | "plan_propose_week" => "Wochenplan aktualisiert",
        "shopping_rebuild" => "Einkaufsliste berechnet",
        "ratings_record" => "Bewertung gespeichert",
        "changes_undo" => "Änderung rückgängig gemacht",
        _ => "MealZ aktualisiert",
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiBootstrap {
    pub onboarding_complete: bool,
    pub recipes: Vec<UiRecipe>,
    pub plan: Vec<UiPlanItem>,
    pub shopping: Vec<UiShoppingItem>,
    pub memories: Vec<UiMemory>,
    pub profile: UiProfile,
    pub messages: Vec<UiAgentMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiAgentFiles {
    pub persona: String,
    pub memory: String,
}
