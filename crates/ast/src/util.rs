use crate::ast::{SolError, SolFunction};
use crate::Spanned;
use alloy_dyn_abi::DynSolType;
use alloy_primitives::{keccak256, FixedBytes, U256};
use evm_glue::opcodes::Opcode;

pub(crate) fn u256_as_push_data<const N: usize>(value: U256) -> Result<[u8; N], String> {
    if value.byte_len() > N {
        return Err(format!(
            "word with {} bytes is too large for PUSH{}",
            value.byte_len(),
            N
        ));
    }
    let input = value.to_be_bytes::<32>();
    let mut output = [0u8; N];
    output.copy_from_slice(&input[32 - N..32]);

    Ok(output)
}

pub fn u256_as_push(value: U256) -> Opcode {
    match value.byte_len() {
        0..=1 => u256_as_push_data::<1>(value).map(Opcode::PUSH1).unwrap(),
        2 => u256_as_push_data::<2>(value).map(Opcode::PUSH2).unwrap(),
        3 => u256_as_push_data::<3>(value).map(Opcode::PUSH3).unwrap(),
        4 => u256_as_push_data::<4>(value).map(Opcode::PUSH4).unwrap(),
        5 => u256_as_push_data::<5>(value).map(Opcode::PUSH5).unwrap(),
        6 => u256_as_push_data::<6>(value).map(Opcode::PUSH6).unwrap(),
        7 => u256_as_push_data::<7>(value).map(Opcode::PUSH7).unwrap(),
        8 => u256_as_push_data::<8>(value).map(Opcode::PUSH8).unwrap(),
        9 => u256_as_push_data::<9>(value).map(Opcode::PUSH9).unwrap(),
        10 => u256_as_push_data::<10>(value).map(Opcode::PUSH10).unwrap(),
        11 => u256_as_push_data::<11>(value).map(Opcode::PUSH11).unwrap(),
        12 => u256_as_push_data::<12>(value).map(Opcode::PUSH12).unwrap(),
        13 => u256_as_push_data::<13>(value).map(Opcode::PUSH13).unwrap(),
        14 => u256_as_push_data::<14>(value).map(Opcode::PUSH14).unwrap(),
        15 => u256_as_push_data::<15>(value).map(Opcode::PUSH15).unwrap(),
        16 => u256_as_push_data::<16>(value).map(Opcode::PUSH16).unwrap(),
        17 => u256_as_push_data::<17>(value).map(Opcode::PUSH17).unwrap(),
        18 => u256_as_push_data::<18>(value).map(Opcode::PUSH18).unwrap(),
        19 => u256_as_push_data::<19>(value).map(Opcode::PUSH19).unwrap(),
        20 => u256_as_push_data::<20>(value).map(Opcode::PUSH20).unwrap(),
        21 => u256_as_push_data::<21>(value).map(Opcode::PUSH21).unwrap(),
        22 => u256_as_push_data::<22>(value).map(Opcode::PUSH22).unwrap(),
        23 => u256_as_push_data::<23>(value).map(Opcode::PUSH23).unwrap(),
        24 => u256_as_push_data::<24>(value).map(Opcode::PUSH24).unwrap(),
        25 => u256_as_push_data::<25>(value).map(Opcode::PUSH25).unwrap(),
        26 => u256_as_push_data::<26>(value).map(Opcode::PUSH26).unwrap(),
        27 => u256_as_push_data::<27>(value).map(Opcode::PUSH27).unwrap(),
        28 => u256_as_push_data::<28>(value).map(Opcode::PUSH28).unwrap(),
        29 => u256_as_push_data::<29>(value).map(Opcode::PUSH29).unwrap(),
        30 => u256_as_push_data::<30>(value).map(Opcode::PUSH30).unwrap(),
        31 => u256_as_push_data::<31>(value).map(Opcode::PUSH31).unwrap(),
        32 => u256_as_push_data::<32>(value).map(Opcode::PUSH32).unwrap(),
        _ => unreachable!(),
    }
}

type Selector = (FixedBytes<4>, FixedBytes<4>);

pub fn compute_selector(
    func: SolFunction,
    err: SolError,
) -> Option<(FixedBytes<4>, FixedBytes<4>)> {
    let build_signature = |name: &Spanned<&str>, args: &Box<[Spanned<DynSolType>]>| -> Vec<u8> {
        let arg_types: Vec<_> = args
            .iter()
            .map(|arg| match arg.0 {
                DynSolType::Address => "address",
                DynSolType::Uint(_) => "uint256",
                DynSolType::String => "string",
                _ => panic!("Unsupported type: {:?}", arg.0),
            })
            .collect();

        let mut signature = String::new();
        signature.push_str(name.0);
        signature.push('(');
        signature.push_str(&arg_types.join(","));
        signature.push(')');
        signature.into_bytes()
    };

    let build_selector =
        |name: &Spanned<&str>, args: &Box<[Spanned<DynSolType>]>| -> FixedBytes<4> {
            let signature = build_signature(name, args);
            let hash = keccak256(signature);
            FixedBytes::<4>::from_slice(&hash[..4])
        };

    let func_selector = build_selector(&func.name, &func.args);
    let err_selector = build_selector(&err.name, &err.args);

    Some((func_selector, err_selector))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use alloy_dyn_abi::DynSolType;
    use alloy_primitives::keccak256;
    use chumsky::span::Span;

    #[test]
    fn test_compute_selector() {
        let func = SolFunction {
            name: Spanned::new("transfer", 0..8),
            args: Box::new([
                Spanned::new(DynSolType::Address, 9..17),
                Spanned::new(DynSolType::Uint(256), 18..26),
            ]),
            rets: Box::new([]),
        };
        let err = SolError {
            name: Spanned::new("TransferFailed", 0..15),
            args: Box::new([
                Spanned::new(DynSolType::String, 16..21),
                Spanned::new(DynSolType::Uint(256), 22..30),
            ]),
        };
        let selectors = compute_selector(func.clone(), err.clone()).unwrap();
        let expected_func_hash = keccak256(selectors.0.clone());
        let expected_err_hash = keccak256(selectors.1.clone());
        //println!("{}", &selectors);
        let expected_func_selector: Selector = (
            FixedBytes::from_slice(&expected_func_hash[..4]),
            FixedBytes::from_slice(&expected_err_hash[..4]),
        );
        assert_eq!(selectors.0, expected_func_selector.0);
        assert_eq!(selectors.1, expected_func_selector.1);
    }
}
