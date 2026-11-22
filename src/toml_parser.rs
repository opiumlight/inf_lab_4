use crate::scanner::Scanner;
use crate::tree::insert;
use crate::value::{Statement, Value};
use core::str::FromStr;

pub type ParseResult<T> = Result<T, String>;

pub struct TomlParser {
    sc: Scanner,
}

impl TomlParser {
    pub fn new(doc: &str) -> Self {
        Self {
            sc: Scanner::new(doc),
        }
    }

    /// Возвращает номер строки и столбца из позиции сканнера в данный момент
    /// Метод был добавлен слишком поздно, так что почти не используется, и все ошибки неизвестно где
    fn get_current_position(&self) -> (usize, usize) {
        let pos = self.sc.pos();
        let mut last_newline: usize = 0;
        let mut lines: usize = 1;
        for (i, &c) in self.sc.chars_slice(pos).iter().enumerate() {
            if c == '\n' {
                lines += 1;
                last_newline = i + 1;
            }
        }
        (lines, pos - last_newline + 1)
    }

    // Document
    pub fn parse(&mut self) -> ParseResult<Vec<Statement>> {
        let mut statements = Vec::new();
        statements.push(self.parse_expression()?);
        while self.parse_newline().is_ok() {
            let new_expr = self.parse_expression();
            if new_expr.is_err() {
                break;
            }
            statements.push(new_expr?);
        }
        if self.sc.peek().is_some() {
            let (lines, pos) = self.get_current_position();
            return Err(format!(
                "Found incomplete expression at line {} pos. {}",
                lines, pos
            ));
        }
        Ok(statements)
    }
    fn parse_newline(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("\r\n") {
            Ok(())
        } else if self.sc.eat('\n') {
            Ok(())
        } else {
            Err(format!("Failed to parse newline. {}", self.sc.pos()))
        }
    }
    fn parse_expression(&mut self) -> ParseResult<Statement> {
        self.parse_ws()?;
        let mark = self.sc.mark();
        let keyval = self.parse_keyval();
        if keyval.is_err() {
            self.sc.reset(mark);
            let table = self.parse_table();
            if table.is_err() {
                self.parse_comment().unwrap_or(());
                return Ok(Statement::Empty);
            }
            self.parse_ws()?;
            self.parse_comment().unwrap_or(());
            return Ok(table?);
        }
        self.parse_ws()?;
        self.parse_comment().unwrap_or(());
        let (key, val) = keyval?;
        Ok(Statement::KeyVal(key, val))
    }
    fn parse_ws(&mut self) -> ParseResult<()> {
        loop {
            if !(self.sc.eat('\t') || self.sc.eat(' ')) {
                break;
            }
        }
        Ok(())
    }
    fn parse_comment(&mut self) -> ParseResult<()> {
        self.sc.expect('#')?;
        while !"\r\n".contains(self.sc.peek().unwrap_or('\n')) {
            self.sc.bump();
        }
        Ok(())
    }

    // Key-value
    fn parse_keyval(&mut self) -> ParseResult<(Vec<String>, Value)> {
        let key = self.parse_key()?;
        self.parse_keyval_sep()?;
        let val = self.parse_val()?;
        Ok((key, val))
    }
    fn parse_key(&mut self) -> ParseResult<Vec<String>> {
        let mark = self.sc.mark();
        let dk = self.parse_dotted_key();
        if dk.is_err() {
            self.sc.reset(mark);
            return Ok(vec![self.parse_simple_key()?]);
        }
        Ok(dk?)
    }
    fn parse_val(&mut self) -> ParseResult<Value> {
        let mark = self.sc.mark();
        let st = self.parse_string();
        if st.is_ok() {
            return Ok(Value::Str(st?));
        }
        self.sc.reset(mark);
        let boolean = self.parse_boolean();
        if boolean.is_ok() {
            return Ok(Value::Bool(boolean?));
        }
        self.sc.reset(mark);
        let array = self.parse_array();
        if array.is_ok() {
            return Ok(array?);
        }
        self.sc.reset(mark);
        let inline_table = self.parse_inline_table();
        if inline_table.is_ok() {
            return Ok(inline_table?);
        }
        self.sc.reset(mark);
        let date_time = self.parse_date_time();
        if date_time.is_ok() {
            return Ok(date_time?);
        }
        self.sc.reset(mark);
        let float = self.parse_float();
        if float.is_ok() {
            return Ok(Value::Float(float?));
        }
        self.sc.reset(mark);
        let integer = self.parse_integer();
        if let Ok(n) = integer {
            if i64::try_from(n).is_ok() {
                return Ok(Value::Int(n));
            }
        }
        self.sc.reset(mark);
        Err("No valid val found.".to_string())
    }
    fn parse_simple_key(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        let quoted = self.parse_quoted_key();
        if quoted.is_ok() {
            return Ok(quoted?);
        }
        self.sc.reset(mark);
        self.parse_unquoted_key()
    }
    fn parse_unquoted_key(&mut self) -> ParseResult<String> {
        let mut key = Vec::<char>::new();
        loop {
            let ch = self.sc.peek().unwrap_or('\\');
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                key.push(ch);
                self.sc.bump();
            } else {
                break;
            }
        }
        if key.len() == 0 {
            return Err("Empty key.".to_string());
        }
        Ok(key.iter().collect::<String>())
    }
    fn parse_quoted_key(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        let basic = self.parse_basic_string();
        if basic.is_ok() {
            return Ok(basic?);
        }
        self.sc.reset(mark);
        self.parse_literal_string()
    }
    fn parse_dotted_key(&mut self) -> ParseResult<Vec<String>> {
        let mut key = vec![self.parse_simple_key()?];
        loop {
            let sep = self.parse_dot_sep();
            if sep.is_err() {
                break;
            }
            let key2 = self.parse_simple_key()?;
            key.push(key2);
        }
        Ok(key)
    }
    fn parse_dot_sep(&mut self) -> ParseResult<()> {
        self.parse_ws()?;
        self.sc.expect('.')?;
        self.parse_ws()
    }
    fn parse_keyval_sep(&mut self) -> ParseResult<()> {
        self.parse_ws()?;
        self.sc.expect('=')?;
        self.parse_ws()
    }

    // Strings
    fn parse_string(&mut self) -> ParseResult<String> {
        let mut mark = self.sc.mark();
        let ml_basic_string = self.parse_ml_basic_string();
        if ml_basic_string.is_ok() {
            return Ok(ml_basic_string?);
        }
        self.sc.reset(mark);
        mark = self.sc.mark();
        let basic_string = self.parse_basic_string();
        if basic_string.is_ok() {
            return Ok(basic_string?);
        }
        self.sc.reset(mark);
        mark = self.sc.mark();
        let ml_literal_string = self.parse_ml_literal_string();
        if ml_literal_string.is_ok() {
            return Ok(ml_literal_string?);
        }
        self.sc.reset(mark);
        let literal_string = self.parse_literal_string();
        if literal_string.is_ok() {
            return Ok(literal_string?);
        }
        Err("No valid string found.".to_string())
    }
    fn parse_basic_string(&mut self) -> ParseResult<String> {
        self.sc.expect('"')?;
        let mut chars = Vec::<char>::new();
        loop {
            let mark = self.sc.mark();
            let ch = self.parse_basic_char();
            if ch.is_err() {
                self.sc.reset(mark);
                break;
            }
            chars.push(ch?);
        }
        self.sc.expect('"')?;
        Ok(chars.iter().collect::<String>())
    }
    fn parse_basic_char(&mut self) -> ParseResult<char> {
        let unescaped = self.parse_basic_unescaped();
        if unescaped.is_ok() {
            return Ok(unescaped?);
        }
        self.parse_escaped()
    }
    fn parse_basic_unescaped(&mut self) -> ParseResult<char> {
        let ch = self.sc.peek().unwrap_or('\\');
        if ch == '\\' || ch == '"' || (ch < ' ' && ch != '\t') || ch == '\x7f' {
            return Err("Expected unescaped basic char, got \\ or EOF".into());
        }
        self.sc.bump();
        Ok(ch)
    }
    // TODO
    fn parse_escaped(&mut self) -> ParseResult<char> {
        self.sc.expect('\\')?;
        let escaped = self.sc.bump().unwrap_or('_');
        match escaped {
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            'b' => Ok('\x08'),
            'e' => Ok('\x1b'),
            'f' => Ok('\x0C'),
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            'x' => self.parse_unicode_escape(2),
            'u' => self.parse_unicode_escape(4),
            'U' => self.parse_unicode_escape(8),
            _ => Err(format!("Invalid escape \\{}", escaped)),
        }
    }
    fn parse_unicode_escape(&mut self, digits: usize) -> ParseResult<char> {
        let mut code: u32 = 0;
        for _ in 0..digits {
            code = code * 16 + self.parse_hexdigit()?.to_digit(16).unwrap();
        }
        char::from_u32(code).ok_or_else(|| format!("Invalid unicode scalar {:X}", code))
    }

    // Multiline basic strings
    fn parse_ml_basic_string(&mut self) -> ParseResult<String> {
        self.parse_ml_basic_string_delim()?;
        self.parse_newline().unwrap_or(());
        let body = self.parse_ml_basic_body()?;
        self.parse_ml_basic_string_delim()?;
        Ok(body)
    }
    fn parse_ml_basic_string_delim(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("\"\"\"") {
            Ok(())
        } else {
            Err("Error on parsing mlb string delim.".to_string())
        }
    }
    fn parse_ml_basic_body(&mut self) -> ParseResult<String> {
        let mut res = String::new();
        loop {
            if self.sc.starts_with("\"\"\"\"\"") {
                res.push_str("\"\"");
                self.sc.eat_str("\"\"");
                continue;
            }
            if self.sc.starts_with("\"\"\"\"") {
                res.push('"');
                self.sc.eat('"');
                continue;
            }
            if self.sc.starts_with("\"\"\"") {
                break;
            }
            if self.sc.eat('"') {
                res.push('"');
                continue;
            }
            let mark = self.sc.mark();
            if self.parse_mlb_escaped_nl().is_ok() {
                continue;
            }
            self.sc.reset(mark);
            if self.parse_newline().is_ok() {
                res.push('\n');
                continue;
            }
            if let Ok(c) = self.parse_basic_char() {
                res.push(c);
                continue;
            }
            self.sc.reset(mark);
            break;
        }
        Ok(res)
    }

    fn parse_mlb_escaped_nl(&mut self) -> ParseResult<()> {
        self.sc.expect('\\')?;
        self.parse_ws()?;
        self.parse_newline()?;
        loop {
            if !"\t \r\n".contains(self.sc.peek().unwrap_or('_')) {
                break;
            } else {
                self.sc.bump();
            }
        }
        Ok(())
    }

    // Literal strings
    fn parse_literal_string(&mut self) -> ParseResult<String> {
        self.sc.expect('\'')?;
        let mut chars = Vec::<char>::new();
        loop {
            let ch = self.parse_literal_char();
            if ch.is_err() {
                break;
            }
            chars.push(ch?);
        }
        self.sc.expect('\'')?;
        Ok(chars.iter().collect::<String>())
    }
    // TODO: сделать как в abnf
    fn parse_literal_char(&mut self) -> ParseResult<char> {
        let ch = self.sc.peek().unwrap_or('\'');
        if ch == '\'' || (ch < ' ' && ch != '\t') || ch == '\x7f' {
            return Err("Not a literal char".into());
        }
        Ok(self.sc.bump().unwrap())
    }
    fn parse_ml_literal_string(&mut self) -> ParseResult<String> {
        self.parse_ml_literal_string_delim()?;
        self.parse_newline().unwrap_or(());
        let body = self.parse_ml_literal_body()?;
        self.parse_ml_literal_string_delim()?;
        Ok(body)
    }
    fn parse_ml_literal_string_delim(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("'''") {
            Ok(())
        } else {
            Err("Error on parsing mll string delim.".to_string())
        }
    }
    fn parse_ml_literal_body(&mut self) -> ParseResult<String> {
        let mut res = String::new();
        loop {
            if self.sc.starts_with("'''''") {
                res.push_str("''");
                self.sc.eat_str("''");
                continue;
            }
            if self.sc.starts_with("''''") {
                res.push('\'');
                self.sc.eat('\'');
                continue;
            }
            if self.sc.starts_with("'''") {
                break;
            }
            if self.sc.eat('\'') {
                res.push('\'');
                continue;
            }
            if let Ok(c) = self.parse_mll_content() {
                res.push(c);
                continue;
            }
            break;
        }
        Ok(res)
    }
    fn parse_mll_content(&mut self) -> ParseResult<char> {
        if let Ok(ch) = self.parse_literal_char() {
            Ok(ch)
        } else if let Ok(_) = self.parse_newline() {
            Ok('\n')
        } else {
            Err("No valid mll content".to_string())
        }
    }

    // Integers
    fn parse_integer(&mut self) -> ParseResult<i128> {
        let mark = self.sc.mark();
        let mut int = self.parse_hex_int();
        if int.is_ok() {
            return Ok(int?);
        }
        self.sc.reset(mark);
        int = self.parse_oct_int();
        if int.is_ok() {
            return Ok(int?);
        }
        self.sc.reset(mark);
        int = self.parse_bin_int();
        if int.is_ok() {
            return Ok(int?);
        }
        self.sc.reset(mark);
        int = self.parse_dec_int();
        if int.is_ok() {
            return Ok(int?);
        }
        self.sc.reset(mark);
        Err("No valid integer found".to_string())
    }
    fn parse_digit0_7(&mut self) -> ParseResult<char> {
        for d in '0'..='7' {
            if self.sc.eat(d) {
                return Ok(d);
            }
        }
        Err(format!("Expected oct digit, got {:?}", self.sc.peek()))
    }
    fn parse_digit0_1(&mut self) -> ParseResult<char> {
        if self.sc.eat('0') {
            return Ok('0');
        }
        if self.sc.eat('1') {
            return Ok('1');
        }
        Err(format!("Expected bin digit, got {:?}", self.sc.peek()))
    }
    fn parse_hex_prefix(&mut self) -> ParseResult<()> {
        self.sc.expect('0')?;
        self.sc.expect('x')
    }
    fn parse_oct_prefix(&mut self) -> ParseResult<()> {
        self.sc.expect('0')?;
        self.sc.expect('o')
    }
    fn parse_bin_prefix(&mut self) -> ParseResult<()> {
        self.sc.expect('0')?;
        self.sc.expect('b')
    }
    fn parse_dec_int(&mut self) -> ParseResult<i128> {
        let sgn = if self.sc.eat('-') {
            -1
        } else if self.sc.eat('+') || self.sc.peek().is_some() {
            1
        } else {
            return Err("Unexpected EOF".into());
        };
        Ok(sgn * self.parse_unsigned_dec_int()? as i128)
    }
    fn parse_unsigned_dec_int(&mut self) -> ParseResult<u128> {
        let mut res: u128 = self.parse_digit()? as u128;
        if res == 0 || !"0123456789_".contains(self.sc.peek().unwrap_or('/')) {
            return Ok(res);
        }
        loop {
            let underscore = self.sc.eat('_');
            let d = self.parse_digit();
            if d.is_err() {
                if underscore {
                    return Err("Dec int cannot end with _".into());
                }
                break;
            }
            let d = d? as u128;
            res = res
                .checked_mul(10)
                .and_then(|r| r.checked_add(d))
                .ok_or("Integer overflow")?;
        }
        Ok(res)
    }
    fn parse_radix_digits(
        &mut self,
        digit: fn(&mut Self) -> ParseResult<char>,
        radix: u32,
    ) -> ParseResult<i128> {
        let mut res = String::from(digit(self)?);
        loop {
            let underscore = self.sc.eat('_');
            match digit(self) {
                Ok(d) => res.push(d),
                Err(_) if underscore => return Err("Expected digit after _".into()),
                Err(_) => break,
            }
        }
        i128::from_str_radix(&res, radix).map_err(|e| e.to_string())
    }
    fn parse_hex_int(&mut self) -> ParseResult<i128> {
        self.parse_hex_prefix()?;
        self.parse_radix_digits(Self::parse_hexdigit, 16)
    }
    fn parse_oct_int(&mut self) -> ParseResult<i128> {
        self.parse_oct_prefix()?;
        self.parse_radix_digits(Self::parse_digit0_7, 8)
    }
    fn parse_bin_int(&mut self) -> ParseResult<i128> {
        self.parse_bin_prefix()?;
        self.parse_radix_digits(Self::parse_digit0_1, 2)
    }

    // Floats
    fn parse_float(&mut self) -> ParseResult<f64> {
        let mut mark = self.sc.mark();
        if let Ok(sf) = self.parse_special_float() {
            return Ok(sf);
        }
        self.sc.reset(mark);
        let int_part = self.parse_float_int_part()?;
        mark = self.sc.mark();
        let exp = self.parse_exp();
        if exp.is_ok() {
            return Ok(f64::from_str(format!("{int_part}{}", exp?).as_ref()).unwrap());
        }
        self.sc.reset(mark);
        let frac = self.parse_frac()?;
        let exp = self.parse_exp().unwrap_or("".to_string());
        Ok(f64::from_str(format!("{int_part}{frac}{exp}").as_str()).unwrap())
    }
    fn parse_float_int_part(&mut self) -> ParseResult<String> {
        let res: String = if self.sc.eat('-') {
            "-".into()
        } else {
            self.sc.eat('+');
            "+".into()
        };
        Ok(res + self.parse_unsigned_dec_int()?.to_string().as_str())
    }
    fn parse_frac(&mut self) -> ParseResult<String> {
        self.parse_decimal_point()?;
        Ok(format!(".{}", self.parse_zero_prefixable_int()?))
    }
    fn parse_decimal_point(&mut self) -> ParseResult<()> {
        self.sc.expect('.')
    }
    fn parse_zero_prefixable_int(&mut self) -> ParseResult<String> {
        let mut res = format!("{}", self.parse_digit()?);
        loop {
            let new_digit = self.parse_digit();
            if self.sc.eat('_') {
                if new_digit.is_err() {
                    return Err("Incorrect zero prefixable int".into());
                }
                res += format!("{}", new_digit?).as_str();
                continue;
            } else {
                if new_digit.is_err() {
                    break;
                } else {
                    res += format!("{}", new_digit?).as_str();
                }
            }
        }
        Ok(res)
    }
    fn parse_exp(&mut self) -> ParseResult<String> {
        self.sc.expect_any(&['e', 'E'])?;
        Ok(format!("e{}", self.parse_float_exp_part()?))
    }
    fn parse_float_exp_part(&mut self) -> ParseResult<String> {
        let res: String = if self.sc.eat('-') {
            "-".into()
        } else {
            self.sc.eat('+');
            "".into()
        };
        Ok(res + self.parse_zero_prefixable_int()?.as_ref())
    }
    fn parse_special_float(&mut self) -> ParseResult<f64> {
        if self.sc.eat('-') {
            if self.sc.eat_str("inf") {
                return Ok(f64::NEG_INFINITY);
            }
            if self.sc.eat_str("nan") {
                return Ok(f64::NAN);
            }
        }
        if self.sc.eat('+') {
            if self.sc.eat_str("inf") {
                return Ok(f64::INFINITY);
            }
            if self.sc.eat_str("nan") {
                return Ok(f64::NAN);
            }
        }
        if self.sc.eat_str("inf") {
            return Ok(f64::INFINITY);
        }
        if self.sc.eat_str("nan") {
            return Ok(f64::NAN);
        }
        Err("No special float found".into())
    }

    // Booleans
    fn parse_boolean(&mut self) -> ParseResult<bool> {
        if self.parse_true().is_ok() {
            return Ok(true);
        }
        if self.parse_false().is_ok() {
            return Ok(false);
        }
        Err("expected a boolean value".into())
    }
    fn parse_true(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("true") {
            Ok(())
        } else {
            Err("expected true".to_string())
        }
    }
    fn parse_false(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("false") {
            Ok(())
        } else {
            Err("expected false".to_string())
        }
    }

    // Date and time
    fn parse_date_time(&mut self) -> ParseResult<Value> {
        let mark = self.sc.mark();
        if let Ok(t) = self.parse_offset_date_time() {
            return Ok(Value::DateTime(t));
        }
        self.sc.reset(mark);
        if let Ok(t) = self.parse_local_date_time() {
            return Ok(Value::DateTime(t));
        }
        self.sc.reset(mark);
        if let Ok(t) = self.parse_local_date() {
            return Ok(Value::DateTime(t));
        }
        self.sc.reset(mark);
        if let Ok(t) = self.parse_local_time() {
            return Ok(Value::DateTime(t));
        }
        self.sc.reset(mark);
        Err("not a valid datetime format".into())
    }
    fn parse_date_fullyear(&mut self) -> ParseResult<u16> {
        let mut val = self.parse_digit()? as u16;
        for _ in 0..3 {
            val *= 10;
            val += self.parse_digit()? as u16;
        }
        Ok(val)
    }
    fn parse_date_month(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 13 && val > 0 {
            return Ok(val);
        }
        Err("month should be between 1 and 12 inclusive".into())
    }
    /// `NOTE:` МЕТОД НЕ УЧИТЫВАЕТ ОСОБЕННОСТИ ОТДЕЛЬНОГО МЕСЯЦА. Проверка происходит в другом месте,
    /// а именно `parse_full_date`
    fn parse_date_mday(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 32 && val > 0 {
            return Ok(val);
        }
        Err("month day should be between 1 and 31 inclusive".into())
    }
    fn parse_time_delim(&mut self) -> ParseResult<char> {
        match self.sc.bump() {
            Some(ch) if ch == 'T' => Ok(ch),
            Some(ch) if ch == 't' => Ok(ch),
            Some(ch) if ch == ' ' => Ok(ch),
            Some(ch) => Err(format!("Expected T, t, or space as delim, got {}", ch)),
            None => Err("Unexpected EOF".into()),
        }
    }
    fn parse_time_hour(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 24 {
            return Ok(val);
        }
        Err("hour should be between 0 and 23 inclusive".into())
    }
    fn parse_time_minute(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 60 {
            return Ok(val);
        }
        Err("minute should be between 0 and 59 inclusive".into())
    }
    fn parse_time_second(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 60 {
            return Ok(val);
        }
        Err("second should be between 0 and 59 inclusive".into())
    }
    fn parse_time_secfrac(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        self.sc.expect('.')?;
        let mut res = vec!['.'];
        let first = self.parse_digit();
        if first.is_err() {
            self.sc.reset(mark);
            let (line, pos) = self.get_current_position();
            return Err(format!(
                "Expected digit after . at line {}  pos {}",
                line, pos
            ));
        }
        res.push((b'0' + first?) as char);
        while let Ok(c) = self.parse_digit() {
            res.push((b'0' + c) as char);
        }
        Ok(res.iter().collect())
    }
    fn parse_time_numoffset(&mut self) -> ParseResult<String> {
        let sign: char;
        if self.sc.eat('-') {
            sign = '-';
        } else if self.sc.eat('+') {
            sign = '+';
        } else {
            return Err("expected -/+ at numoffset beginning".into());
        }
        let time_hour = self.parse_time_hour()?;
        self.sc.expect(':')?;
        let time_minute = self.parse_time_minute()?;
        Ok(format!("{}{:02}:{:02}", sign, time_hour, time_minute))
    }
    fn parse_time_offset(&mut self) -> ParseResult<String> {
        if self.sc.eat('Z') {
            return Ok("Z".into());
        }
        self.parse_time_numoffset()
    }
    fn parse_partial_time(&mut self) -> ParseResult<String> {
        let time_hour = self.parse_time_hour()?;
        self.sc.expect(':')?;
        let time_minute = self.parse_time_minute()?;
        let mut res = format!("{:02}:{:02}", time_hour, time_minute);
        let mark = self.sc.mark();
        if self.sc.eat(':')
            && let Ok(second) = self.parse_time_second()
        {
            res += format!(":{:02}", second).as_ref();
            if let Ok(secfrac) = self.parse_time_secfrac() {
                res += format!("{}", secfrac).as_ref();
            }
        } else {
            self.sc.reset(mark);
        }
        Ok(res)
    }
    fn parse_full_date(&mut self) -> ParseResult<String> {
        let date_fullyear = self.parse_date_fullyear()?;
        self.sc.expect('-')?;
        let date_month = self.parse_date_month()?;
        self.sc.expect('-')?;
        let date_mday = self.parse_date_mday()?;
        let leap = (date_fullyear % 4 == 0 && date_fullyear % 100 != 0) || date_fullyear % 400 == 0;
        let days = match date_month {
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => 31,
        };
        if date_mday > days {
            return Err(format!("в месяце {} нет дня {}", date_month, date_mday));
        }
        Ok(format!(
            "{:04}-{:02}-{:02}",
            date_fullyear, date_month, date_mday
        ))
    }
    fn parse_full_time(&mut self) -> ParseResult<String> {
        let partial_time = self.parse_partial_time()?;
        let time_offset = self.parse_time_offset()?;
        Ok(format!("{}{}", partial_time, time_offset))
    }
    fn parse_offset_date_time(&mut self) -> ParseResult<String> {
        let full_date = self.parse_full_date()?;
        let time_delim = self.parse_time_delim()?;
        let full_time = self.parse_full_time()?;
        Ok(format!("{}{}{}", full_date, time_delim, full_time))
    }
    fn parse_local_date_time(&mut self) -> ParseResult<String> {
        let full_date = self.parse_full_date()?;
        let time_delim = self.parse_time_delim()?;
        let partial_time = self.parse_partial_time()?;
        Ok(format!("{}{}{}", full_date, time_delim, partial_time))
    }
    fn parse_local_date(&mut self) -> ParseResult<String> {
        self.parse_full_date()
    }
    fn parse_local_time(&mut self) -> ParseResult<String> {
        self.parse_partial_time()
    }

    // Arrays
    fn parse_array(&mut self) -> ParseResult<Value> {
        self.parse_array_open()?;
        let values = self.parse_array_values()?;
        self.parse_ws_comment_newline()?;
        self.parse_array_close()?;
        Ok(values)
    }
    fn parse_array_open(&mut self) -> ParseResult<()> {
        self.sc.expect('[')
    }
    fn parse_array_close(&mut self) -> ParseResult<()> {
        self.sc.expect(']')
    }
    fn parse_array_values(&mut self) -> ParseResult<Value> {
        let mut res = Vec::new();
        loop {
            if self.parse_ws_comment_newline().is_err() {
                break;
            }
            if self.sc.peek() == Some(']') {
                break;
            }
            res.push(self.parse_val()?);
            self.parse_ws_comment_newline()?;
            if !self.sc.eat(',') {
                break;
            }
        }
        Ok(Value::Array(res))
    }
    fn parse_ws_comment_newline(&mut self) -> ParseResult<()> {
        let mut mark: usize;
        loop {
            mark = self.sc.mark();
            self.parse_ws()?;
            if self.parse_comment().is_ok() {
                if self.parse_newline().is_ok() {
                    continue;
                }
                return Err("expected newline after comment".into());
            }
            if self.parse_newline().is_ok() {
                continue;
            }
            if mark == self.sc.pos() {
                break;
            }
        }
        Ok(())
    }

    // Tables
    fn parse_table(&mut self) -> ParseResult<Statement> {
        let mark = self.sc.mark();
        let std_table = self.parse_std_table();
        if std_table.is_ok() {
            return Ok(Statement::Table(std_table?));
        }
        self.sc.reset(mark);
        Ok(Statement::ArrayTable(self.parse_array_table()?))
    }
    fn parse_std_table(&mut self) -> ParseResult<Vec<String>> {
        self.parse_std_table_open()?;
        let key = self.parse_key()?;
        self.parse_std_table_close()?;
        Ok(key)
    }
    fn parse_std_table_open(&mut self) -> ParseResult<()> {
        if !self.sc.eat('[') {
            return Err("expected [".into());
        }
        self.parse_ws()
    }
    fn parse_std_table_close(&mut self) -> ParseResult<()> {
        self.parse_ws()?;
        if self.sc.eat(']') {
            return Ok(());
        }
        Err("expected ]".into())
    }
    fn parse_inline_table(&mut self) -> ParseResult<Value> {
        self.parse_inline_table_open()?;
        let keyvals = self.parse_inline_table_keyvals()?;
        self.parse_ws_comment_newline()?;
        self.parse_inline_table_close()?;
        let mut table = Vec::new();
        for (path, val) in keyvals {
            insert(&mut table, &path, val)?;
        }
        Ok(Value::Table(table))
    }
    fn parse_inline_table_open(&mut self) -> ParseResult<()> {
        self.sc.expect('{')
    }
    fn parse_inline_table_close(&mut self) -> ParseResult<()> {
        self.sc.expect('}')
    }
    fn parse_inline_table_keyvals(&mut self) -> ParseResult<Vec<(Vec<String>, Value)>> {
        let mut res = Vec::new();
        loop {
            if self.parse_ws_comment_newline().is_err() {
                break;
            }
            if self.sc.peek() == Some('}') {
                break;
            }
            res.push(self.parse_keyval()?);
            self.parse_ws_comment_newline()?;
            if !self.sc.eat(',') {
                break;
            }
        }
        Ok(res)
    }
    fn parse_array_table(&mut self) -> ParseResult<Vec<String>> {
        self.parse_array_table_open()?;
        let key = self.parse_key()?;
        self.parse_array_table_close()?;
        Ok(key)
    }
    fn parse_array_table_open(&mut self) -> ParseResult<()> {
        if !self.sc.eat_str("[[") {
            return Err("expected array-table open ([[)".into());
        }
        self.parse_ws()
    }
    fn parse_array_table_close(&mut self) -> ParseResult<()> {
        self.parse_ws()?;
        if self.sc.eat_str("]]") {
            return Ok(());
        }
        Err("expected array-table close".into())
    }

    // Character primitives
    fn parse_digit(&mut self) -> ParseResult<u8> {
        for d in '0'..='9' {
            if self.sc.eat(d) {
                return Ok(d as u8 - '0' as u8);
            }
        }
        Err(format!("expected digit, got {:?}", self.sc.peek()))
    }
    fn parse_hexdigit(&mut self) -> ParseResult<char> {
        for d in ('0'..='9').chain('A'..='F').chain('a'..='f') {
            if self.sc.eat(d) {
                return Ok(d);
            }
        }
        Err(format!("expected hexdigit, got {:?}", self.sc.peek()))
    }
}
