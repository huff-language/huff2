use alloy_primitives::{hex, U256};

#[derive(Debug, Clone)]
pub struct ConstantOverride {
    pub name: String,
    pub value: U256,
}

pub fn parse_constant_override(s: &str) -> Result<ConstantOverride, String> {
    let (name, value) = s.split_once('=').ok_or_else(|| {
        format!(
            "Expected constant override in form \"<NAME>=<VALUE>\", got: {:?}",
            s
        )
    })?;

    let value = if value.starts_with("0x") {
        let bytes = hex::decode(value)
            .map_err(|_| format!("The value {:?} is not a valie hex value", value))?;
        if bytes.len() > U256::BYTES {
            return Err(format!(
                "Constant {} larger than {}-bytes",
                value,
                U256::BYTES
            ));
        }
        U256::from_be_slice(&bytes)
    } else {
        U256::from_str_radix(value, 10).map_err(|_| {
            format!(
                "Expected value to be hex 0x... or decimal, got: {:?}",
                value
            )
        })?
    };

    Ok(ConstantOverride {
        name: name.to_string(),
        value,
    })
}
