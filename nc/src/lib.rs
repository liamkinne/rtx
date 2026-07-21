#![cfg_attr(not(test), no_std)]

/// Ack response.
pub const ACK: &str = "ok\n";

#[derive(Debug, defmt::Format)]
pub enum MachineState {
    Idle,
    Run,
    Hold,
    Jog,
    Alarm,
    Door,
}

impl ufmt::uDebug for MachineState {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        match self {
            Self::Idle => f.write_str("Idle"),
            Self::Run => f.write_str("Run"),
            Self::Hold => f.write_str("Hold"),
            Self::Jog => f.write_str("Jog"),
            Self::Alarm => f.write_str("Alarm"),
            Self::Door => f.write_str("Door"),
        }
    }
}

#[derive(Debug, defmt::Format)]
pub struct StatusReport {
    pub state: MachineState,
    pub position: (f32, f32, f32, f32, f32, f32, f32),
}

impl ufmt::uDisplay for StatusReport {
    fn fmt<W>(&self, f: &mut ufmt::Formatter<'_, W>) -> Result<(), W::Error>
    where
        W: ufmt::uWrite + ?Sized,
    {
        let (x, y, z, a, b, c, u) = self.position;
        f.write_str("<")?;
        ufmt::uDebug::fmt(&self.state, f)?;
        f.write_str("|MPos:")?;
        ufmt::uwrite!(
            f,
            "{},{},{},{},{},{},{}",
            ufmt_float::uFmt_f32::Three(x),
            ufmt_float::uFmt_f32::Three(y),
            ufmt_float::uFmt_f32::Three(z),
            ufmt_float::uFmt_f32::Three(a),
            ufmt_float::uFmt_f32::Three(b),
            ufmt_float::uFmt_f32::Three(c),
            ufmt_float::uFmt_f32::Three(u),
        )?;
        f.write_str(">")
    }
}
