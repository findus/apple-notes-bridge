use crate::{Outcome, Event, Task, refetch_notes};
use apple_notes_manager::db::DatabaseService;
use apple_notes_manager::notes::localnote::LocalNote;
use tui::widgets::{Wrap, Borders, Block, Paragraph, ListState, ListItem, List};
use tui::style::{Style, Color, Modifier};
use tui::layout::{Constraint, Direction, Layout, Alignment};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::{thread, io};
use tui::Terminal;
use tui::backend::CrosstermBackend;
use apple_notes_manager::AppleNotes;
use std::sync::mpsc::{Sender, Receiver};
use apple_notes_manager::notes::traits::identifyable_note::IdentifyableNote;
use crossterm::{
    event::{self, Event as CEvent, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use crossterm::event::KeyEvent;

pub struct UiState {
    pub(crate) action_sender: Sender<Task>,
    pub(crate) event_receiver: Receiver<Event<KeyEvent>>,
    pub(crate) event_sender: Arc<Mutex<Sender<Event<KeyEvent>>>>
}

pub struct Ui<'u> {
    pub note_list_state: ListState,
    pub end: bool,
    pub color: Color,
    pub status: String,
    pub app: Arc<Mutex<AppleNotes>>,
    pub ui_state: UiState,
    pub entries: Vec<LocalNote>,
    pub keyword: Option<String>,
    pub items: Vec<ListItem<'u>>,
    pub list: List<'u>,
    pub text: String,
    pub scroll_amount: u16,
    pub in_search_mode: bool,
}

impl<'u> Ui<'u> {

    fn gen_list(&self) -> List<'u> {

        let title = match self.keyword.clone() {
            None => {
                format!("List")
            }
            Some(word) => {
                format!("List Filter:[{}]", word)

            }
        };

        List::new(self.items.clone())
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
            .highlight_symbol(">>")
    }

    fn generate_list_items(&mut self) -> Vec<ListItem<'u>> {
        self.entries.iter()
            .filter(|entry| {
                if self.keyword.is_some() {
                    entry.body[0].text.as_ref().unwrap().to_lowercase().contains(&self.keyword.as_ref().unwrap().to_lowercase())
                } else {
                    return true
                }
            })
            .map(|e| {
                if e.needs_merge() {
                    ListItem::new(format!("[M] {} {}", e.metadata.folder(), e.first_subject()).to_string()).style(Style::default().fg(Color::LightBlue))
                } else if e.content_changed_locally() {
                    ListItem::new(format!("{} {}", e.metadata.folder(), e.first_subject()).to_string()).style(Style::default().fg(Color::LightYellow))
                } else if e.metadata.locally_deleted {
                    ListItem::new(format!("{} {}", e.metadata.folder(), e.first_subject()).to_string()).style(Style::default().fg(Color::LightRed))
                } else if e.metadata.new {
                    ListItem::new(format!("{} {}", e.metadata.folder(), e.first_subject()).to_string()).style(Style::default().fg(Color::LightGreen))
                } else {
                    ListItem::new(format!("{} {}", e.metadata.folder(), e.first_subject()).to_string())
                }
            }).collect()
    }

    fn refresh(&mut self) {
        self.entries = refetch_notes(&self.app.lock().unwrap(), &self.keyword);
        self.items = self.generate_list_items( );
        self.list = self.gen_list();
    }

    fn reload_text(&mut self) {
        // self.note_list_state.select(Some(0));

        match self.note_list_state.selected() {
            Some(index) if matches!(self.entries.get(index), Some(_)) => {
                let entry = self.entries.get(index).unwrap();
                self.text = entry.body[0].text.as_ref().unwrap().clone();
            }
            _ => {
                self.text = "".to_string();
            }
        }
    }

    fn set_status<'a>(&self, text: &'a str, color: Color) -> Paragraph<'a> {
        Paragraph::new(text)
            .block(Block::default().title("Status").borders(Borders::ALL))
            .style(Style::default().fg(color))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {

        enable_raw_mode().expect("can run in raw mode");

        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        terminal.clear().unwrap();

        //let ui_state = self.ui_state.lock().unwrap();

        // Insert Thread for input detection

        let sender = Arc::clone(&self.ui_state.event_sender);

        thread::spawn(move || {
            let tick_rate = Duration::from_millis(1000);
            let mut last_tick = Instant::now();
            loop {

                let sender = sender.lock().unwrap();

                let timeout = tick_rate
                    .checked_sub(last_tick.elapsed())
                    .unwrap_or_else(|| Duration::from_secs(0));

                if event::poll(timeout).expect("poll works") {
                    if let CEvent::Key(key) = event::read().expect("can read events") {
                        sender.send(Event::Input(key)).expect("can send events");
                    }
                }

                if last_tick.elapsed() >= tick_rate {
                    if let Ok(_) = sender.send(Event::Tick) {
                        last_tick = Instant::now();
                    }
                }
            }
        });

        self.note_list_state = ListState::default();
        self.note_list_state.select(Some(0));
        self.end = false;

        self.refresh();

        self.reload_text();
        self.scroll_amount = 0;

        self.status = "Syncing".to_string();
        self.color = Color::Yellow;

        self.ui_state.action_sender.send(Task::Sync);

        loop {

            let a = Arc::clone(&self.app);

            terminal.draw(|f| {

                let value = &self.status;
                let t2 = self.set_status(value, self.color);

                let lay = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Percentage(95),
                            Constraint::Percentage(5),
                        ].as_ref()
                    );

                let chunks = lay.split(f.size());

                let noteslayout = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(0)
                    .constraints(
                        [
                            Constraint::Percentage(20),
                            Constraint::Percentage(80),
                        ].as_ref()
                    ).split(chunks[0]);

                f.render_stateful_widget(
                    self.list.clone(),
                    noteslayout[0],
                    &mut self.note_list_state.clone()
                );

                let t  = Paragraph::new(self.text.clone())
                    .block(Block::default().title("Content").borders(Borders::ALL))
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Left)
                    .scroll((self.scroll_amount,self.scroll_amount))
                    .wrap(Wrap { trim: false });


                f.render_widget(t, noteslayout[1]);
                f.render_widget(t2.clone(), chunks[1]);
            }).unwrap();

            let received_keystroke = self.ui_state.event_receiver.recv()?;

            if self.in_search_mode {
                match received_keystroke {
                    Event::Input(event) => match event.code {
                        KeyCode::Esc => {
                            self.status = "".to_string();
                            self.color = Color::White;
                            self.in_search_mode = false;
                            self.refresh();
                            self.reload_text()
                        }
                        KeyCode::Backspace => {
                            if self.keyword.is_some() {
                                let len = self.keyword.as_ref().unwrap().len();
                                if len > 0 {
                                    let mut d = self.keyword.as_ref().unwrap().clone();
                                    d.pop();
                                    self.keyword = Some(d);
                                    self.status = self.keyword.as_ref().unwrap().clone();
                                }

                                self.refresh();
                                self.note_list_state.select(Some(0));
                            }
                        }
                        KeyCode::Char(c) => {
                            let ed = c;
                            self.keyword = Some(format!("{}{}", self.keyword.as_ref().unwrap(), ed));
                            self.status = self.keyword.as_ref().unwrap().clone();
                            self.refresh();
                        }
                        _ => {}
                    }
                    _ => {}
                }
            } else {
                match received_keystroke {
                    Event::Input(event) => match event.code {
                        KeyCode::Char('j') => {
                            let selected = self.note_list_state.selected();
                            if self.entries.len() > 0 && selected.unwrap_or(0) < self.entries.len() -1 {
                                self.note_list_state.select(Some(selected.unwrap_or(0) + 1));
                                self.reload_text();
                                self.scroll_amount = 0;
                            }
                        },
                        KeyCode::Char('k') => {
                            let selected = self.note_list_state.selected();
                            if selected.unwrap_or(0) > 0 {
                                self.note_list_state.select(Some(selected.unwrap_or(0) - 1));
                                self.reload_text();
                                self.scroll_amount = 0;
                            }
                        },
                        KeyCode::Char('J') => {
                            self.scroll_amount += 4;
                        },
                        KeyCode::Char('K') => {
                            if self.scroll_amount >= 4 {
                                self.scroll_amount -= 4;
                            } else {
                                self.scroll_amount = 0;
                            }
                        },
                        KeyCode::Char('m') => {
                            let note = self.entries.get(self.note_list_state.selected().unwrap()).unwrap();
                            match a.lock().unwrap().merge(&note.metadata.uuid) {
                                Ok(_) => {
                                    let old_uuid = self.entries.get(self.note_list_state.selected().unwrap()).unwrap().metadata.uuid.clone();
                                    self.refresh();

                                    let old_note_idx = self.entries.iter().enumerate().filter(|(_idx,note)| {
                                        note.metadata.uuid == old_uuid
                                    }).last().unwrap().0;

                                    self.note_list_state.select(Some(old_note_idx));
                                    self.reload_text();
                                }
                                Err(e) => {
                                    self.color = Color::Red;
                                    self.status = e.to_string();
                                }
                            }
                        }
                        KeyCode::Char('e') => {
                            let note = self.entries.get(self.note_list_state.selected().unwrap()).unwrap();
                            let app = a.lock().unwrap();
                            let result: Result<LocalNote,Box<dyn std::error::Error>> =
                                app.edit_note(&note, false)
                                    .map_err(|e| e.into())
                                    .and_then(|note| app.update_note(&note).map(|_n| note).map_err(|e| e.into()));

                            match result {
                                Ok(_note) => {
                                    let old_uuid = self.entries.get(self.note_list_state.selected().unwrap()).unwrap().metadata.uuid.clone();

                                    self.refresh();

                                    let old_note_idx = self.entries.iter().enumerate().filter(|(_idx,note)| {
                                        note.metadata.uuid == old_uuid
                                    }).last().unwrap().0;

                                    self.note_list_state.select(Some(old_note_idx));
                                    self.reload_text();

                                }
                                Err(e) => {
                                    self.color = Color::Red;
                                    self.status = e.to_string();
                                }
                            }

                        },
                        KeyCode::Char('d') => {
                            let mut note = self.entries.get(self.note_list_state.selected().unwrap()).unwrap().clone();
                            note.metadata.locally_deleted = !note.metadata.locally_deleted ;

                            let db_connection = apple_notes_manager::db::SqliteDBConnection::new();
                            db_connection.update(&note).unwrap();

                            self.refresh();

                        },
                        KeyCode::Char('s') => {
                            self.status = "Syncing".to_string();
                            self.color = Color::Yellow;

                            self.ui_state.action_sender.send(Task::Sync).unwrap();

                        },
                        KeyCode::Char('x') => {
                            self.status = "Syncing".to_string();
                            self.color = Color::Yellow;

                            self.ui_state.action_sender.send(Task::Test).unwrap();
                        },
                        KeyCode::Char('q') => {
                            self.end = true;

                            self.ui_state.action_sender.send(Task::End).unwrap();
                        },
                        KeyCode::Char('/') => {
                            self.keyword = Some("".to_string());
                            self.status = format!("Search mode: {}", self.keyword.as_ref().unwrap());
                            self.color = Color::Cyan;
                            self.in_search_mode = true;
                        },
                        KeyCode::Char('c') => {
                            self.status = format!("Filter Cleared");
                            self.color = Color::White;

                            self.keyword = None;

                            let mut old_uuid = None;

                            if let Some(old_selected_entry) = self.entries.get(self.note_list_state.selected().unwrap_or(0)) {
                                old_uuid = Some(old_selected_entry.metadata.uuid.clone());
                            }

                            self.refresh();

                            if let Some(uuid) = old_uuid {
                                let old_note_idx = self.entries.iter().enumerate().filter(|(_idx,note)| {
                                    note.metadata.uuid == uuid
                                }).last().unwrap().0;

                                self.note_list_state.select(Some(old_note_idx));
                            }

                        },
                        KeyCode::Esc => {
                            self.status = "".to_string();
                            self.in_search_mode = false;
                        }
                        _ => {}
                    }
                    Event::Tick => {}
                    Event::OutCome(outcome) => match outcome {
                        Outcome::Busy() => {
                            self.color = Color::Red;
                            self.status = "Currently Busy".to_string();
                        }
                        Outcome::Success(s) => {
                            let mut old_uuid = None;

                            if let Some(old_selected_entry) = self.entries.get(self.note_list_state.selected().unwrap_or(0)) {
                                old_uuid = Some(old_selected_entry.metadata.uuid.clone());
                            }

                            self.color = Color::Green;
                            self.status = s;

                            self.refresh();

                            let mut index = self.note_list_state.selected().unwrap_or(0);

                            //TODO old_uuid if present selection
                            if index > self.items.len() - 1 {
                                index = self.items.len() - 1;
                                self.note_list_state.select(Some(index));
                            }

                            if let Some(uuid) = old_uuid {
                                let old_note_idx = self.entries.iter().enumerate().filter(|(_idx,note)| {
                                    note.metadata.uuid == uuid
                                }).last().unwrap().0;

                                self.note_list_state.select(Some(old_note_idx));
                            }

                            self.text = self.entries.get(index).unwrap().body[0].text.as_ref().unwrap().clone();
                        }
                        Outcome::Failure(s) => {
                            self.color = Color::Red;
                            self.status = s;
                            self.refresh();
                        }
                        Outcome::End() => {
                            break;
                        }
                    }
                }
            }


        }

        terminal.clear().unwrap();
        disable_raw_mode().unwrap();

        Ok(())
    }

}