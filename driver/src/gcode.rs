use crate::app::gcode;
use ::gcode::core::BlockVisitor;
use ::gcode::core::CommandVisitor;
use ::gcode::core::ControlFlow;
use ::gcode::core::Diagnostics;
use ::gcode::core::HasDiagnostics;
use ::gcode::core::ProgramVisitor;
use embassy_usb::driver::EndpointError;
use line_reader::LineReader;

pub async fn gcode(cx: gcode::Context<'_>) {
    let usb = cx.local.usb_class;
    let mut lines = LineReader::<128>::new();
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

        match lines.feed(frame) {
            Ok(Some(line)) => {
                defmt::info!("got line: {:a}", line);
                let src = core::str::from_utf8(line).unwrap();
                ::gcode::core::parse(
                    src,
                    &mut Printer {
                        diagnostics: NoopDiagnostics,
                    },
                );
            }
            Ok(None) => {}
            Err(err) => defmt::error!("{}", err),
        }
    }
}

struct NoopDiagnostics;

impl Diagnostics for NoopDiagnostics {}

struct Printer {
    diagnostics: NoopDiagnostics,
}

impl HasDiagnostics for Printer {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        &mut self.diagnostics
    }
}

impl ProgramVisitor for Printer {
    fn start_block(&mut self) -> ::gcode::core::ControlFlow<impl ::gcode::core::BlockVisitor + '_> {
        ControlFlow::Continue(PrintBlock {
            diagnostics: &mut self.diagnostics,
        })
    }
}

struct PrintBlock<'a> {
    diagnostics: &'a mut NoopDiagnostics,
}

impl HasDiagnostics for PrintBlock<'_> {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        self.diagnostics
    }
}

impl BlockVisitor for PrintBlock<'_> {
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

    fn start_general_code(
        &mut self,
        number: ::gcode::core::Number,
    ) -> ControlFlow<impl ::gcode::core::CommandVisitor + '_> {
        defmt::info!("G{}", defmt::Debug2Format(&number));
        ControlFlow::Continue(PrintCommand {
            diagnostics: self.diagnostics,
        })
    }

    fn start_miscellaneous_code(
        &mut self,
        number: ::gcode::core::Number,
    ) -> ControlFlow<impl CommandVisitor + '_> {
        defmt::info!("M{}", defmt::Debug2Format(&number));
        ControlFlow::Continue(PrintCommand {
            diagnostics: self.diagnostics,
        })
    }

    fn start_tool_change_code(
        &mut self,
        number: ::gcode::core::Number,
    ) -> ControlFlow<impl CommandVisitor + '_> {
        defmt::info!("T{}", defmt::Debug2Format(&number));
        ControlFlow::Continue(PrintCommand {
            diagnostics: self.diagnostics,
        })
    }

    fn end_line(self, _span: ::gcode::core::Span) {
        defmt::info!("End of line");
    }
}

struct PrintCommand<'a> {
    diagnostics: &'a mut NoopDiagnostics,
}

impl HasDiagnostics for PrintCommand<'_> {
    fn diagnostics(&mut self) -> &mut dyn Diagnostics {
        self.diagnostics
    }
}

impl CommandVisitor for PrintCommand<'_> {
    fn argument(
        &mut self,
        letter: char,
        value: ::gcode::core::Value<'_>,
        _span: ::gcode::core::Span,
    ) {
        defmt::info!("{}{}", letter, defmt::Debug2Format(&value));
    }

    fn end_command(self, _span: ::gcode::core::Span) {}
}
