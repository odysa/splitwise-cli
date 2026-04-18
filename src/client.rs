use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;

use crate::models::*;

const BASE_URL: &str = "https://secure.splitwise.com/api/v3.0";

pub struct Client {
    token: String,
}

impl Client {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    fn get<T: DeserializeOwned>(&self, path: &str, params: &[(&str, &str)]) -> Result<T> {
        let url = format!("{BASE_URL}{path}");
        let mut req = minreq::get(&url).with_header("Authorization", self.auth_header());
        for &(k, v) in params {
            req = req.with_param(k, v);
        }
        let resp = req.send().with_context(|| format!("GET {path} failed"))?;
        if resp.status_code >= 400 {
            bail!("API error ({}): {}", resp.status_code, resp.as_str().unwrap_or(""));
        }
        let body = resp.as_str().with_context(|| format!("invalid UTF-8 from GET {path}"))?;
        serde_json::from_str(body).with_context(|| format!("failed to parse GET {path} response"))
    }

    fn post_form<T: DeserializeOwned>(
        &self,
        path: &str,
        form: &[(String, String)],
    ) -> Result<T> {
        let url = format!("{BASE_URL}{path}");
        let encoded: String = form
            .iter()
            .enumerate()
            .fold(String::new(), |mut acc, (i, (k, v))| {
                if i > 0 { acc.push('&'); }
                acc.push_str(&urlenc(k));
                acc.push('=');
                acc.push_str(&urlenc(v));
                acc
            });
        let resp = minreq::post(&url)
            .with_header("Authorization", self.auth_header())
            .with_header("Content-Type", "application/x-www-form-urlencoded")
            .with_body(encoded)
            .send()
            .with_context(|| format!("POST {path} failed"))?;
        if resp.status_code >= 400 {
            bail!("API error ({}): {}", resp.status_code, resp.as_str().unwrap_or(""));
        }
        let body = resp.as_str().with_context(|| format!("invalid UTF-8 from POST {path}"))?;
        serde_json::from_str(body)
            .with_context(|| format!("failed to parse POST {path} response"))
    }

    fn post_success(&self, path: &str, form: &[(String, String)]) -> Result<()> {
        let resp: SuccessResponse = self.post_form(path, form)?;
        if !resp.success {
            if let Some(errors) = resp.errors {
                bail!("API error: {errors}");
            }
            bail!("API returned success=false");
        }
        Ok(())
    }

    // ── User ──

    pub fn get_current_user(&self) -> Result<User> {
        let resp: UserResponse = self.get("/get_current_user", &[])?;
        Ok(resp.user)
    }

    pub fn get_user(&self, id: u64) -> Result<User> {
        let resp: UserResponse = self.get(&format!("/get_user/{id}"), &[])?;
        Ok(resp.user)
    }

    pub fn update_user(&self, id: u64, fields: &[(String, String)]) -> Result<User> {
        let resp: UserResponse = self.post_form(&format!("/update_user/{id}"), fields)?;
        Ok(resp.user)
    }

    // ── Groups ──

    pub fn get_groups(&self) -> Result<Vec<Group>> {
        let resp: GroupsResponse = self.get("/get_groups", &[])?;
        Ok(resp.groups)
    }

    pub fn get_group(&self, id: u64) -> Result<Group> {
        let resp: GroupResponse = self.get(&format!("/get_group/{id}"), &[])?;
        Ok(resp.group)
    }

    pub fn create_group(
        &self,
        name: &str,
        group_type: Option<&str>,
        simplify: bool,
    ) -> Result<Group> {
        let mut form = vec![("name".into(), name.to_string())];
        if let Some(t) = group_type {
            form.push(("group_type".into(), t.to_string()));
        }
        if simplify {
            form.push(("simplify_by_default".into(), "true".into()));
        }
        let resp: GroupResponse = self.post_form("/create_group", &form)?;
        Ok(resp.group)
    }

    pub fn delete_group(&self, id: u64) -> Result<()> {
        self.post_success(&format!("/delete_group/{id}"), &[])
    }

    pub fn restore_group(&self, id: u64) -> Result<()> {
        self.post_success(&format!("/undelete_group/{id}"), &[])
    }

    pub fn add_to_group(
        &self,
        group_id: u64,
        user_id: Option<u64>,
        email: Option<&str>,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> Result<()> {
        let mut form = vec![("group_id".into(), group_id.to_string())];
        if let Some(uid) = user_id {
            form.push(("user_id".into(), uid.to_string()));
        }
        if let Some(e) = email {
            form.push(("email".into(), e.to_string()));
        }
        if let Some(f) = first_name {
            form.push(("first_name".into(), f.to_string()));
        }
        if let Some(l) = last_name {
            form.push(("last_name".into(), l.to_string()));
        }
        self.post_success("/add_user_to_group", &form)
    }

    pub fn remove_from_group(&self, group_id: u64, user_id: u64) -> Result<()> {
        self.post_success(
            "/remove_user_from_group",
            &[
                ("group_id".into(), group_id.to_string()),
                ("user_id".into(), user_id.to_string()),
            ],
        )
    }

    // ── Friends ──

    pub fn get_friends(&self) -> Result<Vec<Friend>> {
        let resp: FriendsResponse = self.get("/get_friends", &[])?;
        Ok(resp.friends)
    }

    pub fn get_friend(&self, id: u64) -> Result<Friend> {
        let resp: FriendResponse = self.get(&format!("/get_friend/{id}"), &[])?;
        Ok(resp.friend)
    }

    pub fn add_friend(
        &self,
        email: &str,
        first_name: Option<&str>,
        last_name: Option<&str>,
    ) -> Result<Vec<Friend>> {
        let mut form = vec![("user_email".into(), email.to_string())];
        if let Some(f) = first_name {
            form.push(("user_first_name".into(), f.to_string()));
        }
        if let Some(l) = last_name {
            form.push(("user_last_name".into(), l.to_string()));
        }
        let resp: CreateFriendResponse = self.post_form("/create_friend", &form)?;
        Ok(resp.friends)
    }

    pub fn delete_friend(&self, id: u64) -> Result<()> {
        self.post_success(&format!("/delete_friend/{id}"), &[])
    }

    // ── Expenses ──

    pub fn get_expenses(&self, params: &[(&str, &str)]) -> Result<Vec<Expense>> {
        let resp: ExpensesResponse = self.get("/get_expenses", params)?;
        Ok(resp.expenses)
    }

    pub fn get_expense(&self, id: u64) -> Result<Expense> {
        let resp: ExpenseResponse = self.get(&format!("/get_expense/{id}"), &[])?;
        Ok(resp.expense)
    }

    pub fn create_expense(&self, form: &[(String, String)]) -> Result<Vec<Expense>> {
        let resp: ExpensesResponse = self.post_form("/create_expense", form)?;
        Ok(resp.expenses)
    }

    pub fn update_expense(&self, id: u64, form: &[(String, String)]) -> Result<Vec<Expense>> {
        let resp: ExpensesResponse = self.post_form(&format!("/update_expense/{id}"), form)?;
        Ok(resp.expenses)
    }

    pub fn delete_expense(&self, id: u64) -> Result<()> {
        self.post_success(&format!("/delete_expense/{id}"), &[])
    }

    pub fn restore_expense(&self, id: u64) -> Result<()> {
        self.post_success(&format!("/undelete_expense/{id}"), &[])
    }

    // ── Comments ──

    pub fn get_comments(&self, expense_id: u64) -> Result<Vec<Comment>> {
        let id = expense_id.to_string();
        let resp: CommentsResponse = self.get("/get_comments", &[("expense_id", &id)])?;
        Ok(resp.comments)
    }

    pub fn create_comment(&self, expense_id: u64, content: &str) -> Result<Comment> {
        let resp: CommentResponse = self.post_form(
            "/create_comment",
            &[
                ("expense_id".into(), expense_id.to_string()),
                ("content".into(), content.to_string()),
            ],
        )?;
        Ok(resp.comment)
    }

    pub fn delete_comment(&self, id: u64) -> Result<()> {
        let url = format!("{BASE_URL}/delete_comment/{id}");
        let resp = minreq::post(&url)
            .with_header("Authorization", self.auth_header())
            .send()
            .with_context(|| format!("POST /delete_comment/{id} failed"))?;
        if resp.status_code >= 400 {
            bail!("failed to delete comment {id}");
        }
        Ok(())
    }

    // ── Other ──

    pub fn get_notifications(&self, params: &[(&str, &str)]) -> Result<Vec<Notification>> {
        let resp: NotificationsResponse = self.get("/get_notifications", params)?;
        Ok(resp.notifications)
    }

    pub fn get_currencies(&self) -> Result<Vec<Currency>> {
        let resp: CurrenciesResponse = self.get("/get_currencies", &[])?;
        Ok(resp.currencies)
    }

    pub fn get_categories(&self) -> Result<Vec<Category>> {
        let resp: CategoriesResponse = self.get("/get_categories", &[])?;
        Ok(resp.categories)
    }
}

fn urlenc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(b >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(b & 0xf) as usize]));
            }
        }
    }
    out
}
