use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
    DefaultTerminal, Frame,
};

use crate::client::Client;
use crate::display::display_name;
use crate::models::{Balance, Expense, Friend, Group};

// ── Tab ──

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Friends,
    Groups,
    Expenses,
}

impl Tab {
    const ALL: [Tab; 3] = [Tab::Friends, Tab::Groups, Tab::Expenses];

    fn title(self) -> &'static str {
        match self {
            Tab::Friends => " Friends ",
            Tab::Groups => " Groups ",
            Tab::Expenses => " Expenses ",
        }
    }

    fn next(self) -> Self {
        match self {
            Tab::Friends => Tab::Groups,
            Tab::Groups => Tab::Expenses,
            Tab::Expenses => Tab::Friends,
        }
    }

    fn prev(self) -> Self {
        match self {
            Tab::Friends => Tab::Expenses,
            Tab::Groups => Tab::Friends,
            Tab::Expenses => Tab::Groups,
        }
    }

    fn index(self) -> usize {
        match self {
            Tab::Friends => 0,
            Tab::Groups => 1,
            Tab::Expenses => 2,
        }
    }
}

// ── App state ──

struct App {
    tab: Tab,
    friends: Vec<Friend>,
    groups: Vec<Group>,
    expenses: Vec<Expense>,
    friend_state: ListState,
    group_state: ListState,
    expense_state: ListState,
    should_quit: bool,
}

impl App {
    fn new(client: &Client) -> Self {
        let friends = client.get_friends().unwrap_or_default();
        let groups = client.get_groups().unwrap_or_default();
        let expenses = client
            .get_expenses(&[("limit", "50")])
            .unwrap_or_default();

        let mut app = Self {
            tab: Tab::Friends,
            friends,
            groups,
            expenses,
            friend_state: ListState::default(),
            group_state: ListState::default(),
            expense_state: ListState::default(),
            should_quit: false,
        };
        if !app.friends.is_empty() {
            app.friend_state.select(Some(0));
        }
        if !app.groups.is_empty() {
            app.group_state.select(Some(0));
        }
        if !app.expenses.is_empty() {
            app.expense_state.select(Some(0));
        }
        app
    }

    fn list_len(&self) -> usize {
        match self.tab {
            Tab::Friends => self.friends.len(),
            Tab::Groups => self.groups.len(),
            Tab::Expenses => self.expenses.len(),
        }
    }

    fn selected(&self) -> Option<usize> {
        match self.tab {
            Tab::Friends => self.friend_state.selected(),
            Tab::Groups => self.group_state.selected(),
            Tab::Expenses => self.expense_state.selected(),
        }
    }

    fn state_mut(&mut self) -> &mut ListState {
        match self.tab {
            Tab::Friends => &mut self.friend_state,
            Tab::Groups => &mut self.group_state,
            Tab::Expenses => &mut self.expense_state,
        }
    }

    fn move_down(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let i = self.selected().map(|i| (i + 1) % len).unwrap_or(0);
        self.state_mut().select(Some(i));
    }

    fn move_up(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        let i = self
            .selected()
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.state_mut().select(Some(i));
    }

    fn jump_top(&mut self) {
        if self.list_len() > 0 {
            self.state_mut().select(Some(0));
        }
    }

    fn jump_bottom(&mut self) {
        let len = self.list_len();
        if len > 0 {
            self.state_mut().select(Some(len - 1));
        }
    }

    fn refresh(&mut self, client: &Client) {
        match self.tab {
            Tab::Friends => {
                self.friends = client.get_friends().unwrap_or_default();
                clamp_selection(&mut self.friend_state, self.friends.len());
            }
            Tab::Groups => {
                self.groups = client.get_groups().unwrap_or_default();
                clamp_selection(&mut self.group_state, self.groups.len());
            }
            Tab::Expenses => {
                self.expenses = client
                    .get_expenses(&[("limit", "50")])
                    .unwrap_or_default();
                clamp_selection(&mut self.expense_state, self.expenses.len());
            }
        }
    }
}

fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else if state.selected().is_none() {
        state.select(Some(0));
    } else if let Some(i) = state.selected() {
        if i >= len {
            state.select(Some(len - 1));
        }
    }
}

// ── Entry point ──

pub fn run(client: &Client) -> Result<()> {
    eprint!("Loading...");
    let mut app = App::new(client);
    eprintln!(" done.");

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut app, client);
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, client: &Client) -> Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                KeyCode::Tab => app.tab = app.tab.next(),
                KeyCode::BackTab => app.tab = app.tab.prev(),
                KeyCode::Char('1') => app.tab = Tab::Friends,
                KeyCode::Char('2') => app.tab = Tab::Groups,
                KeyCode::Char('3') => app.tab = Tab::Expenses,
                KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                KeyCode::Char('g') | KeyCode::Home => app.jump_top(),
                KeyCode::Char('G') | KeyCode::End => app.jump_bottom(),
                KeyCode::Char('r') => app.refresh(client),
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

// ── UI ──

fn ui(frame: &mut Frame, app: &mut App) {
    let [tab_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    // ── Tabs ──
    let titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title(" splitwise "))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .select(app.tab.index());
    frame.render_widget(tabs, tab_area);

    // ── Main: list + detail ──
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .areas(main_area);

    match app.tab {
        Tab::Friends => render_friends(frame, app, list_area, detail_area),
        Tab::Groups => render_groups(frame, app, list_area, detail_area),
        Tab::Expenses => render_expenses(frame, app, list_area, detail_area),
    }

    // ── Status bar ──
    let help = Paragraph::new(Line::from(vec![
        Span::styled(" j/k", Style::default().fg(Color::White)),
        Span::styled(":nav  ", Style::default().fg(Color::DarkGray)),
        Span::styled("Tab", Style::default().fg(Color::White)),
        Span::styled(":switch  ", Style::default().fg(Color::DarkGray)),
        Span::styled("1-3", Style::default().fg(Color::White)),
        Span::styled(":jump  ", Style::default().fg(Color::DarkGray)),
        Span::styled("g/G", Style::default().fg(Color::White)),
        Span::styled(":top/bottom  ", Style::default().fg(Color::DarkGray)),
        Span::styled("r", Style::default().fg(Color::White)),
        Span::styled(":refresh  ", Style::default().fg(Color::DarkGray)),
        Span::styled("q", Style::default().fg(Color::White)),
        Span::styled(":quit", Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(help, status_area);
}

// ── Helpers ──

const HIGHLIGHT: Style = Style::new()
    .bg(Color::DarkGray)
    .add_modifier(Modifier::BOLD);

fn balance_style(amount: &str) -> Style {
    let v: f64 = amount.parse().unwrap_or(0.0);
    if v > 0.0 {
        Style::default().fg(Color::Green)
    } else if v < 0.0 {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn balance_text(balances: &[Balance]) -> Vec<Span<'_>> {
    let nonzero: Vec<&Balance> = balances.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        return vec![Span::styled("settled", Style::default().fg(Color::DarkGray))];
    }
    let mut spans = Vec::new();
    for (i, b) in nonzero.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(", "));
        }
        let label = format!("{} {}", b.amount, b.currency_code);
        spans.push(Span::styled(label, balance_style(&b.amount)));
    }
    spans
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{truncated}~")
    }
}

fn date_short(s: &str) -> &str {
    &s[..s.len().min(10)]
}

// ── Friends ──

fn render_friends(
    frame: &mut Frame,
    app: &mut App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items: Vec<ListItem> = app
        .friends
        .iter()
        .map(|f| {
            let name = display_name(&f.first_name, f.last_name.as_deref());
            let mut spans = vec![Span::raw(format!("{:<18} ", trunc(&name, 18)))];
            spans.extend(balance_text(&f.balance));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Friends ({}) ", app.friends.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut app.friend_state);

    // Detail
    let detail = if let Some(idx) = app.friend_state.selected() {
        if let Some(f) = app.friends.get(idx) {
            friend_detail(f)
        } else {
            vec![]
        }
    } else {
        vec![Line::styled(
            "No friend selected",
            Style::default().fg(Color::DarkGray),
        )]
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn friend_detail(f: &Friend) -> Vec<Line<'_>> {
    let name = display_name(&f.first_name, f.last_name.as_deref());
    let mut lines = vec![
        Line::styled(name, Style::default().add_modifier(Modifier::BOLD)),
        Line::raw(format!("ID: {}", f.id)),
    ];
    if let Some(email) = &f.email {
        lines.push(Line::raw(format!("Email: {email}")));
    }
    lines.push(Line::raw(""));

    let nonzero: Vec<&Balance> = f.balance.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        lines.push(Line::styled(
            "All settled up",
            Style::default().fg(Color::Green),
        ));
    } else {
        lines.push(Line::styled(
            "Balance:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        for b in &nonzero {
            let amt: f64 = b.amount.parse().unwrap_or(0.0);
            let (label, style) = if amt > 0.0 {
                (
                    format!("  owes you {} {}", b.amount, b.currency_code),
                    Style::default().fg(Color::Green),
                )
            } else {
                (
                    format!(
                        "  you owe {} {}",
                        b.amount.trim_start_matches('-'),
                        b.currency_code
                    ),
                    Style::default().fg(Color::Red),
                )
            };
            lines.push(Line::styled(label, style));
        }
    }

    if !f.groups.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("Shared groups: {}", f.groups.len()),
            Style::default().fg(Color::DarkGray),
        ));
    }
    lines
}

// ── Groups ──

fn render_groups(
    frame: &mut Frame,
    app: &mut App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items: Vec<ListItem> = app
        .groups
        .iter()
        .map(|g| {
            let name = trunc(&g.name, 20);
            let members = g.members.len();
            ListItem::new(Line::from(vec![
                Span::raw(format!("{name:<20} ")),
                Span::styled(
                    format!("{members} members"),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Groups ({}) ", app.groups.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut app.group_state);

    let detail = if let Some(idx) = app.group_state.selected() {
        if let Some(g) = app.groups.get(idx) {
            group_detail(g)
        } else {
            vec![]
        }
    } else {
        vec![Line::styled(
            "No group selected",
            Style::default().fg(Color::DarkGray),
        )]
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn group_detail(g: &Group) -> Vec<Line<'_>> {
    let mut lines = vec![
        Line::styled(&*g.name, Style::default().add_modifier(Modifier::BOLD)),
        Line::raw(format!("ID: {}", g.id)),
    ];
    if let Some(t) = &g.group_type {
        lines.push(Line::raw(format!("Type: {t}")));
    }
    if let Some(s) = g.simplify_by_default {
        lines.push(Line::raw(format!("Simplify: {s}")));
    }

    if !g.members.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("Members ({}):", g.members.len()),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        for m in &g.members {
            let name = display_name(&m.first_name, m.last_name.as_deref());
            let bals: Vec<String> = m
                .balance
                .iter()
                .filter(|b| b.amount != "0.00")
                .map(|b| format!("{} {}", b.amount, b.currency_code))
                .collect();
            if bals.is_empty() {
                lines.push(Line::raw(format!("  {name}")));
            } else {
                let bal_str = bals.join(", ");
                lines.push(Line::from(vec![
                    Span::raw(format!("  {name}  ")),
                    Span::styled(bal_str, balance_style(&m.balance.first().map(|b| b.amount.as_str()).unwrap_or("0"))),
                ]));
            }
        }
    }

    let debts = if !g.simplified_debts.is_empty() {
        Some(("Debts (simplified)", &g.simplified_debts))
    } else if !g.original_debts.is_empty() {
        Some(("Debts", &g.original_debts))
    } else {
        None
    };
    if let Some((label, debts)) = debts {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            format!("{label}:"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let members: std::collections::HashMap<u64, String> = g
            .members
            .iter()
            .map(|m| (m.id, display_name(&m.first_name, m.last_name.as_deref())))
            .collect();
        for d in debts {
            let from = members.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
            let to = members.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
            lines.push(Line::styled(
                format!("  {from} -> {to}: {} {}", d.amount, d.currency_code),
                Style::default().fg(Color::Yellow),
            ));
        }
    }
    lines
}

// ── Expenses ──

fn render_expenses(
    frame: &mut Frame,
    app: &mut App,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items: Vec<ListItem> = app
        .expenses
        .iter()
        .map(|e| {
            let date = e.date.as_deref().map(date_short).unwrap_or("---");
            let desc = trunc(&e.description, 18);
            let del = if e.deleted_at.is_some() { " x" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{date} "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(format!("{desc:<18} ")),
                Span::styled(
                    format!("{}{del}", e.cost),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Expenses ({}) ", app.expenses.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut app.expense_state);

    let detail = if let Some(idx) = app.expense_state.selected() {
        if let Some(e) = app.expenses.get(idx) {
            expense_detail(e)
        } else {
            vec![]
        }
    } else {
        vec![Line::styled(
            "No expense selected",
            Style::default().fg(Color::DarkGray),
        )]
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn expense_detail(e: &Expense) -> Vec<Line<'_>> {
    let mut lines = vec![Line::styled(
        &*e.description,
        Style::default().add_modifier(Modifier::BOLD),
    )];
    lines.push(Line::raw(format!("ID: {}", e.id)));
    lines.push(Line::from(vec![
        Span::raw("Cost: "),
        Span::styled(
            format!("{} {}", e.cost, e.currency_code),
            Style::default().fg(Color::Cyan),
        ),
    ]));
    if let Some(date) = &e.date {
        lines.push(Line::raw(format!("Date: {}", date_short(date))));
    }
    if let Some(cat) = &e.category {
        lines.push(Line::raw(format!("Category: {}", cat.name)));
    }
    if let Some(creator) = &e.created_by {
        lines.push(Line::raw(format!(
            "Created by: {}",
            display_name(&creator.first_name, creator.last_name.as_deref())
        )));
    }
    if e.deleted_at.is_some() {
        lines.push(Line::styled(
            "DELETED",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    if !e.users.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Shares:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::styled(
            format!(
                "  {:<16} {:>8} {:>8} {:>9}",
                "Name", "Paid", "Owed", "Net"
            ),
            Style::default().fg(Color::DarkGray),
        ));
        for s in &e.users {
            let name = display_name(
                s.first_name.as_deref().unwrap_or("?"),
                s.last_name.as_deref(),
            );
            let net_style = balance_style(&s.net_balance);
            lines.push(Line::from(vec![
                Span::raw(format!(
                    "  {:<16} {:>8} {:>8} ",
                    trunc(&name, 16),
                    s.paid_share,
                    s.owed_share,
                )),
                Span::styled(format!("{:>9}", s.net_balance), net_style),
            ]));
        }
    }

    if !e.repayments.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "Repayments:",
            Style::default().add_modifier(Modifier::BOLD),
        ));
        for r in &e.repayments {
            lines.push(Line::styled(
                format!("  {} -> {}: {}", r.from, r.to, r.amount),
                Style::default().fg(Color::Yellow),
            ));
        }
    }
    lines
}
