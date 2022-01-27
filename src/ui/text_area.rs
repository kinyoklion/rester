use crate::ui::cursor::Cursor;
use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Style;
use tui::widgets::{Block, Paragraph, StatefulWidget, Widget};

pub struct EditState {
    buffer: String,
    pos: usize,
}

pub enum EditCommand {
    InsertCharacter(char),
    BackwardDelete,
    ForwardDelete,
    ForwardCursor,
    BackwardCursor,
    UpCursor,
    DownCursor,
}

pub struct Row {
    start: usize,
    end: usize,
    size: usize,
}

#[derive(Default, Clone)]
pub struct TextArea<'a> {
    /// A block to wrap the widget in
    block: Option<Block<'a>>,
    /// Widget style
    style: Style,
    /// Flag indicating if this component should render as active.
    active: bool,
}

impl<'a> TextArea<'a> {
    pub fn block(mut self, block: Block<'a>) -> TextArea<'a> {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> TextArea<'a> {
        self.style = style;
        self
    }

    pub fn active(mut self, active: bool) -> TextArea<'a> {
        self.active = active;
        self
    }
}

impl<'a> StatefulWidget for TextArea<'a> {
    type State = EditState;

    fn render(mut self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        buf.set_style(area, self.style);

        let text_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        // Don't bother rendering if there isn't enough size.
        if text_area.height < 1 || text_area.width < 1 {
            return;
        }

        let (row, before, pos_in_row) = count_newlines(state.buffer.as_str(), state.pos);
        let mut y_scroll = 0;
        let mut x_scroll = 0;

        if row > (text_area.height as usize - 1) {
            y_scroll = ((text_area.height - 1) as i32 - row as i32).abs() as u16
        }

        if pos_in_row > (text_area.width as usize - 1) {
            x_scroll = ((text_area.width - 1) as i32 - pos_in_row as i32).abs() as u16
        }

        let paragraph = Paragraph::new(state.buffer.as_str())
            // .block(block)
            .scroll((y_scroll, x_scroll));

        paragraph.render(text_area, buf);
        if self.active {
            let cursor = Cursor::default()
                .position((state.pos - before) as u16, row as u16)
                .scroll(y_scroll, x_scroll);

            cursor.render(text_area, buf);
        }
    }
}

pub fn row_topology(s: &str, pos: usize) -> (Vec<Row>, usize) {
    let mut topology = Vec::new();
    let mut bytes_line = 0;
    let mut current_row = 0;
    let mut row_found = false;

    let mut row_start = 0;
    for (index, item) in s.as_bytes().iter().enumerate() {
        if index >= pos as usize && !row_found {
            row_found = true;
            current_row = topology.len()
        }

        bytes_line += 1;
        if *item == b'\n' {
            topology.push(Row {
                start: row_start,
                end: bytes_line + row_start - 1,
                size: bytes_line,
            });
            row_start = bytes_line + row_start;
            bytes_line = 0;
        }
    }
    topology.push(Row {
        start: row_start,
        end: bytes_line + row_start,
        size: bytes_line,
    });
    if pos >= s.len() && topology.len() > 0 {
        current_row = topology.len() - 1
    }
    (topology, current_row)
}

pub fn count_newlines(s: &str, pos: usize) -> (usize, usize, usize) {
    let mut count = 0;
    let mut bytes_before = 0;
    let mut bytes_line = 0;
    for (index, item) in s.as_bytes().iter().enumerate() {
        if index >= pos as usize {
            break;
        }

        bytes_line += 1;
        if *item == b'\n' {
            count += 1;
            bytes_before += bytes_line;
            bytes_line = 0;
        }
    }
    (count, bytes_before, bytes_line)
}

impl EditState {
    pub fn new(init_value: &str) -> Self {
        EditState {
            buffer: init_value.to_string(),
            pos: 0,
        }
    }

    pub fn handle_command(&mut self, command: EditCommand) {
        match command {
            EditCommand::InsertCharacter(c) => {
                self.buffer.insert(self.pos, c);
                self.pos += 1;
            }
            EditCommand::BackwardDelete => {
                if self.pos > 0 {
                    self.buffer.remove(self.pos - 1);
                    self.pos -= 1;
                }
            }
            EditCommand::ForwardDelete => {
                if self.pos < self.buffer.len() {
                    self.buffer.remove(self.pos);
                }
            }
            EditCommand::ForwardCursor => {
                if self.buffer.len() > 0 && self.pos < self.buffer.len() {
                    self.pos += 1
                }
            }
            EditCommand::BackwardCursor => {
                if self.pos > 0 {
                    self.pos -= 1
                }
            }
            EditCommand::UpCursor => {
                let (topology, row) = row_topology(self.buffer.as_str(), self.pos);
                if row == 0 {
                    return;
                }

                let pos_in_row = self.pos - topology[row].start;
                let new_row = row - 1;

                let new_row_topology = &topology[new_row];
                let new_pos = if pos_in_row < new_row_topology.size {
                    new_row_topology.start + pos_in_row
                } else {
                    new_row_topology.end
                };

                self.pos = new_pos;
            }
            EditCommand::DownCursor => {
                let (topology, row) = row_topology(self.buffer.as_str(), self.pos);
                if row == topology.len() - 1 {
                    return;
                }

                let pos_in_row = self.pos - topology[row].start;
                let new_row = row + 1;

                let new_row_topology = &topology[new_row];
                let new_pos = if pos_in_row < new_row_topology.size {
                    new_row_topology.start + pos_in_row
                } else {
                    new_row_topology.end
                };

                self.pos = new_pos;
            }
        };
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn as_str(&self) -> &str {
        self.buffer.as_str()
    }

    pub fn set_value(&mut self, value: String) {
        self.buffer = value;
    }
}
