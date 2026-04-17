use serde::{Deserialize, Serialize};

// ── User ──

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: u64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub default_currency: Option<String>,
    pub locale: Option<String>,
    pub notifications_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct UserResponse {
    pub user: User,
}

// ── Group ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub group_type: Option<String>,
    pub simplify_by_default: Option<bool>,
    pub invite_link: Option<String>,
    #[serde(default)]
    pub members: Vec<GroupMember>,
    #[serde(default)]
    pub original_debts: Vec<Debt>,
    #[serde(default)]
    pub simplified_debts: Vec<Debt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: u64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub balance: Vec<Balance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debt {
    pub from: u64,
    pub to: u64,
    pub amount: String,
    pub currency_code: String,
}

#[derive(Debug, Deserialize)]
pub struct GroupsResponse {
    pub groups: Vec<Group>,
}

#[derive(Debug, Deserialize)]
pub struct GroupResponse {
    pub group: Group,
}

// ── Friend ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Friend {
    pub id: u64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    #[serde(default)]
    pub balance: Vec<Balance>,
    #[serde(default)]
    pub groups: Vec<GroupBalance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub currency_code: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GroupBalance {
    pub group_id: u64,
    #[serde(default)]
    pub balance: Vec<Balance>,
}

#[derive(Debug, Deserialize)]
pub struct FriendsResponse {
    pub friends: Vec<Friend>,
}

#[derive(Debug, Deserialize)]
pub struct FriendResponse {
    pub friend: Friend,
}

#[derive(Debug, Deserialize)]
pub struct CreateFriendResponse {
    pub friends: Vec<Friend>,
}

// ── Expense ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Expense {
    pub id: u64,
    pub description: String,
    pub cost: String,
    pub currency_code: String,
    pub date: Option<String>,
    pub created_by: Option<ExpenseUser>,
    pub category: Option<ExpenseCategory>,
    pub deleted_at: Option<String>,
    pub repeat_interval: Option<String>,
    #[serde(default)]
    pub repayments: Vec<Repayment>,
    #[serde(default)]
    pub users: Vec<Share>,
    #[serde(default)]
    pub comments: Vec<Comment>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpenseUser {
    pub id: u64,
    pub first_name: String,
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExpenseCategory {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Share {
    pub user_id: u64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub paid_share: String,
    pub owed_share: String,
    pub net_balance: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Repayment {
    pub from: u64,
    pub to: u64,
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct ExpensesResponse {
    pub expenses: Vec<Expense>,
}

#[derive(Debug, Deserialize)]
pub struct ExpenseResponse {
    pub expense: Expense,
}

// ── Comment ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment {
    pub id: u64,
    pub content: String,
    pub created_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_by: Option<CommentUser>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommentUser {
    pub id: u64,
    pub first_name: String,
    pub last_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommentsResponse {
    pub comments: Vec<Comment>,
}

#[derive(Debug, Deserialize)]
pub struct CommentResponse {
    pub comment: Comment,
}

// ── Notification ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: u64,
    #[serde(rename = "type")]
    pub notification_type: u64,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub created_by: Option<u64>,
    pub source: Option<NotificationSource>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationSource {
    #[serde(rename = "type")]
    pub source_type: Option<String>,
    pub id: Option<u64>,
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationsResponse {
    pub notifications: Vec<Notification>,
}

// ── Currency ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Currency {
    pub currency_code: String,
    pub unit: String,
}

#[derive(Debug, Deserialize)]
pub struct CurrenciesResponse {
    pub currencies: Vec<Currency>,
}

// ── Category ──

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub subcategories: Vec<SubCategory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubCategory {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CategoriesResponse {
    pub categories: Vec<Category>,
}

// ── Generic success/error ──

#[derive(Debug, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub errors: Option<serde_json::Value>,
}
