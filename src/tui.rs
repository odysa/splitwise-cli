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
use crate::models::{Balance, Expense, Friend, Group, GroupMember};

// ── Group drill-down ──

#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupTab {
    Members,
    Expenses,
    Debts,
}

impl GroupTab {
    const ALL: [GroupTab; 3] = [GroupTab::Members, GroupTab::Expenses, GroupTab::Debts];

    fn title(self) -> &'static str {
        match self {
            GroupTab::Members => " Members ",
            GroupTab::Expenses => " Expenses ",
            GroupTab::Debts => " Debts ",
        }
    }

    fn index(self) -> usize {
        match self {
            GroupTab::Members => 0,
            GroupTab::Expenses => 1,
            GroupTab::Debts => 2,
        }
    }

    fn next(self) -> Self {
        match self {
            GroupTab::Members => GroupTab::Expenses,
            GroupTab::Expenses => GroupTab::Debts,
            GroupTab::Debts => GroupTab::Members,
        }
    }

    fn prev(self) -> Self {
        match self {
            GroupTab::Members => GroupTab::Debts,
            GroupTab::Expenses => GroupTab::Members,
            GroupTab::Debts => GroupTab::Expenses,
        }
    }
}

struct GroupView {
    group: Group,
    expenses: Vec<Expense>,
    tab: GroupTab,
    member_state: ListState,
    expense_state: ListState,
    debt_idx: usize,
}

impl GroupView {
    fn new(group: Group, expenses: Vec<Expense>) -> Self {
        let mut member_state = ListState::default();
        let mut expense_state = ListState::default();
        if !group.members.is_empty() {
            member_state.select(Some(0));
        }
        if !expenses.is_empty() {
            expense_state.select(Some(0));
        }
        Self {
            group,
            expenses,
            tab: GroupTab::Members,
            member_state,
            expense_state,
            debt_idx: 0,
        }
    }

    fn debts(&self) -> &[crate::models::Debt] {
        if !self.group.simplified_debts.is_empty() {
            &self.group.simplified_debts
        } else {
            &self.group.original_debts
        }
    }

    fn list_len(&self) -> usize {
        match self.tab {
            GroupTab::Members => self.group.members.len(),
            GroupTab::Expenses => self.expenses.len(),
            GroupTab::Debts => self.debts().len(),
        }
    }

    fn move_down(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        match self.tab {
            GroupTab::Members => {
                let i = self.member_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
                self.member_state.select(Some(i));
            }
            GroupTab::Expenses => {
                let i = self.expense_state.selected().map(|i| (i + 1) % len).unwrap_or(0);
                self.expense_state.select(Some(i));
            }
            GroupTab::Debts => {
                self.debt_idx = (self.debt_idx + 1) % len;
            }
        }
    }

    fn move_up(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        match self.tab {
            GroupTab::Members => {
                let i = self.member_state.selected().map(|i| if i == 0 { len - 1 } else { i - 1 }).unwrap_or(0);
                self.member_state.select(Some(i));
            }
            GroupTab::Expenses => {
                let i = self.expense_state.selected().map(|i| if i == 0 { len - 1 } else { i - 1 }).unwrap_or(0);
                self.expense_state.select(Some(i));
            }
            GroupTab::Debts => {
                self.debt_idx = if self.debt_idx == 0 { len - 1 } else { self.debt_idx - 1 };
            }
        }
    }

    fn jump_top(&mut self) {
        if self.list_len() == 0 {
            return;
        }
        match self.tab {
            GroupTab::Members => self.member_state.select(Some(0)),
            GroupTab::Expenses => self.expense_state.select(Some(0)),
            GroupTab::Debts => self.debt_idx = 0,
        }
    }

    fn jump_bottom(&mut self) {
        let len = self.list_len();
        if len == 0 {
            return;
        }
        match self.tab {
            GroupTab::Members => self.member_state.select(Some(len - 1)),
            GroupTab::Expenses => self.expense_state.select(Some(len - 1)),
            GroupTab::Debts => self.debt_idx = len - 1,
        }
    }

    fn member_names(&self) -> std::collections::HashMap<u64, String> {
        self.group
            .members
            .iter()
            .map(|m| (m.id, display_name(&m.first_name, m.last_name.as_deref())))
            .collect()
    }
}

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
    group_view: Option<GroupView>,
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
            group_view: None,
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
        if let Some(gv) = &self.group_view {
            let gid = gv.group.id.to_string();
            let group = client.get_group(gv.group.id).ok();
            let expenses = client
                .get_expenses(&[("group_id", &gid), ("limit", "50")])
                .unwrap_or_default();
            if let Some(group) = group {
                self.group_view = Some(GroupView::new(group, expenses));
            }
            return;
        }
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

    fn enter_group(&mut self, client: &Client) {
        if self.tab != Tab::Groups || self.group_view.is_some() {
            return;
        }
        let Some(idx) = self.group_state.selected() else {
            return;
        };
        let Some(g) = self.groups.get(idx) else {
            return;
        };
        let gid = g.id;
        let group = client.get_group(gid).unwrap_or_else(|_| g.clone());
        let gid_str = gid.to_string();
        let expenses = client
            .get_expenses(&[("group_id", &gid_str), ("limit", "50")])
            .unwrap_or_default();
        self.group_view = Some(GroupView::new(group, expenses));
    }

    fn exit_group(&mut self) {
        self.group_view = None;
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

            // Inside group drill-down
            if let Some(gv) = &mut app.group_view {
                match key.code {
                    KeyCode::Esc | KeyCode::Backspace => app.exit_group(),
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Tab => gv.tab = gv.tab.next(),
                    KeyCode::BackTab => gv.tab = gv.tab.prev(),
                    KeyCode::Char('1') => gv.tab = GroupTab::Members,
                    KeyCode::Char('2') => gv.tab = GroupTab::Expenses,
                    KeyCode::Char('3') => gv.tab = GroupTab::Debts,
                    KeyCode::Down | KeyCode::Char('j') => gv.move_down(),
                    KeyCode::Up | KeyCode::Char('k') => gv.move_up(),
                    KeyCode::Char('g') | KeyCode::Home => gv.jump_top(),
                    KeyCode::Char('G') | KeyCode::End => gv.jump_bottom(),
                    KeyCode::Char('r') => app.refresh(client),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
                    KeyCode::Enter => app.enter_group(client),
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
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}

// ── UI ──

fn ui(frame: &mut Frame, app: &mut App) {
    if let Some(gv) = &mut app.group_view {
        render_group_view(frame, gv);
        return;
    }

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
    let help = status_bar(&[
        ("j/k", "nav"),
        ("Enter", "open"),
        ("Tab", "switch"),
        ("1-3", "jump"),
        ("g/G", "top/btm"),
        ("r", "refresh"),
        ("q", "quit"),
    ]);
    frame.render_widget(help, status_area);
}

fn status_bar<'a>(bindings: &[(&'a str, &'a str)]) -> Paragraph<'a> {
    let mut spans = vec![Span::raw(" ")];
    for (key, desc) in bindings {
        spans.push(Span::styled(*key, Style::default().fg(Color::White)));
        spans.push(Span::styled(
            format!(":{desc}  "),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Paragraph::new(Line::from(spans))
}

// ── Group drill-down view ──

fn render_group_view(frame: &mut Frame, gv: &mut GroupView) {
    let [header_area, tab_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    // ── Group header ──
    let gtype = gv.group.group_type.as_deref().unwrap_or("");
    let header_text = format!(" {} ", gv.group.name);
    let header_sub = if gtype.is_empty() {
        format!("ID: {}", gv.group.id)
    } else {
        format!("ID: {} | Type: {gtype}", gv.group.id)
    };
    let header = Paragraph::new(vec![
        Line::styled(
            header_text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(" {header_sub}"),
            Style::default().fg(Color::DarkGray),
        ),
    ])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(header, header_area);

    // ── Sub-tabs ──
    let titles: Vec<&str> = GroupTab::ALL.iter().map(|t| t.title()).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .select(gv.tab.index());
    frame.render_widget(tabs, tab_area);

    // ── Content ──
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .areas(main_area);

    match gv.tab {
        GroupTab::Members => render_gv_members(frame, gv, list_area, detail_area),
        GroupTab::Expenses => render_gv_expenses(frame, gv, list_area, detail_area),
        GroupTab::Debts => render_gv_debts(frame, gv, list_area, detail_area),
    }

    // ── Status bar ──
    let help = status_bar(&[
        ("j/k", "nav"),
        ("Tab", "switch"),
        ("1-3", "jump"),
        ("r", "refresh"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
    frame.render_widget(help, status_area);
}

fn render_gv_members(
    frame: &mut Frame,
    gv: &mut GroupView,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items: Vec<ListItem> = gv
        .group
        .members
        .iter()
        .map(|m| {
            let name = display_name(&m.first_name, m.last_name.as_deref());
            let mut spans = vec![Span::raw(format!("{:<20} ", trunc(&name, 20)))];
            spans.extend(balance_text(&m.balance));
            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Members ({}) ", gv.group.members.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut gv.member_state);

    let detail = if let Some(idx) = gv.member_state.selected() {
        if let Some(m) = gv.group.members.get(idx) {
            member_detail(m)
        } else {
            vec![]
        }
    } else {
        vec![Line::styled(
            "No member selected",
            Style::default().fg(Color::DarkGray),
        )]
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Member "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn member_detail(m: &GroupMember) -> Vec<Line<'static>> {
    let name = display_name(&m.first_name, m.last_name.as_deref());
    let mut lines = vec![
        Line::styled(name, Style::default().add_modifier(Modifier::BOLD)),
        Line::raw(format!("ID: {}", m.id)),
    ];
    if let Some(email) = &m.email {
        lines.push(Line::raw(format!("Email: {email}")));
    }
    lines.push(Line::raw(""));

    let nonzero: Vec<&Balance> = m.balance.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        lines.push(Line::styled(
            "Settled up in this group",
            Style::default().fg(Color::Green),
        ));
    } else {
        lines.push(Line::styled(
            "Balance in group:",
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
    lines
}

fn render_gv_expenses(
    frame: &mut Frame,
    gv: &mut GroupView,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let items: Vec<ListItem> = gv
        .expenses
        .iter()
        .map(|e| {
            let date = e.date.as_deref().map(date_short).unwrap_or("---");
            let desc = trunc(&e.description, 18);
            let del = if e.deleted_at.is_some() { " x" } else { "" };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{date} "), Style::default().fg(Color::DarkGray)),
                Span::raw(format!("{desc:<18} ")),
                Span::styled(
                    format!("{} {}{del}", e.cost, e.currency_code),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Expenses ({}) ", gv.expenses.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, list_area, &mut gv.expense_state);

    let detail = if let Some(idx) = gv.expense_state.selected() {
        if let Some(e) = gv.expenses.get(idx) {
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
        .block(Block::default().borders(Borders::ALL).title(" Expense "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn render_gv_debts(
    frame: &mut Frame,
    gv: &mut GroupView,
    list_area: ratatui::layout::Rect,
    detail_area: ratatui::layout::Rect,
) {
    let names = gv.member_names();
    let debts = gv.debts();
    let is_simplified = !gv.group.simplified_debts.is_empty();

    let items: Vec<ListItem> = debts
        .iter()
        .map(|d| {
            let from = names.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
            let to = names.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
            ListItem::new(Line::from(vec![
                Span::raw(format!("{from} ")),
                Span::styled("->", Style::default().fg(Color::DarkGray)),
                Span::raw(format!(" {to}  ")),
                Span::styled(
                    format!("{} {}", d.amount, d.currency_code),
                    Style::default().fg(Color::Yellow),
                ),
            ]))
        })
        .collect();

    let title = if is_simplified {
        " Debts (simplified) "
    } else {
        " Debts "
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{title}({}) ", debts.len())),
        )
        .highlight_style(HIGHLIGHT)
        .highlight_symbol("> ");

    // Debts use a manual index since we don't have a ListState for it
    let mut debt_list_state = ListState::default();
    if !debts.is_empty() {
        debt_list_state.select(Some(gv.debt_idx));
    }
    frame.render_stateful_widget(list, list_area, &mut debt_list_state);

    // Detail: show the selected debt with context
    let detail = if let Some(d) = debts.get(gv.debt_idx) {
        let from = names.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
        let to = names.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
        vec![
            Line::styled(
                format!("{from} owes {to}"),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Line::raw(""),
            Line::from(vec![
                Span::raw("Amount: "),
                Span::styled(
                    format!("{} {}", d.amount, d.currency_code),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::raw(""),
            Line::raw(format!("From: {from} (ID: {})", d.from)),
            Line::raw(format!("To:   {to} (ID: {})", d.to)),
        ]
    } else {
        vec![Line::styled(
            "No debts in this group",
            Style::default().fg(Color::Green),
        )]
    };
    let para = Paragraph::new(detail)
        .block(Block::default().borders(Borders::ALL).title(" Debt "))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
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
