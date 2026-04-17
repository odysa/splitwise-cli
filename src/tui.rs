use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Padding, Paragraph, Tabs, Wrap},
    DefaultTerminal, Frame,
};

use crate::client::Client;
use crate::display::display_name;
use crate::models::{Balance, Expense, Friend, Group, GroupMember};

// ── Theme ──

const GREEN: Color = Color::Rgb(28, 186, 101); // Splitwise brand
const GREEN_DIM: Color = Color::Rgb(60, 120, 80);
const RED: Color = Color::Rgb(235, 87, 87);
const GOLD: Color = Color::Rgb(241, 196, 15);
const CYAN: Color = Color::Rgb(86, 182, 194);
const BG_HL: Color = Color::Rgb(40, 60, 50); // selected row
const FG: Color = Color::Rgb(220, 220, 220);
const FG2: Color = Color::Rgb(140, 140, 150); // secondary
const FG3: Color = Color::Rgb(80, 80, 90); // muted
const BORDER: Color = Color::Rgb(60, 65, 70);
const STATUS_BG: Color = Color::Rgb(24, 24, 30);

const HIGHLIGHT: Style = Style::new().bg(BG_HL).fg(FG).add_modifier(Modifier::BOLD);
const SEL: &str = " \u{258c} "; // left half-block selection bar

fn block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(FG).add_modifier(Modifier::BOLD),
        ))
}

fn block_focus(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(GREEN))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ))
}

fn detail_block(title: &str) -> Block<'_> {
    block(title).padding(Padding::new(1, 1, 0, 0))
}

fn label_val(label: &str, val: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(FG2)),
        Span::styled(val, Style::default().fg(FG)),
    ])
}

fn section_hdr(text: &str) -> Line<'static> {
    Line::styled(
        text.to_string(),
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
    )
}

fn status_bar<'a>(bindings: &[(&'a str, &'a str)]) -> Paragraph<'a> {
    let mut spans = vec![Span::raw(" ")];
    for (i, (key, desc)) in bindings.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" \u{2502} ", Style::default().fg(FG3)));
        }
        spans.push(Span::styled(
            format!(" {key} "),
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!("{desc}"),
            Style::default().fg(FG2),
        ));
    }
    Paragraph::new(Line::from(spans)).style(Style::default().bg(STATUS_BG))
}

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

// ── Hit-test areas ──

#[derive(Default, Clone, Copy)]
struct Areas {
    tab_bar: Rect,
    list: Rect,
    gv_sub_tabs: Rect,
    gv_list: Rect,
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
    areas: Areas,
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
            areas: Areas::default(),
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
        if len == 0 { return; }
        let i = self.selected().map(|i| (i + 1) % len).unwrap_or(0);
        self.state_mut().select(Some(i));
    }

    fn move_up(&mut self) {
        let len = self.list_len();
        if len == 0 { return; }
        let i = self.selected().map(|i| if i == 0 { len - 1 } else { i - 1 }).unwrap_or(0);
        self.state_mut().select(Some(i));
    }

    fn jump_top(&mut self) {
        if self.list_len() > 0 { self.state_mut().select(Some(0)); }
    }

    fn jump_bottom(&mut self) {
        let len = self.list_len();
        if len > 0 { self.state_mut().select(Some(len - 1)); }
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
                self.expenses = client.get_expenses(&[("limit", "50")]).unwrap_or_default();
                clamp_selection(&mut self.expense_state, self.expenses.len());
            }
        }
    }

    fn enter_group(&mut self, client: &Client) {
        if self.tab != Tab::Groups || self.group_view.is_some() { return; }
        let Some(idx) = self.group_state.selected() else { return; };
        let Some(g) = self.groups.get(idx) else { return; };
        let gid = g.id;
        let group = client.get_group(gid).unwrap_or_else(|_| g.clone());
        let gid_str = gid.to_string();
        let expenses = client
            .get_expenses(&[("group_id", &gid_str), ("limit", "50")])
            .unwrap_or_default();
        self.group_view = Some(GroupView::new(group, expenses));
    }

    fn exit_group(&mut self) { self.group_view = None; }
}

fn clamp_selection(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
    } else if state.selected().is_none() {
        state.select(Some(0));
    } else if let Some(i) = state.selected() {
        if i >= len { state.select(Some(len - 1)); }
    }
}

// ── Entry point ──

pub fn run(client: &Client) -> Result<()> {
    eprint!("Loading...");
    let mut app = App::new(client);
    eprintln!(" done.");

    let mut terminal = ratatui::init();
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let result = event_loop(&mut terminal, &mut app, client);
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, client: &Client) -> Result<()> {
    loop {
        terminal.draw(|frame| ui(frame, app))?;

        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
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
            Event::Mouse(mouse) => handle_mouse(app, client, mouse),
            _ => {}
        }

        if app.should_quit { break; }
    }
    Ok(())
}

fn handle_mouse(app: &mut App, client: &Client, mouse: crossterm::event::MouseEvent) {
    let col = mouse.column;
    let row = mouse.row;
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if let Some(gv) = &mut app.group_view {
                if let Some(idx) = tab_hit(app.areas.gv_sub_tabs, col, row, GroupTab::ALL.len()) {
                    gv.tab = GroupTab::ALL[idx];
                    return;
                }
                if let Some(clicked) = list_hit(app.areas.gv_list, col, row) {
                    let (state, len) = match gv.tab {
                        GroupTab::Members => (&mut gv.member_state, gv.group.members.len()),
                        GroupTab::Expenses => (&mut gv.expense_state, gv.expenses.len()),
                        GroupTab::Debts => {
                            let len = gv.debts().len();
                            gv.debt_idx = clicked.min(len.saturating_sub(1));
                            return;
                        }
                    };
                    let idx = clicked + state.offset();
                    if idx < len { state.select(Some(idx)); }
                }
            } else {
                if let Some(idx) = tab_hit(app.areas.tab_bar, col, row, Tab::ALL.len()) {
                    app.tab = Tab::ALL[idx];
                    return;
                }
                if let Some(clicked) = list_hit(app.areas.list, col, row) {
                    let (state, len) = match app.tab {
                        Tab::Friends => (&mut app.friend_state, app.friends.len()),
                        Tab::Groups => (&mut app.group_state, app.groups.len()),
                        Tab::Expenses => (&mut app.expense_state, app.expenses.len()),
                    };
                    let idx = clicked + state.offset();
                    if idx < len {
                        if app.tab == Tab::Groups && state.selected() == Some(idx) {
                            app.enter_group(client);
                            return;
                        }
                        state.select(Some(idx));
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            if let Some(gv) = &mut app.group_view { gv.move_up(); } else { app.move_up(); }
        }
        MouseEventKind::ScrollDown => {
            if let Some(gv) = &mut app.group_view { gv.move_down(); } else { app.move_down(); }
        }
        _ => {}
    }
}

fn tab_hit(area: Rect, col: u16, row: u16, count: usize) -> Option<usize> {
    if area.width == 0 || count == 0 { return None; }
    if row < area.y + 1 || row >= area.y + area.height.saturating_sub(1) { return None; }
    if col <= area.x || col >= area.x + area.width.saturating_sub(1) { return None; }
    let inner_w = area.width.saturating_sub(2) as usize;
    let x_in = (col - area.x - 1) as usize;
    let idx = x_in * count / inner_w;
    if idx < count { Some(idx) } else { None }
}

fn list_hit(area: Rect, col: u16, row: u16) -> Option<usize> {
    if col <= area.x || col >= area.x + area.width.saturating_sub(1) { return None; }
    if row <= area.y || row >= area.y + area.height.saturating_sub(1) { return None; }
    Some((row - area.y - 1) as usize)
}

// ── UI ──

fn ui(frame: &mut Frame, app: &mut App) {
    if app.group_view.is_some() {
        render_group_view(frame, app);
        return;
    }

    let [tab_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    app.areas.tab_bar = tab_area;

    // ── Tabs ──
    let titles: Vec<&str> = Tab::ALL.iter().map(|t| t.title()).collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(GREEN))
                .title(Line::from(vec![
                    Span::styled(" \u{25c8} ", Style::default().fg(GREEN)),
                    Span::styled(
                        "splitwise ",
                        Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                    ),
                ])),
        )
        .style(Style::default().fg(FG2))
        .highlight_style(
            Style::default()
                .fg(GREEN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled(" \u{2502} ", Style::default().fg(FG3)))
        .select(app.tab.index());
    frame.render_widget(tabs, tab_area);

    // ── Main ──
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
            .areas(main_area);
    app.areas.list = list_area;

    match app.tab {
        Tab::Friends => render_friends(frame, app, list_area, detail_area),
        Tab::Groups => render_groups(frame, app, list_area, detail_area),
        Tab::Expenses => render_expenses(frame, app, list_area, detail_area),
    }

    let help = status_bar(&[
        ("\u{2190}\u{2191}\u{2193}\u{2192}", "nav"),
        ("\u{23ce}", "open"),
        ("Tab", "switch"),
        ("r", "refresh"),
        ("q", "quit"),
    ]);
    frame.render_widget(help, status_area);
}

// ── Group drill-down ──

fn render_group_view(frame: &mut Frame, app: &mut App) {
    let [header_area, tab_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    app.areas.gv_sub_tabs = tab_area;
    let gv = app.group_view.as_mut().unwrap();

    // ── Header ──
    let gtype = gv.group.group_type.as_deref().unwrap_or("");
    let header_sub = if gtype.is_empty() {
        format!("ID: {}", gv.group.id)
    } else {
        format!("ID: {} \u{2022} {gtype}", gv.group.id)
    };
    let header = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(" \u{25c6} ", Style::default().fg(GREEN)),
            Span::styled(
                gv.group.name.clone(),
                Style::default().fg(FG).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("   "),
            Span::styled(header_sub, Style::default().fg(FG2)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(GREEN)),
    );
    frame.render_widget(header, header_area);

    // ── Sub-tabs ──
    let titles: Vec<&str> = GroupTab::ALL.iter().map(|t| t.title()).collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(BORDER)),
        )
        .style(Style::default().fg(FG2))
        .highlight_style(
            Style::default()
                .fg(CYAN)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .divider(Span::styled(" \u{2502} ", Style::default().fg(FG3)))
        .select(gv.tab.index());
    frame.render_widget(tabs, tab_area);

    // ── Content ──
    let [list_area, detail_area] =
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .areas(main_area);
    app.areas.gv_list = list_area;
    let gv = app.group_view.as_mut().unwrap();

    match gv.tab {
        GroupTab::Members => render_gv_members(frame, gv, list_area, detail_area),
        GroupTab::Expenses => render_gv_expenses(frame, gv, list_area, detail_area),
        GroupTab::Debts => render_gv_debts(frame, gv, list_area, detail_area),
    }

    let help = status_bar(&[
        ("\u{2190}\u{2191}\u{2193}\u{2192}", "nav"),
        ("Tab", "switch"),
        ("r", "refresh"),
        ("Esc", "back"),
        ("q", "quit"),
    ]);
    frame.render_widget(help, status_area);
}

// ── GV: Members ──

fn render_gv_members(frame: &mut Frame, gv: &mut GroupView, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = gv.group.members.iter().map(|m| {
        let name = display_name(&m.first_name, m.last_name.as_deref());
        let mut spans = vec![Span::styled(format!("{:<20} ", trunc(&name, 20)), Style::default().fg(FG))];
        spans.extend(balance_text(&m.balance));
        ListItem::new(Line::from(spans))
    }).collect();

    let title = format!("Members ({})", gv.group.members.len());
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);
    frame.render_stateful_widget(list, list_area, &mut gv.member_state);

    let detail = gv.member_state.selected()
        .and_then(|i| gv.group.members.get(i))
        .map(member_detail)
        .unwrap_or_else(|| vec![Line::styled("No member selected", Style::default().fg(FG3))]);
    let para = Paragraph::new(detail).block(detail_block("Member")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn member_detail(m: &GroupMember) -> Vec<Line<'static>> {
    let name = display_name(&m.first_name, m.last_name.as_deref());
    let mut lines = vec![
        Line::styled(name, Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        label_val("ID", m.id.to_string()),
    ];
    if let Some(email) = &m.email {
        lines.push(label_val("Email", email.clone()));
    }
    lines.push(Line::raw(""));

    let nonzero: Vec<&Balance> = m.balance.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        lines.push(Line::styled(
            "\u{2714} Settled up in this group",
            Style::default().fg(GREEN_DIM),
        ));
    } else {
        lines.push(section_hdr("Balance in group"));
        for b in &nonzero {
            lines.extend(balance_line(b));
        }
    }
    lines
}

// ── GV: Expenses ──

fn render_gv_expenses(frame: &mut Frame, gv: &mut GroupView, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = gv.expenses.iter().map(|e| expense_list_item(e)).collect();
    let title = format!("Expenses ({})", gv.expenses.len());
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);
    frame.render_stateful_widget(list, list_area, &mut gv.expense_state);

    let detail = gv.expense_state.selected()
        .and_then(|i| gv.expenses.get(i))
        .map(expense_detail)
        .unwrap_or_else(|| vec![Line::styled("No expense selected", Style::default().fg(FG3))]);
    let para = Paragraph::new(detail).block(detail_block("Expense")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

// ── GV: Debts ──

fn render_gv_debts(frame: &mut Frame, gv: &mut GroupView, list_area: Rect, detail_area: Rect) {
    let names = gv.member_names();
    let debts = gv.debts();
    let is_simplified = !gv.group.simplified_debts.is_empty();

    let items: Vec<ListItem> = debts.iter().map(|d| {
        let from = names.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
        let to = names.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
        ListItem::new(Line::from(vec![
            Span::styled(from.to_string(), Style::default().fg(FG)),
            Span::styled(" \u{2192} ", Style::default().fg(FG3)),
            Span::styled(to.to_string(), Style::default().fg(FG)),
            Span::raw("  "),
            Span::styled(format!("{} {}", d.amount, d.currency_code), Style::default().fg(GOLD)),
        ]))
    }).collect();

    let title = if is_simplified { format!("Debts \u{2022} simplified ({})", debts.len()) }
        else { format!("Debts ({})", debts.len()) };
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);

    let mut debt_list_state = ListState::default();
    if !debts.is_empty() { debt_list_state.select(Some(gv.debt_idx)); }
    frame.render_stateful_widget(list, list_area, &mut debt_list_state);

    let detail = if let Some(d) = debts.get(gv.debt_idx) {
        let from = names.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
        let to = names.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
        vec![
            Line::from(vec![
                Span::styled(from.to_string(), Style::default().fg(FG).add_modifier(Modifier::BOLD)),
                Span::styled(" \u{2192} ", Style::default().fg(FG3)),
                Span::styled(to.to_string(), Style::default().fg(FG).add_modifier(Modifier::BOLD)),
            ]),
            Line::raw(""),
            Line::from(vec![
                Span::styled("Amount  ", Style::default().fg(FG2)),
                Span::styled(
                    format!("{} {}", d.amount, d.currency_code),
                    Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::raw(""),
            label_val("From", format!("{from} ({})", d.from)),
            label_val("To", format!("{to} ({})", d.to)),
        ]
    } else {
        vec![Line::styled("\u{2714} No debts in this group", Style::default().fg(GREEN_DIM))]
    };
    let para = Paragraph::new(detail).block(detail_block("Debt")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

// ── Helpers ──

fn bal_style(amount: &str) -> (Style, &'static str) {
    let v: f64 = amount.parse().unwrap_or(0.0);
    if v > 0.0 {
        (Style::default().fg(GREEN), "\u{25b2}")
    } else if v < 0.0 {
        (Style::default().fg(RED), "\u{25bc}")
    } else {
        (Style::default().fg(FG3), "\u{2022}")
    }
}

fn balance_text(balances: &[Balance]) -> Vec<Span<'_>> {
    let nonzero: Vec<&Balance> = balances.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        return vec![Span::styled("\u{2714} settled", Style::default().fg(FG3))];
    }
    let mut spans = Vec::new();
    for (i, b) in nonzero.iter().enumerate() {
        if i > 0 { spans.push(Span::styled("  ", Style::default())); }
        let (style, arrow) = bal_style(&b.amount);
        spans.push(Span::styled(format!("{arrow} {} {}", b.amount, b.currency_code), style));
    }
    spans
}

fn balance_line(b: &Balance) -> Vec<Line<'static>> {
    let amt: f64 = b.amount.parse().unwrap_or(0.0);
    let (text, style, arrow) = if amt > 0.0 {
        (format!("owes you {} {}", b.amount, b.currency_code), Style::default().fg(GREEN), "\u{25b2}")
    } else {
        (
            format!("you owe {} {}", b.amount.trim_start_matches('-'), b.currency_code),
            Style::default().fg(RED),
            "\u{25bc}",
        )
    };
    let abs = amt.abs();
    let bar_w = ((abs / 200.0) * 20.0).min(20.0).max(1.0) as usize;
    let bar: String = "\u{2588}".repeat(bar_w);
    vec![
        Line::from(vec![
            Span::styled(format!("  {arrow} "), style),
            Span::styled(text, style),
        ]),
        Line::from(vec![
            Span::raw("    "),
            Span::styled(bar, style),
        ]),
    ]
}

fn trunc(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}\u{2026}")
    }
}

fn date_short(s: &str) -> &str {
    &s[..s.len().min(10)]
}

fn expense_list_item(e: &Expense) -> ListItem<'static> {
    let date = e.date.as_deref().map(date_short).unwrap_or("---").to_string();
    let desc = trunc(&e.description, 18);
    let del = if e.deleted_at.is_some() { " \u{2716}" } else { "" };
    ListItem::new(Line::from(vec![
        Span::styled(format!("{date} "), Style::default().fg(FG3)),
        Span::styled(format!("{desc:<18} "), Style::default().fg(FG)),
        Span::styled(format!("{}{del}", e.cost), Style::default().fg(CYAN)),
    ]))
}

// ── Friends ──

fn render_friends(frame: &mut Frame, app: &mut App, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = app.friends.iter().map(|f| {
        let name = display_name(&f.first_name, f.last_name.as_deref());
        let mut spans = vec![Span::styled(format!("{:<20} ", trunc(&name, 20)), Style::default().fg(FG))];
        spans.extend(balance_text(&f.balance));
        ListItem::new(Line::from(spans))
    }).collect();

    let title = format!("Friends ({})", app.friends.len());
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);
    frame.render_stateful_widget(list, list_area, &mut app.friend_state);

    let detail = app.friend_state.selected()
        .and_then(|i| app.friends.get(i))
        .map(friend_detail)
        .unwrap_or_else(|| vec![Line::styled("No friend selected", Style::default().fg(FG3))]);
    let para = Paragraph::new(detail).block(detail_block("Details")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn friend_detail(f: &Friend) -> Vec<Line<'_>> {
    let name = display_name(&f.first_name, f.last_name.as_deref());
    let mut lines = vec![
        Line::styled(name, Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        label_val("ID", f.id.to_string()),
    ];
    if let Some(email) = &f.email {
        lines.push(label_val("Email", email.clone()));
    }
    lines.push(Line::raw(""));

    let nonzero: Vec<&Balance> = f.balance.iter().filter(|b| b.amount != "0.00").collect();
    if nonzero.is_empty() {
        lines.push(Line::styled(
            "\u{2714} All settled up",
            Style::default().fg(GREEN_DIM),
        ));
    } else {
        lines.push(section_hdr("Balance"));
        for b in &nonzero {
            lines.extend(balance_line(b));
        }
    }

    if !f.groups.is_empty() {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled("Shared groups  ", Style::default().fg(FG2)),
            Span::styled(f.groups.len().to_string(), Style::default().fg(FG)),
        ]));
    }
    lines
}

// ── Groups ──

fn render_groups(frame: &mut Frame, app: &mut App, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = app.groups.iter().map(|g| {
        let name = trunc(&g.name, 22);
        let members = g.members.len();
        ListItem::new(Line::from(vec![
            Span::styled(format!("{name:<22} "), Style::default().fg(FG)),
            Span::styled(format!("{members}"), Style::default().fg(CYAN)),
            Span::styled(" members", Style::default().fg(FG3)),
        ]))
    }).collect();

    let title = format!("Groups ({})", app.groups.len());
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);
    frame.render_stateful_widget(list, list_area, &mut app.group_state);

    let detail = app.group_state.selected()
        .and_then(|i| app.groups.get(i))
        .map(group_detail)
        .unwrap_or_else(|| vec![Line::styled("No group selected", Style::default().fg(FG3))]);
    let para = Paragraph::new(detail).block(detail_block("Details")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn group_detail(g: &Group) -> Vec<Line<'_>> {
    let mut lines = vec![
        Line::styled(g.name.clone(), Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        label_val("ID", g.id.to_string()),
    ];
    if let Some(t) = &g.group_type {
        lines.push(label_val("Type", t.clone()));
    }
    if let Some(s) = g.simplify_by_default {
        lines.push(label_val("Simplify", s.to_string()));
    }

    if !g.members.is_empty() {
        lines.push(Line::raw(""));
        lines.push(section_hdr(&format!("Members ({})", g.members.len())));
        for m in &g.members {
            let name = display_name(&m.first_name, m.last_name.as_deref());
            let bals: Vec<String> = m.balance.iter()
                .filter(|b| b.amount != "0.00")
                .map(|b| format!("{} {}", b.amount, b.currency_code))
                .collect();
            if bals.is_empty() {
                lines.push(Line::from(vec![
                    Span::styled("  \u{2022} ", Style::default().fg(FG3)),
                    Span::styled(name, Style::default().fg(FG)),
                ]));
            } else {
                let (style, _) = bal_style(&m.balance.first().map(|b| b.amount.as_str()).unwrap_or("0"));
                lines.push(Line::from(vec![
                    Span::styled("  \u{2022} ", Style::default().fg(FG3)),
                    Span::styled(format!("{name}  "), Style::default().fg(FG)),
                    Span::styled(bals.join(", "), style),
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
        lines.push(section_hdr(label));
        let members: std::collections::HashMap<u64, String> = g.members.iter()
            .map(|m| (m.id, display_name(&m.first_name, m.last_name.as_deref())))
            .collect();
        for d in debts {
            let from = members.get(&d.from).map(|s| s.as_str()).unwrap_or("?");
            let to = members.get(&d.to).map(|s| s.as_str()).unwrap_or("?");
            lines.push(Line::from(vec![
                Span::styled(format!("  {from} "), Style::default().fg(FG)),
                Span::styled("\u{2192}", Style::default().fg(FG3)),
                Span::styled(format!(" {to}  ", ), Style::default().fg(FG)),
                Span::styled(format!("{} {}", d.amount, d.currency_code), Style::default().fg(GOLD)),
            ]));
        }
    }
    lines
}

// ── Expenses ──

fn render_expenses(frame: &mut Frame, app: &mut App, list_area: Rect, detail_area: Rect) {
    let items: Vec<ListItem> = app.expenses.iter().map(|e| expense_list_item(e)).collect();
    let title = format!("Expenses ({})", app.expenses.len());
    let list = List::new(items)
        .block(block_focus(&title))
        .highlight_style(HIGHLIGHT)
        .highlight_symbol(SEL);
    frame.render_stateful_widget(list, list_area, &mut app.expense_state);

    let detail = app.expense_state.selected()
        .and_then(|i| app.expenses.get(i))
        .map(expense_detail)
        .unwrap_or_else(|| vec![Line::styled("No expense selected", Style::default().fg(FG3))]);
    let para = Paragraph::new(detail).block(detail_block("Details")).wrap(Wrap { trim: false });
    frame.render_widget(para, detail_area);
}

fn expense_detail(e: &Expense) -> Vec<Line<'_>> {
    let mut lines = vec![
        Line::styled(e.description.clone(), Style::default().fg(FG).add_modifier(Modifier::BOLD)),
        label_val("ID", e.id.to_string()),
        Line::from(vec![
            Span::styled("Cost: ", Style::default().fg(FG2)),
            Span::styled(
                format!("{} ", e.cost),
                Style::default().fg(GOLD).add_modifier(Modifier::BOLD),
            ),
            Span::styled(e.currency_code.clone(), Style::default().fg(FG2)),
        ]),
    ];
    if let Some(date) = &e.date {
        lines.push(label_val("Date", date_short(date).to_string()));
    }
    if let Some(cat) = &e.category {
        lines.push(label_val("Category", cat.name.clone()));
    }
    if let Some(creator) = &e.created_by {
        lines.push(label_val(
            "Created by",
            display_name(&creator.first_name, creator.last_name.as_deref()),
        ));
    }
    if e.deleted_at.is_some() {
        lines.push(Line::styled(
            "\u{2716} DELETED",
            Style::default().fg(RED).add_modifier(Modifier::BOLD),
        ));
    }

    if !e.users.is_empty() {
        lines.push(Line::raw(""));
        lines.push(section_hdr("Shares"));
        lines.push(Line::styled(
            format!("  {:<16} {:>8} {:>8} {:>9}", "Name", "Paid", "Owed", "Net"),
            Style::default().fg(FG3),
        ));
        lines.push(Line::styled(
            format!("  {}", "\u{2500}".repeat(46)),
            Style::default().fg(FG3),
        ));
        for s in &e.users {
            let name = display_name(s.first_name.as_deref().unwrap_or("?"), s.last_name.as_deref());
            let (net_style, _) = bal_style(&s.net_balance);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:<16} ", trunc(&name, 16)),
                    Style::default().fg(FG),
                ),
                Span::styled(format!("{:>8} ", s.paid_share), Style::default().fg(CYAN)),
                Span::styled(format!("{:>8} ", s.owed_share), Style::default().fg(FG2)),
                Span::styled(format!("{:>9}", s.net_balance), net_style),
            ]));
        }
    }

    if !e.repayments.is_empty() {
        lines.push(Line::raw(""));
        lines.push(section_hdr("Repayments"));
        for r in &e.repayments {
            lines.push(Line::from(vec![
                Span::styled(format!("  {} ", r.from), Style::default().fg(FG)),
                Span::styled("\u{2192}", Style::default().fg(FG3)),
                Span::styled(format!(" {}  ", r.to), Style::default().fg(FG)),
                Span::styled(r.amount.clone(), Style::default().fg(GOLD)),
            ]));
        }
    }
    lines
}
