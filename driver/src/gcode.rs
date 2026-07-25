use crate::app::gcode;
use crate::axes::*;
use ::gcode::core::BlockVisitor;
use ::gcode::core::CommandVisitor;
use ::gcode::core::ControlFlow;
use ::gcode::core::Diagnostics;
use ::gcode::core::HasDiagnostics;
use ::gcode::core::Number;
use ::gcode::core::ProgramVisitor;
use ::gcode::core::Value;
use embassy_stm32::peripherals::USB;
use embassy_stm32::usb::Driver;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::driver::EndpointError;
use nc::ACK;
use nc::lines::LineReader;

const CONFIG: &str = r#"board: RTX driver
name: UMI RTX
meta: Robot Arm
axes:
  x:
    max_travel_mm: 180.0
  y:
    max_travel_mm: 180.0
  z:
    max_travel_mm: 180.0
  a:
    max_travel_mm: 180.0
  b:
    max_travel_mm: 180.0
  c:
    max_travel_mm: 180.0
  d:
    max_travel_mm: 180.0
"#;

pub async fn gcode(cx: gcode::Context<'_>) {
    let usb = cx.local.usb_class;
    let mut lines = LineReader::<1024>::new();

    let chunk_send = async |usb: &mut CdcAcmClass<'static, Driver<'static, USB>>, data: &str| {
        let bytes = data.as_bytes();
        for chunk in bytes.chunks(usb.max_packet_size() as usize) {
            if let Err(err) = usb.write_packet(chunk).await {
                defmt::error!("Failed to write status report: {}", err);
                break;
            }
        }
    };

    loop {
        usb.wait_connection().await;

        let frame = match usb.read_packet(cx.local.packet).await {
            Ok(n) => &cx.local.packet[..n],
            Err(EndpointError::Disabled) => continue,
            Err(EndpointError::BufferOverflow) => {
                defmt::error!("read packet buffer overflow");
                continue;
            }
        };

        if frame == nc::STATUS_REPORT {
            let zed = ZED.from_counts(cx.shared.motor_zed.position());
            let shoulder = SHOULDER.from_counts(cx.shared.motor_shoulder.position());
            let elbow = ELBOW.from_counts(cx.shared.motor_elbow.position());
            let yaw = YAW.from_counts(cx.shared.motor_yaw.position());
            let w1 = cx.shared.motor_wrist_1.position();
            let w2 = cx.shared.motor_wrist_2.position();
            let wrist_pitch = WRIST_PITCH.from_counts(w1 + w2);
            let wrist_roll = WRIST_ROLL.from_counts(w1 - w2);
            let grip = GRIP.from_counts(cx.shared.motor_grip.position());

            let report = nc::StatusReport {
                state: nc::MachineState::Idle,
                position: (zed, shoulder, elbow, yaw, wrist_pitch, wrist_roll, grip),
            };
            let mut status = heapless::String::<128>::new();
            ufmt::uwriteln!(status, "{}", report).unwrap();
            chunk_send(usb, &status).await;
            continue;
        }

        let line = match lines.feed(frame) {
            Ok(Some(l)) => l,
            Ok(None) => continue,
            Err(err) => {
                defmt::error!("{}", err);
                continue;
            }
        };

        let string = match core::str::from_utf8(line) {
            Ok(s) => s,
            Err(err) => {
                defmt::error!("UTF-8 error: {}", defmt::Display2Format(&err));
                continue;
            }
        };

        defmt::info!("Got line: {}", string);

        match string {
            "" => {
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::BUILD_INFO => {
                usb.write_packet("[VER:3.4 FluidNC v3.3.0:]\r\n".as_bytes())
                    .await
                    .unwrap();
                usb.write_packet("[OPT:N,1,1024]\r\n".as_bytes())
                    .await
                    .unwrap();
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::ERRORS_LIST => {
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::ALARMS_LIST => {
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::STARTUP_SHOW => {
                usb.write_packet("[MSG:INFO: UMI RTX]\r\n".as_bytes())
                    .await
                    .unwrap();
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::GCODE_MODES => {
                usb.write_packet("[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]\n".as_bytes())
                    .await
                    .unwrap();
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::VERBOSE_ERRORS => {
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            nc::CONFIG_DUMP => {
                chunk_send(usb, CONFIG).await;
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
            line => {
                let gcode_str = line.strip_prefix(nc::JOG_PREFIX).unwrap_or(line);

                let mut visitor = GcodeVisitor {
                    diagnostics: DefmtDiagnostics,
                    pending_move: PendingMove::default(),
                    incremental: false,
                };
                ::gcode::core::parse(gcode_str, &mut visitor);

                defmt::info!("Pending move: {}", visitor.pending_move);
                usb.write_packet(ACK.as_bytes()).await.unwrap();
            }
        }
    }
}

struct DefmtDiagnostics;

impl Diagnostics for DefmtDiagnostics {
    fn emit_unknown_content(&mut self, text: &str, span: ::gcode::core::Span) {
        defmt::warn!("Line {} unknown content: {}", span.line, text);
    }

    fn emit_unexpected(
        &mut self,
        actual: &str,
        expected: &[::gcode::core::TokenType],
        span: ::gcode::core::Span,
    ) {
        defmt::warn!(
            "Line {} expected {}, got {}",
            span.line,
            defmt::Debug2Format(&expected),
            actual
        );
    }

    fn emit_parse_int_error(
        &mut self,
        value: &str,
        error: core::num::ParseIntError,
        span: ::gcode::core::Span,
    ) {
        defmt::warn!(
            "Line {} failed to parse int {} with error {}",
            span.line,
            value,
            defmt::Debug2Format(&error)
        )
    }
}

#[derive(Debug, Default, defmt::Format)]
struct PendingMove {
    pending_move_zed: Option<f32>,
    pending_move_shoulder: Option<f32>,
    pending_move_elbow: Option<f32>,
    pending_move_yaw: Option<f32>,
    pending_move_wrist_pitch: Option<f32>,
    pending_move_wrist_roll: Option<f32>,
    pending_move_grip: Option<f32>,
}

struct GcodeVisitor {
    diagnostics: DefmtDiagnostics,
    pending_move: PendingMove,
    /// True when the most recently seen distance-mode command is G91 (incremental).
    incremental: bool,
}

impl HasDiagnostics for GcodeVisitor {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        &mut self.diagnostics
    }
}

impl ProgramVisitor for GcodeVisitor {
    fn start_block(&mut self) -> ControlFlow<impl BlockVisitor + '_> {
        ControlFlow::Continue(GcodeBlock {
            diagnostics: &mut self.diagnostics,
            pending_move: &mut self.pending_move,
            incremental: &mut self.incremental,
        })
    }
}

struct GcodeBlock<'a> {
    diagnostics: &'a mut DefmtDiagnostics,
    pending_move: &'a mut PendingMove,
    incremental: &'a mut bool,
}

impl HasDiagnostics for GcodeBlock<'_> {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        self.diagnostics
    }
}

impl BlockVisitor for GcodeBlock<'_> {
    fn line_number(&mut self, n: u32, _span: ::gcode::core::Span) {
        defmt::info!("Line: {}", n);
    }

    fn comment(&mut self, value: &str, _span: ::gcode::core::Span) {
        defmt::info!("Comment: {}", value)
    }

    fn program_number(&mut self, number: u32, _span: ::gcode::core::Span) {
        defmt::info!("Program number: {}", number);
    }

    fn program_delimiter(&mut self, _span: ::gcode::core::Span) {
        defmt::info!("Program delimiter");
    }

    fn word_address(
        &mut self,
        letter: char,
        value: ::gcode::core::Value<'_>,
        _span: ::gcode::core::Span,
    ) {
        defmt::info!("Word address: {}, {}", letter, defmt::Debug2Format(&value));
    }

    fn start_general_code(&mut self, number: Number) -> ControlFlow<impl CommandVisitor + '_> {
        let is_move = match number.major() {
            // G0/G1: explicit motion command; Z is target/offset.
            0 | 1 => true,
            // G91: incremental mode. The axis words that follow (e.g. Z1 F100 in
            // `G91Z1F100`) are parsed as arguments to this command, so treat
            // this command itself as the move.
            91 => {
                *self.incremental = true;
                true
            }
            // G90: absolute mode; no motion implied.
            90 => {
                *self.incremental = false;
                false
            }
            // G21: metric - already assumed, no action needed.
            21 => false,
            n => {
                defmt::info!("Unrecognised G{}", defmt::Debug2Format(&n));
                false
            }
        };

        ControlFlow::Continue(GcodeCommand {
            diagnostics: self.diagnostics,
            pending_move: self.pending_move,
            is_move,
        })
    }

    fn start_miscellaneous_code(
        &mut self,
        _number: Number,
    ) -> ControlFlow<impl CommandVisitor + '_> {
        defmt::error!("Miscellaneous operations are not implemented");
        ControlFlow::<GcodeCommand>::Break(())
    }

    fn start_tool_change_code(&mut self, _number: Number) -> ControlFlow<impl CommandVisitor + '_> {
        defmt::error!("Tool changes are not implemented");
        ControlFlow::<GcodeCommand>::Break(())
    }

    fn end_line(self, _span: ::gcode::core::Span) {
        defmt::info!("End of line");
    }
}

struct GcodeCommand<'a> {
    diagnostics: &'a mut DefmtDiagnostics,
    pending_move: &'a mut PendingMove,
    /// True when this command carries axis words that describe a Z motion.
    is_move: bool,
}

impl HasDiagnostics for GcodeCommand<'_> {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        self.diagnostics
    }
}

impl CommandVisitor for GcodeCommand<'_> {
    fn argument(&mut self, letter: char, value: Value<'_>, _span: ::gcode::core::Span) {
        defmt::info!("{}{}", letter, defmt::Debug2Format(&value));
        if self.is_move {
            match letter {
                'X' | 'x' => {
                    if let Value::Literal(x_deg) = value {
                        self.pending_move.pending_move_shoulder = Some(x_deg);
                    }
                }
                'Y' | 'y' => {
                    if let Value::Literal(y_deg) = value {
                        self.pending_move.pending_move_elbow = Some(y_deg);
                    }
                }
                'Z' | 'z' => {
                    if let Value::Literal(z_mm) = value {
                        self.pending_move.pending_move_zed = Some(z_mm);
                    }
                }
                _ => {}
            }
        }
    }

    fn end_command(self, _span: ::gcode::core::Span) {}
}
