use boltffi::*;

use crate::records::blittable::Point;

#[export]
pub fn safe_divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("division by zero".to_string())
    } else {
        Ok(a / b)
    }
}

#[export]
pub fn safe_sqrt(x: f64) -> Result<f64, String> {
    if x < 0.0 {
        Err("negative input".to_string())
    } else {
        Ok(x.sqrt())
    }
}

#[export]
pub fn parse_point(s: String) -> Result<Point, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err("expected format: x,y".to_string());
    }
    let x = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| "invalid x coordinate".to_string())?;
    let y = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| "invalid y coordinate".to_string())?;
    Ok(Point { x, y })
}

#[export]
pub fn always_ok(v: i32) -> Result<i32, String> {
    Ok(v * 2)
}

#[export]
pub fn always_err(msg: String) -> Result<i32, String> {
    Err(msg)
}

#[export]
pub fn result_to_string(v: Result<i32, String>) -> String {
    match v {
        Ok(val) => format!("ok: {}", val),
        Err(err) => format!("err: {}", err),
    }
}

#[export]
pub fn divide(a: i32, b: i32) -> Result<i32, String> {
    safe_divide(a, b)
}

#[export]
pub fn parse_int(input: String) -> Result<i32, String> {
    input
        .parse::<i32>()
        .map_err(|_| "invalid integer".to_string())
}

#[export]
pub fn validate_name(name: String) -> Result<String, String> {
    if name.is_empty() {
        Err("name cannot be empty".to_string())
    } else if name.len() > 100 {
        Err("name too long".to_string())
    } else {
        Ok(format!("Hello, {}!", name))
    }
}

#[cfg(test)]
mod tests {
    use boltffi::__private::wire;

    #[test]
    fn exported_result_string_parameter_round_trips() {
        let input_bytes = wire::encode(&Err::<i32, String>("bad".to_owned()));
        let output_buffer =
            unsafe { super::boltffi_result_to_string(input_bytes.as_ptr(), input_bytes.len()) };
        let output_bytes = unsafe { output_buffer.as_byte_slice() }.to_vec();
        drop(output_buffer);

        let output_string: String =
            wire::decode(&output_bytes).expect("exported result_to_string should decode");

        assert_eq!(output_string, "err: bad");
    }
}
