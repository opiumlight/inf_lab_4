use crate::scanner::Scanner;
use crate::value::Value;
use core::str::FromStr;

pub type ParseResult<T> = Result<T, String>;

pub struct TomlParser {
    sc: Scanner,
}

impl TomlParser {
    pub fn new(doc: &str) -> Self {
        Self { sc: Scanner::new(doc) }
    }

    // Document
    pub fn parse(&mut self) -> ParseResult<Vec<Value>> {
        let mut values: Vec<Value> = Vec::new();
        values.push(self.parse_expression()?);
        while self.parse_newline().is_ok() {
            let new_expr = self.parse_expression();
            if new_expr.is_err() {
                break;
            }
            values.push(new_expr?);
        }
        if self.sc.peek().is_some() {
            return Err("Found incomplete expression".to_string());
        }
        Ok(values)
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
    fn parse_expression(&mut self) -> ParseResult<Value> {
        self.parse_ws()?;
        let mark = self.sc.mark();
        let keyval = self.parse_keyval();
        if keyval.is_err() {
            self.sc.reset(mark);
            let table = self.parse_table();
            if table.is_err() {
                return Ok(Value::Empty(self.parse_comment().unwrap_or(())))
            }
            self.parse_ws()?;
            self.parse_comment().unwrap_or(());
            return Ok(table?);
        }
        self.parse_ws()?;
        self.parse_comment().unwrap_or(());
        Ok(Value::KeyValue(keyval?))
    }
    fn parse_ws(&mut self) -> ParseResult<()> {
        loop {
            if !(self.sc.eat('\t') ||self.sc.eat(' ')) {
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
    fn parse_keyval(&mut self) -> ParseResult<(String, Box<Value>)> {
        let key = self.parse_key()?;
        self.parse_keyval_sep()?;
        let val = self.parse_val()?;
        Ok((key, Box::new(val)))
    }
    fn parse_key(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        let dk = self.parse_dotted_key();
        if dk.is_err() {
            self.sc.reset(mark);
            return Ok(self.parse_simple_key()?)
        }
        Ok(dk?)
    }
    fn parse_val(&mut self) -> ParseResult<Value> {
        let mark = self.sc.mark();
        let st = self.parse_string();
        if st.is_ok() {return Ok(Value::Str(st?));}
        self.sc.reset(mark);
        let boolean = self.parse_boolean();
        if boolean.is_ok() {return Ok(Value::Bool(boolean?));}
        self.sc.reset(mark);
        let array = self.parse_array();
        if array.is_ok() {return Ok(array?);}
        self.sc.reset(mark);
        let inline_table = self.parse_inline_table();
        if inline_table.is_ok() {return Ok(inline_table?);}
        self.sc.reset(mark);
        let date_time = self.parse_date_time();
        if date_time.is_ok() {return Ok(date_time?);}
        self.sc.reset(mark);
        let float = self.parse_float();
        if float.is_ok() {return Ok(Value::Float(float?));}
        self.sc.reset(mark);
        let integer = self.parse_integer();
        if integer.is_ok() {return Ok(Value::Int(integer?));}
        self.sc.reset(mark);
        Err("No valid val found.".to_string())
    }
    fn parse_simple_key(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        let quoted = self.parse_quoted_key();
        if quoted.is_ok() { return Ok(quoted?); }
        self.sc.reset(mark);
        self.parse_unquoted_key()
    }
    fn parse_unquoted_key(&mut self) -> ParseResult<String> {
        let mut key = Vec::<char>::new();
        let alphas: Vec<char> = ('A'..='Z').chain('a'..='z').chain('0'..='9').collect();
        loop {
            let ch = self.sc.peek().unwrap_or('\\');
            if alphas.contains(&ch) || ch == '-' || ch == '_' {
                key.push(ch);
                self.sc.bump();
            } else {
                break;
            }
        }
        if key.len() == 0 {return Err("Empty key.".to_string());}
        Ok(key.iter().collect::<String>())
    }
    fn parse_quoted_key(&mut self) -> ParseResult<String> {
        let mark = self.sc.mark();
        let basic = self.parse_basic_string();
        if basic.is_ok() {return Ok(basic?);}
        self.sc.reset(mark);
        self.parse_literal_string()
    }
    fn parse_dotted_key(&mut self) -> ParseResult<String> {
        let mut key = self.parse_simple_key()?;
        loop {
            let sep = self.parse_dot_sep();
            if sep.is_err() {break;}
            let key2 = self.parse_simple_key()?;
            key += ".";
            key += &key2;
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
        if ml_basic_string.is_ok() {return Ok(ml_basic_string?);}
        self.sc.reset(mark);
        mark = self.sc.mark();
        let basic_string = self.parse_basic_string();
        if basic_string.is_ok() {return Ok(basic_string?);}
        self.sc.reset(mark);
        mark = self.sc.mark();
        let ml_literal_string = self.parse_ml_literal_string();
        if ml_literal_string.is_ok() {return Ok(ml_literal_string?);}
        self.sc.reset(mark);
        let literal_string = self.parse_literal_string();
        if literal_string.is_ok() {return Ok(literal_string?);}
        Err("No valid string found.".to_string())
    }
    fn parse_basic_string(&mut self) -> ParseResult<String> {
        self.sc.expect('"')?;
        let mut chars = Vec::<char>::new();
        loop {
            let ch = self.parse_basic_char();
            if ch.is_err() {break;}
            chars.push(ch?);
        }
        self.sc.expect('"')?;
        Ok(chars.iter().collect::<String>())
    }
    fn parse_basic_char(&mut self) -> ParseResult<char> {
        let unescaped = self.parse_basic_unescaped();
        if unescaped.is_ok() {return Ok(unescaped?);}
        self.parse_escaped()
    }
    fn parse_basic_unescaped(&mut self) -> ParseResult<char> {
        let ch = self.sc.peek().unwrap_or('\\');
        if ch == '\\' || ch == '"' {
            return Err("Expected unescaped basic char, got \\ or EOF".into());
        }
        self.sc.bump();
        Ok(ch)
    }
    // TODO
    fn parse_escaped(&mut self) -> ParseResult<char> {
        self.sc.expect('\\')?;
        let escaped = self.sc.bump().unwrap_or('_');
        if "\"\\bef\n\r\t".contains(escaped) {
            return Ok(escaped);
        } else if escaped == 'x' {
            return Ok(escaped);
            todo!()
        } else if escaped == 'u' {
            return Ok(escaped);
            todo!()
        } else if escaped == 'U' {
            return Ok(escaped);
            todo!()
        } else {
            // Пока что \ просто исчезает и оставляет символ, не проверял на стандарте
            Ok(escaped)
        }
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
        let mut mlb_content = Vec::<char>::new();
        loop {
            let ch = self.parse_mlb_content();
            if ch.is_ok() {break;}
            mlb_content.push(ch?);
        }
        let mut mlb_quotes_content = Vec::<char>::new();
        loop {
            if self.parse_mlb_quotes().is_err() {break;};
            let mut content = Vec::<char>::new();
            loop {
                let ch = self.parse_mlb_content();
                if ch.is_err() {break;}
                content.push(ch?);
            }
            if content.len() == 0 {return Err("At least 1 mlb-content required".into())}
            mlb_quotes_content.append(&mut content);
        }
        self.parse_mlb_quotes()?;
        mlb_content.append(&mut mlb_quotes_content);
        Ok(mlb_content.iter().collect::<String>())
    }
    fn parse_mlb_content(&mut self) -> ParseResult<char> {
        let basic_char = self.parse_basic_char();
        if basic_char.is_ok() {return Ok(basic_char?);}
        if self.parse_newline().is_ok() || self.parse_mlb_escaped_nl().is_ok() {return Ok('\n');}
        Err("No valid mlb content found.".to_string())
    }
    fn parse_mlb_quotes(&mut self) -> ParseResult<()> {
        self.sc.expect('"')?;
        Ok(self.sc.expect('"').unwrap_or(()))
    }
    fn parse_mlb_escaped_nl(&mut self) -> ParseResult<()> {
        self.sc.expect('\\')?;
        self.parse_ws()?;
        self.parse_newline()?;
        loop { if !"\t \r\n".contains(self.sc.peek().unwrap_or('_')) {break;} }
        Ok(())
    }

    // Literal strings
    fn parse_literal_string(&mut self) -> ParseResult<String> {
        self.sc.expect('\'')?;
        let mut chars = Vec::<char>::new();
        loop {
            let ch = self.parse_literal_char();
            if ch.is_err() {break;}
            chars.push(ch?);
        }
        self.sc.eat('\'');
        Ok(chars.iter().collect::<String>())
    }
    // TODO: сделать как в abnf
    fn parse_literal_char(&mut self) -> ParseResult<char> {
        let ch = self.sc.peek().unwrap_or('\'');
        if ch == '\'' || ch == '\n' {return Err("Not a literal char".into());}
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
        let mut mll_content = Vec::<char>::new();
        loop {
            let ch = self.parse_mll_content();
            if ch.is_ok() {break;}
            mll_content.push(ch?);
        }
        let mut mll_quotes_content = Vec::<char>::new();
        loop {
            if self.parse_mll_quotes().is_err() {break;};
            let mut content = Vec::<char>::new();
            loop {
                let ch = self.parse_mll_content();
                if ch.is_err() {break;}
                content.push(ch?);
            }
            if content.len() == 0 {return Err("At least 1 mll-content required".into())}
            mll_quotes_content.append(&mut content);
        }
        self.parse_mll_quotes()?;
        mll_content.append(&mut mll_quotes_content);
        Ok(mll_content.iter().collect::<String>())
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
    fn parse_mll_quotes(&mut self) -> ParseResult<()> {
        self.sc.expect('\'')?;
        self.sc.eat('\'');
        Ok(())
    }

    // Integers
    fn parse_integer(&mut self) -> ParseResult<i128> {
        let mark = self.sc.mark();
        let mut int = self.parse_hex_int();
        if int.is_ok() {return Ok(int?);}
        self.sc.reset(mark);
        int = self.parse_dec_int();
        if int.is_ok() {return Ok(int?);}
        self.sc.reset(mark);
        int = self.parse_oct_int();
        if int.is_ok() {return Ok(int?);}
        self.sc.reset(mark);
        int = self.parse_bin_int();
        if int.is_ok() {return Ok(int?);}
        self.sc.reset(mark);
        Err("No valid integer found".to_string())
    }
    fn parse_digit1_9(&mut self) -> ParseResult<char> {
        for d in '1'..='9' {
            if self.sc.eat(d) {
                return Ok(d);
            }
        }
        Err(format!("Expected digit from 1 to 9, got {:?}", self.sc.peek()))
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
        if self.sc.eat('0') {return Ok('0')}
        if self.sc.eat('1') {return Ok('1')}
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
        let sgn = if self.sc.eat('-')
        {-1} else if self.sc.eat('+') || self.sc.peek().is_some()
        {1} else {return Err("Unexpected EOF".into())};
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
                if underscore { return Err("Dec int cannot end with _".into());}
                break;
            }
            res *= 10;
            res += d? as u128;
        }
        Ok(res)
    }
    fn parse_hex_int(&mut self) -> ParseResult<i128> {
        self.parse_hex_prefix()?;
        let mut res = vec![self.parse_hexdigit()?];
        loop {
            let hd = self.parse_hexdigit();
            if self.sc.eat('_') && hd.is_err() {
                return Err("Expected hex digit after _".to_string());
            }
            if hd.is_ok() {
                res.push(hd?);
                continue;
            }
            break;
        }
        Ok(i128::from_str_radix(res.iter().collect::<String>().as_str(), 16).unwrap())
    }
    fn parse_oct_int(&mut self) -> ParseResult<i128> {
        self.parse_oct_prefix()?;
        let mut res = vec![self.parse_digit0_7()?];
        loop {
            let hd = self.parse_digit0_7();
            if self.sc.eat('_') && hd.is_err() {
                return Err("Expected oct digit after _".to_string());
            }
            if hd.is_ok() {
                res.push(hd?);
                continue;
            }
            break;
        }
        Ok(i128::from_str_radix(res.iter().collect::<String>().as_str(), 16).unwrap())
    }
    fn parse_bin_int(&mut self) -> ParseResult<i128> {
        self.parse_bin_prefix()?;
        let mut res = vec![self.parse_digit0_1()?];
        loop {
            let hd = self.parse_digit0_1();
            if self.sc.eat('_') && hd.is_err() {
                return Err("Expected bin digit after _".to_string());
            }
            if hd.is_ok() {
                res.push(hd?);
                continue;
            }
            break;
        }
        Ok(i128::from_str_radix(res.iter().collect::<String>().as_str(), 16).unwrap())
    }

    // Floats
    fn parse_float(&mut self) -> ParseResult<f64> {
        if let Ok(sf) = self.parse_special_float() {
            let sgn = if sf.starts_with("-") {-1} else {1};
            return if sf.contains("inf") {
                if sgn == -1 { Ok(f64::NEG_INFINITY) } else { Ok(f64::INFINITY) }
            } else if sf.contains("nan") {
                Ok(f64::NAN)
            } else { // ну а вдруг?
                Err("something surely isn't good. (PARSE_FLOAT)".into())
            }
        }
        let int_part = self.parse_float_int_part()?;
        let exp = self.parse_exp();
        // Это место может паниковать. let it happen.
        match exp {
            Ok(exp) => {
                Ok(f64::from_str(format!("{int_part}{exp}").as_str()).unwrap())
            }
            Err(_) => {
                let frac = self.parse_frac()?;
                let exp = self.parse_exp()?;
                Ok(f64::from_str(format!("{int_part}{frac}{exp}").as_str()).unwrap())
            }
        }
    }
    fn parse_float_int_part(&mut self) -> ParseResult<String> {
        let val = self.parse_dec_int()?;
        Ok(val.to_string())
    }
    fn parse_frac(&mut self) -> ParseResult<String> {
        self.parse_decimal_point()?;
        self.parse_zero_prefixable_int()
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
                res += format!("_{}", new_digit?).as_str();
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
        self.sc.expect('e')?;
        Ok(format!("e{}", self.parse_float_exp_part()?))
    }
    fn parse_float_exp_part(&mut self) -> ParseResult<String> {
        let res: String;
        match self.sc.bump() {
            Some(c) if '-' == c => {res = "-".into()},
            Some(c) if '+' == c => {res = "+".into()},
            _ => {res = "".into()},
        }
        Ok(res + self.parse_zero_prefixable_int()?.as_ref())
    }
    fn parse_special_float(&mut self) -> ParseResult<String> {
        match self.sc.bump() {
            Some(c) if '-' == c || '+' == c => {
                if self.sc.eat_str("inf") {
                    return Ok(format!("{c}inf"))
                }
                if self.sc.eat_str("nan") {
                    return Ok(format!("{c}nan"))
                }
                Err("Incorrect special float".into())
            }
            Some(c) => Err(format!("Invalid special float sign: {}", c)),
            None => Err("Unexpected EOF".into()),
        }
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
        if self.sc.eat_str("true") { Ok(()) } else { Err("expected true".to_string()) }
    }
    fn parse_false(&mut self) -> ParseResult<()> {
        if self.sc.eat_str("false") { Ok(()) } else { Err("expected false".to_string()) }
    }

    // Date and time
    fn parse_date_time(&mut self) -> ParseResult<Value> {
        if let Ok(t) = self.parse_offset_date_time() {
            return Ok(Value::Str(t));
        }
        if let Ok(t) = self.parse_local_date_time() {
            return Ok(Value::Str(t));
        }
        if let Ok(t) = self.parse_local_date() {
            return Ok(Value::Str(t));
        }
        if let Ok(t) = self.parse_local_time() {
            return Ok(Value::Str(t));
        }
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
        if val < 13 {
            return Ok(val);
        }
        Err("mday should be between 0 and 12 inclusive".into())
    }
    /// NOTE: МЕТОД НЕ УЧИТЫВАЕТ ОСОБЕННОСТИ ОТДЕЛЬНОГО МЕСЯЦА
    fn parse_date_mday(&mut self) -> ParseResult<u8> {
        let val = self.parse_digit()? * 10u8 + self.parse_digit()?;
        if val < 32 {
            return Ok(val);
        }
        Err("mday should be between 0 and 31 inclusive".into())
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
        self.sc.expect('.')?;
        let mut res = vec!['.'];
        while let Ok(c) = self.parse_digit() {
            res.push((c - '0' as u8) as char);
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
            return Err("expected -/+ at numoffset beginning".into())
        }
        let time_hour = self.parse_time_hour()?;
        let time_minute = self.parse_time_minute()?;
        Ok(format!("{}{}:{}", sign, time_hour, time_minute))
    }
    fn parse_time_offset(&mut self) -> ParseResult<String> {
        if self.sc.eat('Z') {
            return Ok("Z".into());
        }
        self.parse_time_numoffset()
    }
    fn parse_partial_time(&mut self) -> ParseResult<String> {
        let time_hour = self.parse_time_hour()?;
        let time_minute = self.parse_time_minute()?;
        let mut res = format!("{}:{}", time_hour, time_minute);
        if let Ok(second) = self.parse_time_second() {
            res += format!(":{}", second).as_ref();
            if let Ok(secfrac) = self.parse_time_secfrac() {
                res += format!("{}", secfrac).as_ref();
            }
        }
        Ok(res)
    }
    fn parse_full_date(&mut self) -> ParseResult<String> {
        let date_fullyear = self.parse_date_fullyear()?;
        let date_month = self.parse_date_month()?;
        let date_mday = self.parse_date_mday()?;
        Ok(format!("{}-{}-{}", date_fullyear, date_month, date_mday))
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
            if self.parse_ws_comment_newline().is_err() {break;}
            res.push(self.parse_val()?);
            self.parse_ws_comment_newline()?;
            if !self.sc.eat(',') {break;}
        }
        Ok(Value::Array(res))
    }
    fn parse_ws_comment_newline(&mut self) -> ParseResult<()> {
        let mut mark: usize;
        loop {
            mark = self.sc.mark();
            self.parse_ws()?;
            if mark == self.sc.pos() {
                break;
            } else {
                if self.parse_comment().is_ok() {
                    if self.parse_newline().is_ok() {
                        continue;
                    }
                    return Err("expected newline after comment".into());
                }
                if self.parse_newline().is_ok() {
                    continue;
                }
                break;
            }
        }
        Ok(())
    }


    // Tables
    fn parse_table(&mut self) -> ParseResult<Value> {
        let mark = self.sc.mark();
        let std_table = self.parse_std_table();
        if std_table.is_ok() {return Ok(Value::Str(std_table?));}
        self.sc.reset(mark);
        Ok(Value::Str(self.parse_array_table()?))
    }
    fn parse_std_table(&mut self) -> ParseResult<String> {
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
        Ok(Value::Table(keyvals))
    }
    fn parse_inline_table_open(&mut self) -> ParseResult<()> {
        self.sc.expect('{')
    }
    fn parse_inline_table_close(&mut self) -> ParseResult<()> {
        self.sc.expect('}')
    }
    fn parse_inline_table_keyvals(&mut self) -> ParseResult<Vec<(String, Box<Value>)>> {
        let mut res = Vec::new();
        loop {
            if self.parse_ws_comment_newline().is_err() {break;}
            res.push(self.parse_keyval()?);
            self.parse_ws_comment_newline()?;
            if !self.sc.eat(',') {break;}
        }
        Ok(res)
    }
    fn parse_array_table(&mut self) -> ParseResult<String> {
        self.parse_array_table_open()?;
        let key = self.parse_key()?;
        self.parse_array_table_close()?;

        Ok(key)
    }
    fn parse_array_table_open(&mut self) -> ParseResult<()> {
        if !self.sc.eat_str("[[") {
            return Err("expected array-table open ([[)".into())
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
    fn parse_alpha(&mut self) -> ParseResult<char> {
        for a in ('a'..='z').chain('A'..='Z') {
            if self.sc.eat(a) {
                return Ok(a);
            }
        }
        Err(format!("expected alpha, got {:?}", self.sc.peek()))
    }
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