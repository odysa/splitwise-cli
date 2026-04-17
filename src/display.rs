use crate::models::*;

pub fn display_name(first: &str, last: Option<&str>) -> String {
    match last {
        Some(l) if !l.is_empty() => format!("{first} {l}"),
        _ => first.to_string(),
    }
}

fn strip_html(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }
    result
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}

fn date_short(s: &str) -> &str {
    &s[..s.len().min(10)]
}

// ── User ──

pub fn print_user(user: &User) {
    println!("ID:       {}", user.id);
    println!(
        "Name:     {}",
        display_name(&user.first_name, user.last_name.as_deref())
    );
    if let Some(email) = &user.email {
        println!("Email:    {email}");
    }
    if let Some(currency) = &user.default_currency {
        println!("Currency: {currency}");
    }
    if let Some(locale) = &user.locale {
        println!("Locale:   {locale}");
    }
    if let Some(n) = user.notifications_count {
        if n > 0 {
            println!("Notifications: {n}");
        }
    }
}

// ── Groups ──

pub fn print_groups(groups: &[Group]) {
    if groups.is_empty() {
        println!("No groups found.");
        return;
    }
    println!("{:<8}  {:<30}  {:<12}  Members", "ID", "Name", "Type");
    println!("{}", "-".repeat(62));
    for g in groups {
        let gtype = g.group_type.as_deref().unwrap_or("-");
        println!(
            "{:<8}  {:<30}  {:<12}  {}",
            g.id,
            truncate(&g.name, 30),
            gtype,
            g.members.len()
        );
    }
}

pub fn print_group(group: &Group) {
    println!("ID:       {}", group.id);
    println!("Name:     {}", group.name);
    if let Some(t) = &group.group_type {
        println!("Type:     {t}");
    }
    if let Some(s) = group.simplify_by_default {
        println!("Simplify: {s}");
    }
    if let Some(link) = &group.invite_link {
        println!("Invite:   {link}");
    }

    if !group.members.is_empty() {
        println!("\nMembers:");
        for m in &group.members {
            let name = display_name(&m.first_name, m.last_name.as_deref());
            let bals: Vec<String> = m
                .balance
                .iter()
                .filter(|b| b.amount != "0.00")
                .map(|b| format!("{} {}", b.amount, b.currency_code))
                .collect();
            if bals.is_empty() {
                println!("  {name} ({})", m.id);
            } else {
                println!("  {name} ({}) [{}]", m.id, bals.join(", "));
            }
        }
    }

    let debts = if !group.simplified_debts.is_empty() {
        Some(("simplified", &group.simplified_debts))
    } else if !group.original_debts.is_empty() {
        Some(("original", &group.original_debts))
    } else {
        None
    };
    if let Some((label, debts)) = debts {
        println!("\nDebts ({label}):");
        for d in debts {
            println!(
                "  {} -> {}: {} {}",
                d.from, d.to, d.amount, d.currency_code
            );
        }
    }
}

// ── Friends ──

pub fn print_friends(friends: &[Friend]) {
    if friends.is_empty() {
        println!("No friends found.");
        return;
    }
    println!("{:<8}  {:<25}  Balance", "ID", "Name");
    println!("{}", "-".repeat(55));
    for f in friends {
        let name = display_name(&f.first_name, f.last_name.as_deref());
        let bals: Vec<String> = f
            .balance
            .iter()
            .filter(|b| b.amount != "0.00")
            .map(|b| format!("{} {}", b.amount, b.currency_code))
            .collect();
        let bal = if bals.is_empty() {
            "settled up".into()
        } else {
            bals.join(", ")
        };
        println!("{:<8}  {:<25}  {bal}", f.id, truncate(&name, 25));
    }
}

pub fn print_friend(friend: &Friend) {
    println!("ID:    {}", friend.id);
    println!(
        "Name:  {}",
        display_name(&friend.first_name, friend.last_name.as_deref())
    );
    if let Some(email) = &friend.email {
        println!("Email: {email}");
    }

    let nonzero: Vec<&Balance> = friend
        .balance
        .iter()
        .filter(|b| b.amount != "0.00")
        .collect();
    if nonzero.is_empty() {
        println!("Balance: settled up");
    } else {
        println!("\nBalance:");
        for b in nonzero {
            let amt: f64 = b.amount.parse().unwrap_or(0.0);
            if amt > 0.0 {
                println!("  owes you {} {}", b.amount, b.currency_code);
            } else {
                println!("  you owe {} {}", b.amount.trim_start_matches('-'), b.currency_code);
            }
        }
    }
}

// ── Expenses ──

pub fn print_expenses(expenses: &[Expense]) {
    if expenses.is_empty() {
        println!("No expenses found.");
        return;
    }
    println!(
        "{:<8}  {:<30}  {:<10}  {:<5}  Date",
        "ID", "Description", "Cost", "Curr"
    );
    println!("{}", "-".repeat(72));
    for e in expenses {
        let date = e.date.as_deref().map(date_short).unwrap_or("-");
        let del = if e.deleted_at.is_some() {
            " [deleted]"
        } else {
            ""
        };
        println!(
            "{:<8}  {:<30}  {:<10}  {:<5}  {date}{del}",
            e.id,
            truncate(&e.description, 30),
            e.cost,
            e.currency_code,
        );
    }
}

pub fn print_expense(expense: &Expense) {
    println!("ID:          {}", expense.id);
    println!("Description: {}", expense.description);
    println!("Cost:        {} {}", expense.cost, expense.currency_code);
    if let Some(date) = &expense.date {
        println!("Date:        {}", date_short(date));
    }
    if let Some(cat) = &expense.category {
        println!("Category:    {}", cat.name);
    }
    if let Some(creator) = &expense.created_by {
        println!(
            "Created by:  {} ({})",
            display_name(&creator.first_name, creator.last_name.as_deref()),
            creator.id
        );
    }
    if expense.deleted_at.is_some() {
        println!("Status:      DELETED");
    }
    if let Some(interval) = &expense.repeat_interval {
        println!("Repeats:     {interval}");
    }

    if !expense.users.is_empty() {
        println!("\nShares:");
        println!(
            "  {:<8}  {:<20}  {:<10}  {:<10}  Net",
            "UserID", "Name", "Paid", "Owed"
        );
        println!("  {}", "-".repeat(58));
        for s in &expense.users {
            let name = display_name(
                s.first_name.as_deref().unwrap_or("?"),
                s.last_name.as_deref(),
            );
            println!(
                "  {:<8}  {:<20}  {:<10}  {:<10}  {}",
                s.user_id,
                truncate(&name, 20),
                s.paid_share,
                s.owed_share,
                s.net_balance
            );
        }
    }

    if !expense.repayments.is_empty() {
        println!("\nRepayments:");
        for r in &expense.repayments {
            println!("  {} -> {}: {}", r.from, r.to, r.amount);
        }
    }
}

// ── Comments ──

pub fn print_comments(comments: &[Comment]) {
    if comments.is_empty() {
        println!("No comments.");
        return;
    }
    for c in comments {
        if c.deleted_at.is_some() {
            continue;
        }
        let by = c
            .created_by
            .as_ref()
            .map(|u| display_name(&u.first_name, u.last_name.as_deref()))
            .unwrap_or_else(|| "Unknown".into());
        let date = c.created_at.as_deref().map(date_short).unwrap_or("-");
        let content = strip_html(&c.content);
        println!("[{}] {by} ({date}): {content}", c.id);
    }
}

pub fn print_comment(comment: &Comment) {
    let by = comment
        .created_by
        .as_ref()
        .map(|u| display_name(&u.first_name, u.last_name.as_deref()))
        .unwrap_or_else(|| "Unknown".into());
    println!("ID:      {}", comment.id);
    println!("By:      {by}");
    println!("Content: {}", strip_html(&comment.content));
    if let Some(date) = &comment.created_at {
        println!("Date:    {date}");
    }
}

// ── Balances ──

pub fn print_balances(friends: &[Friend]) {
    let mut found = false;
    for f in friends {
        let nonzero: Vec<&Balance> = f
            .balance
            .iter()
            .filter(|b| b.amount != "0.00")
            .collect();
        if nonzero.is_empty() {
            continue;
        }
        found = true;
        let name = display_name(&f.first_name, f.last_name.as_deref());
        for b in nonzero {
            let amt: f64 = b.amount.parse().unwrap_or(0.0);
            if amt > 0.0 {
                println!("{name} owes you {} {}", b.amount, b.currency_code);
            } else {
                println!(
                    "you owe {name} {} {}",
                    b.amount.trim_start_matches('-'),
                    b.currency_code
                );
            }
        }
    }
    if !found {
        println!("All settled up!");
    }
}

pub fn print_group_balances(group: &Group) {
    let debts = if !group.simplified_debts.is_empty() {
        &group.simplified_debts
    } else {
        &group.original_debts
    };
    if debts.is_empty() {
        println!("All settled up!");
        return;
    }
    let members: std::collections::HashMap<u64, String> = group
        .members
        .iter()
        .map(|m| (m.id, display_name(&m.first_name, m.last_name.as_deref())))
        .collect();
    for d in debts {
        let from = members.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
        let to = members.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
        println!("{from} owes {to} {} {}", d.amount, d.currency_code);
    }
}

// ── Currencies ──

pub fn print_currencies(currencies: &[Currency]) {
    println!("{:<6}  Unit", "Code");
    println!("{}", "-".repeat(30));
    for c in currencies {
        println!("{:<6}  {}", c.currency_code, c.unit);
    }
}

// ── Categories ──

pub fn print_categories(categories: &[Category]) {
    for cat in categories {
        println!("{} ({})", cat.name, cat.id);
        for sub in &cat.subcategories {
            println!("  {} ({})", sub.name, sub.id);
        }
    }
}

// ── Notifications ──

const NOTIFICATION_TYPES: &[(u64, &str)] = &[
    (0, "Expense added"),
    (1, "Expense updated"),
    (2, "Expense deleted"),
    (3, "Comment added"),
    (4, "Added to group"),
    (5, "Removed from group"),
    (6, "Group deleted"),
    (7, "Group settings changed"),
    (8, "Added as friend"),
    (9, "Removed as friend"),
    (10, "News"),
    (11, "Debt simplification"),
    (12, "Group undeleted"),
    (13, "Expense undeleted"),
    (14, "Group currency conversion"),
    (15, "Friend currency conversion"),
];

pub fn print_notifications(notifications: &[Notification]) {
    if notifications.is_empty() {
        println!("No notifications.");
        return;
    }
    for n in notifications {
        let type_name = NOTIFICATION_TYPES
            .iter()
            .find(|(id, _)| *id == n.notification_type)
            .map(|(_, name)| *name)
            .unwrap_or("Unknown");
        let content = n
            .content
            .as_deref()
            .map(|c| strip_html(c))
            .unwrap_or_default();
        let date = n.created_at.as_deref().map(date_short).unwrap_or("-");
        println!("[{date}] {type_name} | {content} ({})", n.id);
    }
}
