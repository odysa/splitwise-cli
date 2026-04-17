mod client;
mod config;
mod display;
mod models;
mod tui;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "splitwise", about = "Splitwise CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Launch interactive TUI
    #[arg(long)]
    tui: bool,

    /// Output raw JSON
    #[arg(long, short, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Save your Splitwise API key
    Auth {
        /// API key from https://secure.splitwise.com/apps
        key: String,
    },

    /// Show current user
    Me,

    /// Get user by ID
    User { id: u64 },

    /// Update user fields (key=value pairs)
    UpdateUser {
        id: u64,
        /// e.g. first_name=John last_name=Doe
        #[arg(value_parser = parse_key_val)]
        fields: Vec<(String, String)>,
    },

    /// List all groups
    Groups,

    /// Get group details
    Group { id: u64 },

    /// Create a new group
    CreateGroup {
        name: String,
        /// e.g. apartment, house, trip, other
        #[arg(long)]
        group_type: Option<String>,
        /// Enable debt simplification
        #[arg(long)]
        simplify: bool,
    },

    /// Delete a group
    DeleteGroup { id: u64 },

    /// Restore a deleted group
    RestoreGroup { id: u64 },

    /// Add a user to a group
    AddToGroup {
        group_id: u64,
        /// Add existing Splitwise user by ID
        #[arg(long)]
        user_id: Option<u64>,
        /// Invite by email
        #[arg(long)]
        email: Option<String>,
        #[arg(long)]
        first_name: Option<String>,
        #[arg(long)]
        last_name: Option<String>,
    },

    /// Remove a user from a group
    RemoveFromGroup { group_id: u64, user_id: u64 },

    /// List all friends
    Friends,

    /// Get friend details
    Friend { id: u64 },

    /// Add a friend by email
    AddFriend {
        email: String,
        #[arg(long)]
        first_name: Option<String>,
        #[arg(long)]
        last_name: Option<String>,
    },

    /// Delete a friend
    DeleteFriend { id: u64 },

    /// List expenses
    Expenses {
        #[arg(long)]
        group_id: Option<u64>,
        #[arg(long)]
        friend_id: Option<u64>,
        /// Show expenses after this date (YYYY-MM-DD)
        #[arg(long)]
        after: Option<String>,
        /// Show expenses before this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,
        #[arg(long, default_value = "20")]
        limit: u64,
        #[arg(long)]
        offset: Option<u64>,
    },

    /// Get expense details
    Expense { id: u64 },

    /// Create an expense
    CreateExpense {
        #[arg(long, short)]
        description: String,
        /// Total cost, e.g. "50.00"
        #[arg(long, short)]
        cost: String,
        #[arg(long, short)]
        group_id: u64,
        #[arg(long)]
        currency: Option<String>,
        #[arg(long)]
        category_id: Option<u64>,
        /// YYYY-MM-DD
        #[arg(long)]
        date: Option<String>,
        /// Split equally among group members (you pay all)
        #[arg(long)]
        split_equally: bool,
        /// Custom share: uid:paid_share:owed_share (repeatable)
        #[arg(long = "user", short)]
        users: Vec<String>,
    },

    /// Update an expense
    UpdateExpense {
        id: u64,
        #[arg(long, short)]
        description: Option<String>,
        #[arg(long, short)]
        cost: Option<String>,
        #[arg(long, short)]
        group_id: Option<u64>,
        #[arg(long)]
        currency: Option<String>,
        #[arg(long)]
        category_id: Option<u64>,
        #[arg(long)]
        date: Option<String>,
    },

    /// Delete an expense
    DeleteExpense { id: u64 },

    /// Restore a deleted expense
    RestoreExpense { id: u64 },

    /// List comments on an expense
    Comments { expense_id: u64 },

    /// Add a comment to an expense
    CreateComment { expense_id: u64, content: String },

    /// Delete a comment
    DeleteComment { comment_id: u64 },

    /// View balances (overall or per group)
    Balances {
        #[arg(long)]
        group_id: Option<u64>,
    },

    /// List supported currencies
    Currencies,

    /// List expense categories
    Categories,

    /// View notifications
    Notifications {
        /// Show notifications after this date
        #[arg(long)]
        after: Option<String>,
        #[arg(long)]
        limit: Option<u64>,
    },
}

fn parse_key_val(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid key=value: no `=` in `{s}`"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

macro_rules! json_or {
    ($json:expr, $data:expr, $print_fn:expr) => {
        if $json {
            println!("{}", serde_json::to_string_pretty(&$data)?);
        } else {
            $print_fn(&$data);
        }
    };
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    if cli.tui {
        let token = config::load_token()?;
        let client = client::Client::new(token);
        return tui::run(&client);
    }

    let Some(command) = cli.command else {
        use clap::CommandFactory;
        Cli::command().print_help()?;
        println!();
        return Ok(());
    };

    let json = cli.json;

    // Auth doesn't need a token
    if let Command::Auth { key } = &command {
        config::save_token(key)?;
        println!("API key saved.");
        return Ok(());
    }

    let token = config::load_token()?;
    let client = client::Client::new(token);

    match command {
        Command::Auth { .. } => unreachable!(),

        Command::Me => {
            let user = client.get_current_user()?;
            json_or!(json, user, display::print_user);
        }

        Command::User { id } => {
            let user = client.get_user(id)?;
            json_or!(json, user, display::print_user);
        }

        Command::UpdateUser { id, fields } => {
            let user = client.update_user(id, &fields)?;
            json_or!(json, user, display::print_user);
        }

        Command::Groups => {
            let groups = client.get_groups()?;
            json_or!(json, groups, display::print_groups);
        }

        Command::Group { id } => {
            let group = client.get_group(id)?;
            json_or!(json, group, display::print_group);
        }

        Command::CreateGroup {
            name,
            group_type,
            simplify,
        } => {
            let group = client.create_group(&name, group_type.as_deref(), simplify)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&group)?);
            } else {
                println!("Group created (ID: {})", group.id);
                display::print_group(&group);
            }
        }

        Command::DeleteGroup { id } => {
            client.delete_group(id)?;
            println!("Group {id} deleted.");
        }

        Command::RestoreGroup { id } => {
            client.restore_group(id)?;
            println!("Group {id} restored.");
        }

        Command::AddToGroup {
            group_id,
            user_id,
            email,
            first_name,
            last_name,
        } => {
            if user_id.is_none() && email.is_none() {
                bail!("provide --user-id or --email");
            }
            client.add_to_group(
                group_id,
                user_id,
                email.as_deref(),
                first_name.as_deref(),
                last_name.as_deref(),
            )?;
            println!("User added to group {group_id}.");
        }

        Command::RemoveFromGroup { group_id, user_id } => {
            client.remove_from_group(group_id, user_id)?;
            println!("User {user_id} removed from group {group_id}.");
        }

        Command::Friends => {
            let friends = client.get_friends()?;
            json_or!(json, friends, display::print_friends);
        }

        Command::Friend { id } => {
            let friend = client.get_friend(id)?;
            json_or!(json, friend, display::print_friend);
        }

        Command::AddFriend {
            email,
            first_name,
            last_name,
        } => {
            let friends =
                client.add_friend(&email, first_name.as_deref(), last_name.as_deref())?;
            if json {
                println!("{}", serde_json::to_string_pretty(&friends)?);
            } else {
                for f in &friends {
                    println!(
                        "Added: {} (ID: {})",
                        display::display_name(&f.first_name, f.last_name.as_deref()),
                        f.id
                    );
                }
            }
        }

        Command::DeleteFriend { id } => {
            client.delete_friend(id)?;
            println!("Friend {id} deleted.");
        }

        Command::Expenses {
            group_id,
            friend_id,
            after,
            before,
            limit,
            offset,
        } => {
            let mut params: Vec<(String, String)> = vec![("limit".into(), limit.to_string())];
            if let Some(g) = group_id {
                params.push(("group_id".into(), g.to_string()));
            }
            if let Some(f) = friend_id {
                params.push(("friend_id".into(), f.to_string()));
            }
            if let Some(a) = &after {
                params.push(("dated_after".into(), a.clone()));
            }
            if let Some(b) = &before {
                params.push(("dated_before".into(), b.clone()));
            }
            if let Some(o) = offset {
                params.push(("offset".into(), o.to_string()));
            }
            let refs: Vec<(&str, &str)> = params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let expenses = client.get_expenses(&refs)?;
            json_or!(json, expenses, display::print_expenses);
        }

        Command::Expense { id } => {
            let expense = client.get_expense(id)?;
            json_or!(json, expense, display::print_expense);
        }

        Command::CreateExpense {
            description,
            cost,
            group_id,
            currency,
            category_id,
            date,
            split_equally,
            users,
        } => {
            let mut form: Vec<(String, String)> = vec![
                ("description".into(), description),
                ("cost".into(), cost),
                ("group_id".into(), group_id.to_string()),
            ];
            if let Some(c) = currency {
                form.push(("currency_code".into(), c));
            }
            if let Some(c) = category_id {
                form.push(("category_id".into(), c.to_string()));
            }
            if let Some(d) = date {
                form.push(("date".into(), d));
            }
            if split_equally {
                form.push(("split_equally".into(), "true".into()));
            } else if !users.is_empty() {
                for (i, u) in users.iter().enumerate() {
                    let parts: Vec<&str> = u.split(':').collect();
                    if parts.len() != 3 {
                        bail!("--user format: uid:paid_share:owed_share (got \"{u}\")");
                    }
                    form.push((format!("users__{i}__user_id"), parts[0].into()));
                    form.push((format!("users__{i}__paid_share"), parts[1].into()));
                    form.push((format!("users__{i}__owed_share"), parts[2].into()));
                }
            }
            let expenses = client.create_expense(&form)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&expenses)?);
            } else if let Some(e) = expenses.first() {
                println!("Expense created (ID: {})", e.id);
                display::print_expense(e);
            }
        }

        Command::UpdateExpense {
            id,
            description,
            cost,
            group_id,
            currency,
            category_id,
            date,
        } => {
            let mut form: Vec<(String, String)> = Vec::new();
            if let Some(d) = description {
                form.push(("description".into(), d));
            }
            if let Some(c) = cost {
                form.push(("cost".into(), c));
            }
            if let Some(g) = group_id {
                form.push(("group_id".into(), g.to_string()));
            }
            if let Some(c) = currency {
                form.push(("currency_code".into(), c));
            }
            if let Some(c) = category_id {
                form.push(("category_id".into(), c.to_string()));
            }
            if let Some(d) = date {
                form.push(("date".into(), d));
            }
            let expenses = client.update_expense(id, &form)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&expenses)?);
            } else if let Some(e) = expenses.first() {
                println!("Expense {id} updated.");
                display::print_expense(e);
            }
        }

        Command::DeleteExpense { id } => {
            client.delete_expense(id)?;
            println!("Expense {id} deleted.");
        }

        Command::RestoreExpense { id } => {
            client.restore_expense(id)?;
            println!("Expense {id} restored.");
        }

        Command::Comments { expense_id } => {
            let comments = client.get_comments(expense_id)?;
            json_or!(json, comments, display::print_comments);
        }

        Command::CreateComment {
            expense_id,
            content,
        } => {
            let comment = client.create_comment(expense_id, &content)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&comment)?);
            } else {
                println!("Comment created (ID: {})", comment.id);
                display::print_comment(&comment);
            }
        }

        Command::DeleteComment { comment_id } => {
            client.delete_comment(comment_id)?;
            println!("Comment {comment_id} deleted.");
        }

        Command::Balances { group_id } => {
            if let Some(gid) = group_id {
                let group = client.get_group(gid)?;
                if json {
                    let debts = if !group.simplified_debts.is_empty() {
                        &group.simplified_debts
                    } else {
                        &group.original_debts
                    };
                    println!("{}", serde_json::to_string_pretty(debts)?);
                } else {
                    display::print_group_balances(&group);
                }
            } else {
                let friends = client.get_friends()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&friends)?);
                } else {
                    display::print_balances(&friends);
                }
            }
        }

        Command::Currencies => {
            let currencies = client.get_currencies()?;
            json_or!(json, currencies, display::print_currencies);
        }

        Command::Categories => {
            let categories = client.get_categories()?;
            json_or!(json, categories, display::print_categories);
        }

        Command::Notifications { after, limit } => {
            let mut params: Vec<(String, String)> = Vec::new();
            if let Some(a) = &after {
                params.push(("updated_after".into(), a.clone()));
            }
            if let Some(l) = limit {
                params.push(("limit".into(), l.to_string()));
            }
            let refs: Vec<(&str, &str)> = params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            let notifications = client.get_notifications(&refs)?;
            json_or!(json, notifications, display::print_notifications);
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
