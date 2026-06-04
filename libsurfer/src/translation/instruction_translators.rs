use super::{TranslationPreference, ValueKind, check_single_wordlength};
use crate::wave_container::{ScopeId, VarId, VariableMeta};

use eyre::Result;
use instruction_decoder::{Decoder, specs};
use surfer_translation_types::{BasicTranslator, VariableValue, check_vector_variable};
use toml::Value;

pub struct InstructionTranslator {
    pub name: String,
    pub decoder: Decoder,
    pub num_bits: u32,
}

impl BasicTranslator<VarId, ScopeId> for InstructionTranslator {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn basic_translate(&self, num_bits: u32, value: &VariableValue) -> (String, ValueKind) {
        let u64_value = match value {
            VariableValue::BigUint(v) => v.to_u64_digits().last().copied(),
            VariableValue::String(s) => match check_vector_variable(s) {
                Some(v) => return v,
                None => u64::from_str_radix(s, 2).ok(),
            },
        }
        .unwrap_or(0);

        match self
            .decoder
            .decode_from_i64(u64_value as i64, num_bits as usize)
        {
            Ok(iform) => (iform, ValueKind::Normal),
            _ => (
                format!(
                    "UNKNOWN INSN ({:#0width$x})",
                    u64_value,
                    width = num_bits.div_ceil(4) as usize + 2
                ),
                ValueKind::Warn,
            ),
        }
    }

    fn translates(&self, variable: &VariableMeta) -> Result<TranslationPreference> {
        check_single_wordlength(variable.num_bits, self.num_bits)
    }
}

const INTEGER_REGISTER_NAMES: [&str; 32] = [
    "x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8", "x9", "x10", "x11", "x12", "x13", "x14",
    "x15", "x16", "x17", "x18", "x19", "x20", "x21", "x22", "x23", "x24", "x25", "x26", "x27",
    "x28", "x29", "x30", "x31",
];

const FLOAT_REGISTER_NAMES: [&str; 32] = [
    "f0", "f1", "f2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", "f10", "f11", "f12", "f13", "f14",
    "f15", "f16", "f17", "f18", "f19", "f20", "f21", "f22", "f23", "f24", "f25", "f26", "f27",
    "f28", "f29", "f30", "f31",
];

fn numeric_register_spec(spec: &str) -> toml::Table {
    let mut table = spec
        .parse::<toml::Table>()
        .expect("Can't parse bundled RV32 decoder spec");
    let mappings = table
        .get_mut("mappings")
        .and_then(Value::as_table_mut)
        .expect("Bundled RV32 decoder spec is missing mappings");

    mappings.insert(
        "Register_int".to_owned(),
        Value::Array(
            INTEGER_REGISTER_NAMES
                .iter()
                .map(|name| Value::String((*name).to_owned()))
                .collect(),
        ),
    );
    mappings.insert(
        "Register_float".to_owned(),
        Value::Array(
            FLOAT_REGISTER_NAMES
                .iter()
                .map(|name| Value::String((*name).to_owned()))
                .collect(),
        ),
    );

    table
}

fn rv32_specs() -> Vec<String> {
    specs::rv32::RV32
        .iter()
        .map(|spec| (*spec).to_owned())
        .collect()
}

fn rv32_numeric_register_specs() -> Vec<toml::Table> {
    specs::rv32::RV32
        .iter()
        .map(|spec| numeric_register_spec(spec))
        .collect()
}

#[must_use]
pub fn new_rv32_translator() -> InstructionTranslator {
    InstructionTranslator {
        name: "RV32".into(),
        decoder: Decoder::new(&rv32_specs()).expect("Can't build RV32 decoder"),
        num_bits: 32,
    }
}

#[must_use]
pub fn new_rv32_reg_translator() -> InstructionTranslator {
    InstructionTranslator {
        name: "RV32-Reg".into(),
        decoder: Decoder::new_from_table(rv32_numeric_register_specs())
            .expect("Can't build RV32-Reg decoder"),
        num_bits: 32,
    }
}

#[must_use]
pub fn new_rv64_translator() -> InstructionTranslator {
    InstructionTranslator {
        name: "RV64".into(),
        decoder: Decoder::new(
            &specs::rv64::RV64
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
        )
        .expect("Can't build RV64 decoder"),
        num_bits: 32,
    }
}

#[must_use]
pub fn new_mips_translator() -> InstructionTranslator {
    InstructionTranslator {
        name: "MIPS".into(),
        decoder: Decoder::new(&[specs::MIPS.to_owned()]).expect("Can't build mips decoder"),
        num_bits: 32,
    }
}

#[must_use]
pub fn new_la64_translator() -> InstructionTranslator {
    InstructionTranslator {
        name: "LA64".into(),
        decoder: Decoder::new(&[specs::LA64.to_owned()]).expect("Can't build LA64 decoder"),
        num_bits: 32,
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn riscv_from_bigunit() {
        let rv32_translator = new_rv32_translator();
        let rv32_reg_translator = new_rv32_reg_translator();
        let rv64_translator = new_rv64_translator();
        assert_eq!(
            rv32_translator
                .basic_translate(32, &VariableValue::BigUint(1u32.into()))
                .0,
            "c.nop"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(32, &VariableValue::BigUint(0b1000000010011111u32.into()))
                .0,
            "UNKNOWN INSN (0x0000809f)"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(
                    32,
                    &VariableValue::BigUint(0b1000_0001_0011_0101_0000_0101_1001_0011_u32.into())
                )
                .0,
            "addi a1, a0, -2029"
        );
        assert_eq!(
            rv32_reg_translator
                .basic_translate(
                    32,
                    &VariableValue::BigUint(0b1000_0001_0011_0101_0000_0101_1001_0011_u32.into())
                )
                .0,
            "addi x11, x10, -2029"
        );
        assert_eq!(
            rv64_translator
                .basic_translate(32, &VariableValue::BigUint(1u32.into()))
                .0,
            "c.nop"
        );
        assert_eq!(
            rv64_translator
                .basic_translate(32, &VariableValue::BigUint(0b1000000010011111u32.into()))
                .0,
            "UNKNOWN INSN (0x0000809f)"
        );
        assert_eq!(
            rv64_translator
                .basic_translate(
                    32,
                    &VariableValue::BigUint(0b1000_0001_0011_0101_0000_0101_1001_0011_u32.into())
                )
                .0,
            "addi a1, a0, -2029"
        );
    }
    #[test]
    fn riscv_from_string() {
        let rv32_translator = new_rv32_translator();
        let rv32_reg_translator = new_rv32_reg_translator();
        assert_eq!(
            rv32_translator
                .basic_translate(32, &VariableValue::String("1".to_owned()))
                .0,
            "c.nop"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01001000100010001000100011111111".to_owned())
                )
                .0,
            "UNKNOWN INSN (0x488888ff)"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01xzz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "UNDEF"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(
                    32,
                    &VariableValue::String("010zz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "HIGHIMP"
        );
        assert_eq!(
            rv32_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01011-hlw0010001000100010001000".to_owned())
                )
                .0,
            "DON'T CARE"
        );
        assert_eq!(
            rv32_reg_translator
                .basic_translate(
                    32,
                    &VariableValue::String("10000001001101010000010110010011".to_owned())
                )
                .0,
            "addi x11, x10, -2029"
        );
    }

    #[test]
    fn mips_from_bigunit() {
        let mips_translator = new_mips_translator();
        assert_eq!(
            mips_translator
                .basic_translate(32, &VariableValue::BigUint(0x3a873u32.into()))
                .0,
            "UNKNOWN INSN (0x0003a873)"
        );
        assert_eq!(
            mips_translator
                .basic_translate(32, &VariableValue::BigUint(0x24210000u32.into()))
                .0,
            "addiu $at, $at, 0"
        );
    }

    #[test]
    fn mips_from_string() {
        let mips_translator = new_mips_translator();
        assert_eq!(
            mips_translator
                .basic_translate(
                    32,
                    &VariableValue::String("10101111110000010000000000000000".to_owned())
                )
                .0,
            "sw $at, 0($fp)"
        );
        assert_eq!(
            mips_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01xzz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "UNDEF"
        );
        assert_eq!(
            mips_translator
                .basic_translate(
                    32,
                    &VariableValue::String("010zz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "HIGHIMP"
        );
        assert_eq!(
            mips_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01011-hlw0010001000100010001000".to_owned())
                )
                .0,
            "DON'T CARE"
        );
    }

    #[test]
    fn la64_from_bigunit() {
        let la64_translator = new_la64_translator();
        assert_eq!(
            la64_translator
                .basic_translate(32, &VariableValue::BigUint(0xffffffffu32.into()))
                .0,
            "UNKNOWN INSN (0xffffffff)"
        );
        assert_eq!(
            la64_translator
                .basic_translate(32, &VariableValue::BigUint(0x1a000004u32.into()))
                .0,
            "pcalau12i $a0, 0"
        );
    }

    #[test]
    fn la64_from_string() {
        let la64_translator = new_la64_translator();
        assert_eq!(
            la64_translator
                .basic_translate(
                    32,
                    &VariableValue::String("00101001101111111011001011001100".to_owned())
                )
                .0,
            "st.w $t0, $fp, -20"
        );
        assert_eq!(
            la64_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01xzz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "UNDEF"
        );
        assert_eq!(
            la64_translator
                .basic_translate(
                    32,
                    &VariableValue::String("010zz-hlw0010001000100010001000".to_owned())
                )
                .0,
            "HIGHIMP"
        );
        assert_eq!(
            la64_translator
                .basic_translate(
                    32,
                    &VariableValue::String("01011-hlw0010001000100010001000".to_owned())
                )
                .0,
            "DON'T CARE"
        );
    }
}
