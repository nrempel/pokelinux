//! Native IMS protocol components recovered against original-code fixtures.
//! Installed native IMS registration/state integration remains unimplemented.

pub mod reader;
pub mod registration;
pub mod requests;

/// Little-endian scalar encoding used by the audited Shannon DataWriter.
/// Narrow writes retain low bits, including for negative Java int inputs.
#[derive(Default)]
pub struct DataWriter(Vec<u8>);
impl DataWriter {
    pub fn write_byte(&mut self, value: i32) {
        self.0.push(value.to_le_bytes()[0]);
    }
    pub fn write_short(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes()[..2]);
    }
    pub fn write_int(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

/// Encodes the original stack-start payload; does not validate modem policy.
/// The source writes SIM status then stack type as truncated bytes.
pub fn encode_stack_start(sim_status: i32, stack_type: i32) -> Vec<u8> {
    let mut writer = DataWriter::default();
    writer.write_byte(sim_status);
    writer.write_byte(stack_type);
    writer.into_bytes()
}

/// Encodes the original registration-state payload, not an IMS registration flow.
/// Valid state/RAT values and call sequencing require separate recovery.
pub fn encode_registration_state(state: i32, rat: i32) -> Vec<u8> {
    let mut writer = DataWriter::default();
    writer.write_int(state);
    writer.write_int(rat);
    writer.into_bytes()
}
