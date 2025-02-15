use wasm_bindgen::prelude::*;
use wasm_bindgen::JsError;
use js_sys::Array;
#[wasm_bindgen]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Fumen {
    pages: Vec<Page>,
    pub guideline: bool
}
#[wasm_bindgen]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Page {
    pub piece: Option<Piece>,

    pub rise: bool,
    pub mirror: bool,
    pub lock: bool,
    comment: Option<String>,
    /// y-up
    field: [[CellColor; 10]; 23],
    garbage_row: [CellColor; 10]
}
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum PieceType {
    I = 1,
    L = 2,
    O = 3,
    Z = 4,
    T = 5,
    J = 6,
    S = 7
}
impl PieceType { 
    pub fn from_i64(v: i64) -> PieceType {
        match v {
            1 => PieceType::I,
            2 => PieceType::L,
            3 => PieceType::O,
            4 => PieceType::Z,
            5 => PieceType::T,
            6 => PieceType::J,
            7 => PieceType::S,
            _ => unreachable!()
        }
    }
}
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum RotationState {
    South = 0,
    East = 1,
    North = 2,
    West = 3
}


/// Represents a tetromino piece using true rotation.
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Piece {
    pub kind: PieceType,
    pub rotation: RotationState,
    pub x: u32,
    /// y-up
    pub y: u32
}
#[wasm_bindgen]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum CellColor {
    Empty = 0,
    I = 1,
    L = 2,
    O = 3,
    Z = 4,
    T = 5,
    J = 6,
    S = 7,
    Grey = 8
}

const BASE64_CHARS: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J',
    b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T',
    b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd',
    b'e', b'f', b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n',
    b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x',
    b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
    b'8', b'9', b'+', b'/'
];
#[wasm_bindgen]
impl Fumen {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Fumen {
        Fumen {
            pages: vec![],
            guideline: true
        }
    }
    /// Encode as a fumen data string.
    pub fn encode(&self) -> String {
        // we need a vec and not a string here since we need to go back and patch in the
        // length of empty field sequences... and i don't want to do 2-pass encoding
        let mut data = b"v115@".to_vec();
        let mut prev_field = [[CellColor::Empty; 10]; 24];
        let mut empty_field = None;
        let mut first = true;
        for page in &self.pages {
            // encode field
            let deltas = fumen_field_delta(prev_field, (*page).fumen_field());
            if deltas == [[8; 10]; 24] {
                // do special-case unchanged field stuff
                if let Some((ref mut index, ref mut count)) = empty_field {
                    // count empty fields
                    *count += 1;
                    if *count == 63 {
                        data[*index] = BASE64_CHARS[*count];
                        empty_field = None;
                    }
                } else {
                    // new empty field encoding
                    data.push(b'v');
                    data.push(b'h');
                    empty_field = Some((data.len(), 0));
                    data.push(0);
                }
            } else {
                // finalize the empty field sequence
                if let Some((index, count)) = empty_field {
                    data[index] = BASE64_CHARS[count];
                    empty_field = None;
                }
                // do run-length encoding of deltas
                let mut prev = deltas[0][0];
                let mut count = 0;
                for y in 0..24 {
                    for x in 0..10 {
                        if deltas[y][x] == prev {
                            count += 1;
                        } else {
                            let num = prev * 240 + count - 1;
                            data.push(BASE64_CHARS[num & 0x3F]);
                            data.push(BASE64_CHARS[num >> 6 & 0x3F]);
                            prev = deltas[y][x];
                            count = 1;
                        }
                    }
                }
                let num = prev * 240 + count - 1;
                data.push(BASE64_CHARS[num & 0x3F]);
                data.push(BASE64_CHARS[num >> 6 & 0x3F]);
            }

            let page_flags = (*page).fumen_number() as usize + if first {
                first = false;
                self.guideline as usize * 240 * 128
            } else { 0 };
            data.push(BASE64_CHARS[page_flags & 0x3F]);
            data.push(BASE64_CHARS[page_flags >> 6 & 0x3F]);
            data.push(BASE64_CHARS[page_flags >> 12 & 0x3F]);

            if let Some(ref comment) = (*page).comment {
                let mut encoded = js_escape(comment);
                encoded.truncate(4095);
                data.push(BASE64_CHARS[encoded.len() & 0x3F]);
                data.push(BASE64_CHARS[encoded.len() >> 6 & 0x3F]);

                for c in encoded.chunks(4) {
                    let mut v = 0;
                    for &c in c.iter().rev() {
                        v *= 96;
                        v += c as usize - 0x20;
                    }
                    for _ in 0..5 {
                        data.push(BASE64_CHARS[v & 0x3F]);
                        v >>= 6;
                    }
                }
            }

            // this handles piece locking, line clear, mirror, and rise rules
            prev_field = (*page).next_page().fumen_field();
        }

        // finalize the empty field sequence
        if let Some((index, count)) = empty_field {
            data[index] = BASE64_CHARS[count];
        }

        String::from_utf8(data).unwrap()
    }

    /// Decodes a fumen data string.
    /// 
    pub fn decode(data: &str) -> Result<Fumen, DecodeFumenError> {
        unsafe { 
            Fumen::decode_opt(data).ok_or(DecodeFumenError)
        }
        
    }

    unsafe fn decode_opt(data: &str) -> Option<Fumen> {
        if data.chars().take(5).collect::<String>() != "v115@" {
            return None;
        }
        let mut iter = data[5..].chars().filter(|&c| c != '?').map(from_base64).peekable();
        let mut fumen = Fumen::default();
        let mut empty_fields = 0;
        while iter.peek().is_some() {
            let page = fumen.add_page();
            if empty_fields == 0 {
                // decode field spec
                let mut delta = [[0; 10]; 24];
                let mut x = 0;
                let mut y = 0;
                while y != 24 {
                    let number = iter.next()?? + 64 * iter.next()??;
                    let value = number / 240;
                    let repeats = number % 240 + 1;
                    for _ in 0..repeats {
                        if y == 24 {
                            return None;
                        }
                        delta[y][x] = value;
                        x += 1;
                        if x == 10 {
                            y += 1;
                            x = 0;
                        }
                    }
                }
                if delta == [[8; 10]; 24] {
                    empty_fields = iter.next()??;
                }
                for y in 0..23 {
                    for x in 0..10 {
                        let value = delta[y][x] + (*page).field[22-y][x] as usize - 8;
                        (*page).field[22-y][x] = decode_cell_color(value)?;
                    }
                }
                for x in 0..10 {
                    let value = delta[23][x] + (*page).garbage_row[x] as usize - 8;
                    (*page).garbage_row[x] = decode_cell_color(value)?;
                }
            } else {
                empty_fields -= 1;
            }

            // decode page data
            let number = iter.next()?? + iter.next()?? * 64 + iter.next()?? * 64*64;
            let piece_type = number % 8;
            let piece_rot = number / 8 % 4;
            let piece_pos = number / 32 % 240;

            (*page).piece = if piece_type == 0 { None } else {
                let kind = match piece_type {
                    1 => PieceType::I,
                    2 => PieceType::L,
                    3 => PieceType::O,
                    4 => PieceType::Z,
                    5 => PieceType::T,
                    6 => PieceType::J,
                    7 => PieceType::S,
                    _ => unreachable!()
                };
                let rotation = match piece_rot {
                    0 => RotationState::South,
                    1 => RotationState::East,
                    2 => RotationState::North,
                    3 => RotationState::West,
                    _ => unreachable!()
                };
                let x = piece_pos as u32 % 10;
                let y = 22 - piece_pos as u32 / 10;
                Some(Piece {
                    kind, rotation,
                    // we need to convert fumen centers to SRS true rotation centers
                    x: match (kind, rotation) {
                        (PieceType::S, RotationState::East) => x - 1,
                        (PieceType::Z, RotationState::West) => x + 1,
                        (PieceType::O, RotationState::West) => x + 1,
                        (PieceType::O, RotationState::South) => x + 1,
                        (PieceType::I, RotationState::South) => x + 1,
                        _ => x
                    },
                    y: match (kind, rotation) {
                        (PieceType::S, RotationState::North) => y - 1,
                        (PieceType::Z, RotationState::North) => y - 1,
                        (PieceType::O, RotationState::North) => y - 1,
                        (PieceType::O, RotationState::West) => y - 1,
                        (PieceType::I, RotationState::West) => y - 1,
                        _ => y
                    }
                })
            };

            let flags = number / 32 / 240;
            (*page).rise = flags & 0b1 != 0;
            (*page).mirror = flags & 0b10 != 0;
            let guideline = flags & 0b100 != 0;
            let comment = flags & 0b1000 != 0;
            (*page).lock = flags & 0b10000 == 0;

            if comment {
                let mut length = iter.next()?? + iter.next()?? * 64;
                let mut escaped = String::new();
                while length > 0 {
                    let mut number = iter.next()?? + iter.next()?? * 64 + iter.next()?? * 64 * 64
                        + iter.next()?? * 64 * 64 * 64 + iter.next()?? * 64 * 64 * 64 * 64;
                    for _ in 0..length.min(4) {
                        escaped.push(std::char::from_u32(number as u32 % 96 + 0x20)?);
                        length -= 1;
                        number /= 96;
                    }
                }
                (*page).comment = Some(js_unescape(&escaped));
            }

            if fumen.pages.len() == 1 {
                fumen.guideline = guideline;
            }
        }
        Some(fumen)
    }

    /// Create a new page, in the same way as creating a new page in fumen does.
    ///
    /// This will apply the piece locking, line clear, rise, and mirror rules just like fumen does.
    #[wasm_bindgen(js_name = "addPage")]
    pub fn add_page(&mut self) -> *mut Page {
        self.pages.push(match self.pages.last() {
            Some(p) => p.next_page(),
            None => Page::default()
        });
        self.pages.last_mut().unwrap()
    }
    #[wasm_bindgen(getter)]
    pub fn pages(&self) -> Array {
        let array = Array::new();
        for pg in self.pages.iter() {
            array.push(&JsValue::from(pg.clone()));
        }
        array
    }
}
impl Fumen {
    pub fn get_pages(&self) -> &Vec<Page> {
        &self.pages
    }
    pub fn get_pages_mut(&mut self) -> &mut Vec<Page> {
        &mut self.pages
    }
}

fn fumen_field_delta(
    from: [[CellColor; 10]; 24], to: [[CellColor; 10]; 24]
) -> [[usize; 10]; 24] {
    let mut deltas = [[0; 10]; 24];
    for y in 0..24 {
        for x in 0..10 {
            deltas[y][x] = 8 + to[y][x] as usize - from[y][x] as usize
        }
    }
    deltas
}

fn decode_cell_color(value: usize) -> Option<CellColor> {
    Some(match value {
        0 => CellColor::Empty,
        1 => CellColor::I,
        2 => CellColor::L,
        3 => CellColor::O,
        4 => CellColor::Z,
        5 => CellColor::T,
        6 => CellColor::J,
        7 => CellColor::S,
        8 => CellColor::Grey,
        _ => return None
    })
}

fn from_base64(c: char) -> Option<usize> {
    Some(match c {
        'A' ..= 'Z' => c as usize - 'A' as usize,
        'a' ..= 'z' => c as usize - 'a' as usize + 26,
        '0' ..= '9' => c as usize - '0' as usize + 52,
        '+' => 62,
        '/' => 63,
        _ => return None
    })
}
#[wasm_bindgen()]
impl Page {
    fn fumen_number(&self) -> u32 {
        self.piece.map(|p| p.fumen_number()).unwrap_or(0) + 240 * 32 * (
            self.rise as u32 +
            2 * self.mirror as u32 +
            8 * self.comment.is_some() as u32 +
            16 * !self.lock as u32
        )
    }

    
    fn fumen_field(&self) -> [[CellColor; 10]; 24] {
        let mut field = [[CellColor::Empty; 10]; 24];
        for y in 0..23 {
            field[22-y] = self.field[y];
        }
        field[23] = self.garbage_row;
        field
    }
    #[wasm_bindgen(js_name = "nextPage")]
    /// Create a page from this page in the same way as fumen does.
    ///
    /// This will apply the piece locking, line clear, rise, and mirror rules just like fumen does.
    pub fn next_page(&self) -> Page {
        let mut field = self.field;

        // do piece placement
        if let Some(piece) = self.piece {
            if self.lock {
                for &(x, y) in &piece.cells() {
                    field[y as usize][x as usize] = piece.kind.into();
                }
            }
        }

        // do line clear rule
        if self.lock {
            let mut y = 0;
            for i in 0..23 {
                let mut cleared = true;
                for x in 0..10 {
                    if field[i][x] == CellColor::Empty {
                        cleared = false;
                    }
                }
                if !cleared {
                    field[y] = field[i];
                    y += 1;
                }
            }
            for i in y..23 {
                field[i] = [CellColor::Empty; 10];
            }
        }

        // do "rise" rule
        if self.rise {
            for i in (1..23).rev() {
                field[i] = field[i-1];
            }
            field[0] = self.garbage_row;
        }

        // do "mirror" rule
        if self.mirror {
            for row in &mut field {
                row.reverse();
            }
        }

        Page {
            piece: if self.lock { None } else { self.piece },
            comment: None,
            rise: false,
            mirror: false,
            lock: self.lock,
            field,
            garbage_row: if self.rise {
                [CellColor::Empty; 10]
            } else {
                self.garbage_row
            }
        }
    }
    #[wasm_bindgen(getter)]
    pub fn comment(&self) -> Option<String> {
        self.comment.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn field(&self) -> Array {
        let array = Array::new();
        for row in &self.field {
            let array_row = Array::new();
            for cell in row {
            
                array_row.push(&JsValue::from(*cell as u8));
            }
        }
        array
    }
    #[wasm_bindgen(getter)]
    pub fn garbage_row(&self) -> Array {
        let array = Array::new();
        for cell in &self.garbage_row {
            array.push(&JsValue::from(*cell as u8));
        }
        array
    }
    #[wasm_bindgen(setter)]
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment
    }
    #[wasm_bindgen(setter)]
    pub fn set_field(&mut self, field: Array) {
        for (y, row) in field.iter().enumerate() {
            let row: Array = Array::from(&row);
            for (x, cell) in row.iter().enumerate() {
                let c = cell.as_f64();
                if let Some(c) = c {
                    
                    self.field[y][x] = PieceType::from_i64(c as i64).into();
                }
            }
        }
    }
    #[wasm_bindgen(setter)]
    pub fn set_garbage_row(&mut self, garbage_row: Array) {
        for (x, cell) in garbage_row.iter().enumerate() {
            let c = cell.as_f64();
            if let Some(c) = c {
                self.garbage_row[x] = PieceType::from_i64(c as i64).into();
            }
        }
    }
}
impl Page {
    pub fn get_field(&self) -> [[CellColor; 10]; 23] {
        self.field
    }
    pub fn get_garbage_row(&self) -> [CellColor; 10] {
        self.garbage_row
    }
    pub fn get_comment(&self) -> Option<String> {
        self.comment.clone()
    }
    pub fn set_field_rs(&mut self, field: [[CellColor; 10]; 23]) {
        self.field = field;
    }
    pub fn set_garbage_row_rs(&mut self, garbage_row: [CellColor; 10]) {
        self.garbage_row = garbage_row;
    }
    pub fn set_comment_rs(&mut self, comment: Option<String>) {
        self.comment = comment
    }
}
#[wasm_bindgen]
impl Piece {
    fn fumen_number(&self) -> u32 {
        self.kind as u32 +
            8 * self.rotation as u32 +
            32 * self.fumen_pos()
    }

    fn fumen_pos(&self) -> u32 {
        // Convert true SRS piece centers to fumen's system
        let x = match (self.kind, self.rotation) {
            (PieceType::S, RotationState::East) => self.x + 1,
            (PieceType::Z, RotationState::West) => self.x - 1,
            (PieceType::O, RotationState::West) => self.x - 1,
            (PieceType::O, RotationState::South) => self.x - 1,
            (PieceType::I, RotationState::South) => self.x - 1,
            _ => self.x
        };
        let y = match (self.kind, self.rotation) {
            (PieceType::S, RotationState::North) => self.y + 1,
            (PieceType::Z, RotationState::North) => self.y + 1,
            (PieceType::O, RotationState::North) => self.y + 1,
            (PieceType::O, RotationState::West) => self.y + 1,
            (PieceType::I, RotationState::West) => self.y + 1,
            _ => self.y
        };

        x + (22 - y) * 10
    }

    fn cells(&self) -> [(i32, i32); 4] {
        let mut cells = match self.kind {
            PieceType::I => [(-1, 0), (0, 0), (1, 0), (2, 0)],
            PieceType::O => [(0, 0), (1, 0), (0, 1), (1, 1)],
            PieceType::T => [(-1, 0), (0, 0), (1, 0), (0, 1)],
            PieceType::L => [(-1, 0), (0, 0), (1, 0), (1, 1)],
            PieceType::J => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
            PieceType::S => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            PieceType::Z => [(1, 0), (0, 0), (0, 1), (-1, 1)]
        };

        for (x, y) in &mut cells {
            match self.rotation {
                RotationState::North => {}
                RotationState::East => {
                    std::mem::swap(x, y);
                    *y = -*y;
                }
                RotationState::South => {
                    *x = -*x;
                    *y = -*y;
                }
                RotationState::West => {
                    std::mem::swap(x, y);
                    *x = -*x;
                }
            }

            *x += self.x as i32;
            *y += self.y as i32;
        }

        cells
    }
}

impl Default for Fumen {
    fn default() -> Self {
        Fumen {
            pages: vec![],
            guideline: true
        }
    }
}
impl Default for Page {
    fn default() -> Self {
        Page {
            piece: None,
            field: [[CellColor::Empty; 10]; 23],
            garbage_row: [CellColor::Empty; 10],
            rise: false,
            mirror: false,
            lock: true,
            comment: None
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Fumen {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(&self.encode())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Fumen {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Fumen;
            fn expecting(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(fmt, "an encoded fumen string")
            }
            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Fumen, E> {
                Fumen::decode(s).map_err(E::custom)
            }
        }
        de.deserialize_str(Visitor)
    }
}

impl From<PieceType> for CellColor {
    fn from(v: PieceType) -> CellColor {
        match v {
            PieceType::I => CellColor::I,
            PieceType::L => CellColor::L,
            PieceType::O => CellColor::O,
            PieceType::Z => CellColor::Z,
            PieceType::T => CellColor::T,
            PieceType::J => CellColor::J,
            PieceType::S => CellColor::S,
        }
    }
}

fn js_escape(s: &str) -> Vec<u8> {
    const HEX_DIGITS: [u8; 16] = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
        b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F'
    ];

    let mut result = Vec::new();
    for c in s.chars() {
        match c {
            'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' |
            '@' | '*' | '_' | '+' | '-' | '.' | '/' => result.push(c as u8),
            '\u{0}' ..= '\u{FF}' => {
                result.push(b'%');
                result.push(HEX_DIGITS[(c as usize) >> 4 & 0xF]);
                result.push(HEX_DIGITS[(c as usize) >> 0 & 0xF]);
            }
            _ => {
                let mut buf = [0; 2];
                for &mut c in c.encode_utf16(&mut buf) {
                    result.extend_from_slice(b"%u");
                    result.push(HEX_DIGITS[(c as usize) >> 12 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 8 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 4 & 0xF]);
                    result.push(HEX_DIGITS[(c as usize) >> 0 & 0xF]);
                }
            }
        }
    }
    result
}

fn js_unescape(s: &str) -> String {
    fn decode(mut i: impl Iterator<Item=char>, c: usize) -> u16 {
        let mut number = 0;
        for _ in 0..c {
            if let Some(c) = i.next() {
                if let Some(v) = c.to_digit(16) {
                    number *= 16;
                    number += v as u16;
                }
            }
        }
        number
    }

    let mut iter = s.chars().peekable();
    let mut result_utf16 = vec![];
    while let Some(c) = iter.next() {
        match c {
            '%' => match iter.peek() {
                Some('u') => {
                    iter.next();
                    result_utf16.push(decode(&mut iter, 4));
                }
                _ => result_utf16.push(decode(&mut iter, 2))
            }
            _ => result_utf16.push(c as u16)
        }
    }
    String::from_utf16_lossy(&result_utf16)
}
#[wasm_bindgen]
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct DecodeFumenError;

impl std::fmt::Display for DecodeFumenError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "the string does not contain valid fumen data")
    }
}

impl std::error::Error for DecodeFumenError {}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn empty() {
        let fumen = Fumen::default();
        assert_eq!(fumen.encode(), "v115@");
        unsafe {
            assert_eq!(Fumen::decode("v115@"), Ok(fumen));
        }
        
    }

    #[test]
    fn one_page_lock_piece() {
        let mut fumen = Fumen::default();
        unsafe {
            (*fumen.add_page()).piece = Some(Piece {
                kind: PieceType::T,
                rotation: RotationState::North,
                x: 2,
                y: 0
            });
            assert_eq!(fumen.encode(), "v115@vhAVPJ");
            assert_eq!(Fumen::decode("v115@vhAVPJ"), Ok(fumen));
        }

    }

    #[test]
    fn lock_piece() {
        let mut fumen = Fumen::default();
        unsafe {
            (*fumen.add_page()).piece = Some(Piece {
                kind: PieceType::T,
                rotation: RotationState::North,
                x: 2,
                y: 0
            });
            fumen.pages.push(Page::default());
            assert_eq!(fumen.encode(), "v115@vhAVPJThQLHeSLPeAAA");
            assert_eq!(Fumen::decode("v115@vhAVPJThQLHeSLPeAAA"), Ok(fumen));
        }
       
    }

    #[test]
    fn o_piece_wobble() {
        unsafe {
            let mut fumen = Fumen::default();
            let page = fumen.add_page();
            (*page).field[2][3] = CellColor::Grey;
            (*page).field[5][3] = CellColor::Grey;
            (*page).field[8][3] = CellColor::Grey;
            (*page).piece = Some(Piece {
                kind: PieceType::O,
                rotation: RotationState::North,
                x: 3, y: 0
            });
            (*fumen.add_page()).piece = Some(Piece {
                kind: PieceType::O,
                rotation: RotationState::West,
                x: 4, y: 3
            });
            (*fumen.add_page()).piece = Some(Piece {
                kind: PieceType::O,
                rotation: RotationState::South,
                x: 4, y: 7
            });
            (*fumen.add_page()).piece = Some(Piece {
                kind: PieceType::O,
                rotation: RotationState::East,
                x: 3, y: 10
            });
            fumen.pages.push(Page::default());
            assert_eq!(
                fumen.encode(),
                "v115@OgA8ceA8ceA8jezKJvhC7bBjMBr9A6fxSHexSHeAAIexSHexSHeAAIexSHexSHeAAIexSHexSOeAAA"
            );
            assert_eq!(Fumen::decode(
                "v115@OgA8ceA8ceA8jezKJvhC7bBjMBr9A6fxSHexSHeAAIexSHexSHeAAIexSHexSHeAAIexSHexSOeAAA"
            ), Ok(fumen));
        }

    }

    #[test]
    fn fumen_field() {
        let mut page = Page::default();
        page.field[0] = [CellColor::Grey; 10];
        page.garbage_row[0] = CellColor::Grey;
        let mut fumen_field = [[CellColor::Empty; 10]; 24];
        fumen_field[22] = [CellColor::Grey; 10];
        fumen_field[23][0] = CellColor::Grey;
        assert_eq!(page.fumen_field(), fumen_field);
    }

    #[test]
    fn fumen_field_deltas() {
        unsafe {
            let mut page = Page::default();
            let empty = page.fumen_field();
            page.field[0] = [CellColor::Grey; 10];
            page.garbage_row[0] = CellColor::Grey;
            let mut deltas = [[8; 10]; 24];
            deltas[22] = [16; 10];
            deltas[23][0] = 16;
            assert_eq!(fumen_field_delta(empty, page.fumen_field()), deltas);

        }
    }
      

    #[test]
    fn simple_field() {
        unsafe {
            let mut fumen = Fumen::default();
            (*fumen.add_page()).field[22][0] = CellColor::Grey;
            assert_eq!(fumen.encode(), "v115@A8uhAgH");
            assert_eq!(Fumen::decode("v115@A8uhAgH"), Ok(fumen));
        }

    }

    #[test]
    fn arbitrary_field() {
        unsafe {
            let mut fumen = Fumen::default();
            let page = fumen.add_page();
            (*page).field[0] = [CellColor::Grey; 10];
            (*page).field[0][4] = CellColor::Empty;
            (*page).field[0][7] = CellColor::T;
            (*page).field[1] = [CellColor::S; 10];
            (*page).field[1][1] = CellColor::Empty;
            (*page).field[1][9] = CellColor::L;
            (*page).field[2] = [CellColor::Z; 10];
            (*page).field[2][6] = CellColor::Empty;
            (*page).field[2][2] = CellColor::O;
            (*page).field[3] = [CellColor::I; 10];
            (*page).field[3][2] = CellColor::Empty;
            (*page).field[3][6] = CellColor::J;
            assert_eq!(fumen.encode(), "v115@9gxhAeyhg0yhBtQpCtAeCtQ4AeW4glD8AeB8wwB8JeAgH");
            assert_eq!(
                Fumen::decode("v115@9gxhAeyhg0yhBtQpCtAeCtQ4AeW4glD8AeB8wwB8JeAgH"),
                Ok(fumen)
            );
        }
       
    }

    #[test]
    fn line_clear() {
        unsafe {
            let mut fumen = Fumen::default();
            (*fumen.add_page()).field[0] = [CellColor::Grey; 10];
            fumen.add_page();
            assert_eq!(fumen.encode(), "v115@bhJ8JeAgHvhAAAA");
            assert_eq!(Fumen::decode("v115@bhJ8JeAgHvhAAAA"), Ok(fumen));

        }
      
    }

    #[test]
    fn rise() {
        unsafe {
            let mut fumen = Fumen::default();
            let page = fumen.add_page();
            (*page).field[0][1] = CellColor::I;
            (*page).garbage_row[4] = CellColor::Grey;
            (*page).rise = true;
            fumen.add_page();
            fumen.pages.push(Page::default());
            assert_eq!(fumen.encode(), "v115@chwhLeA8EeAYJvhAAAAShQaLeAAOeAAA");
            assert_eq!(Fumen::decode("v115@chwhLeA8EeAYJvhAAAAShQaLeAAOeAAA"), Ok(fumen));
        }
        }
       

    #[test]
    fn mirror() {
        unsafe {
            let mut fumen = Fumen::default();
            let page = fumen.add_page();
            (*page).field[0] = [
                CellColor::I, CellColor::L, CellColor::O, CellColor::Z, CellColor::T,
                CellColor::J, CellColor::S, CellColor::Grey, CellColor::Empty, CellColor::Empty
            ];
            (*page).mirror = true;
            fumen.add_page();
            fumen.pages.push(Page::default());
            assert_eq!(fumen.encode(), "v115@bhwhglQpAtwwg0Q4A8LeAQLvhAAAAdhAAwDgHQLAPwSgWQaJeAAA");
            assert_eq!(
                Fumen::decode("v115@bhwhglQpAtwwg0Q4A8LeAQLvhAAAAdhAAwDgHQLAPwSgWQaJeAAA"),
                Ok(fumen)
            );
        }
      
    }

    #[test]
    fn comment() {
        unsafe {
            let mut fumen = Fumen::default();
            (*fumen.add_page()).comment = Some("Hello World!".to_owned());
            assert_eq!(fumen.encode(), "v115@vhAAgWQAIoMDEvoo2AXXaDEkoA6A");
            assert_eq!(Fumen::decode("v115@vhAAgWQAIoMDEvoo2AXXaDEkoA6A"), Ok(fumen));    
        }
    }

    #[test]
    fn comment_unicode() {
        unsafe {
            let mut fumen = Fumen::default();
            (*fumen.add_page()).comment = Some("こんにちは世界".to_owned());
            assert_eq!(
                fumen.encode(), "v115@vhAAgWqAlvs2A1sDfEToABBlvs2AWDEfET4J6Alvs2AWJEfE0H3KBlvtHB00AAA"
            );
            assert_eq!(Fumen::decode(
            "v115@vhAAgWqAlvs2A1sDfEToABBlvs2AWDEfET4J6Alvs2AWJEfE0H3KBlvtHB00AAA"
            ), Ok(fumen));
    }
    }

    #[test]
    fn comment_surrogate_pair() {
        unsafe {
            let mut fumen = Fumen::default();
            (*fumen.add_page()).comment = Some("🂡🆛🏍😵".to_owned());
            assert_eq!(
                fumen.encode(),
                "v115@vhAAgWwAl/SSBzEEfEEFj6Al/SSBzEEfEkGpzBl/SSBzEEfEkpv6Bl/SSBTGEfEEojHB"
            );
            assert_eq!(Fumen::decode(
            "v115@vhAAgWwAl/SSBzEEfEEFj6Al/SSBzEEfEkGpzBl/SSBzEEfEkpv6Bl/SSBTGEfEEojHB"
            ), Ok(fumen));
      }
    }

    #[test]
    fn not_a_fumen() {
        unsafe{
            assert_eq!(Fumen::decode(""), Err(DecodeFumenError));
            assert_eq!(Fumen::decode("v115@hello world"), Err(DecodeFumenError));
            assert_eq!(Fumen::decode("無効"), Err(DecodeFumenError));
        }
    }

    #[test]
    fn no_piece_lock() {
        unsafe {
            let mut fumen = Fumen::default();
            let page = fumen.add_page();
            (*page).field[0] = [CellColor::Grey; 10];
            (*page).lock = false;
            (*page).piece = Some(Piece {
                kind: PieceType::T,
                rotation: RotationState::North,
                x: 3,
                y: 1
            });
            fumen.add_page();
            assert_eq!(fumen.encode(), "v115@bhJ8Je1KnvhA1qf");
            assert_eq!(Fumen::decode("v115@bhJ8Je1KnvhA1qf"), Ok(fumen));
        } 
    }
}