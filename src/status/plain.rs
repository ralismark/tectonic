use super::{ChatterLevel, MessageKind, StatusBackend};
use crate::errors::Error;
use std::io::{self, Write};

pub struct PlainStatusBackend {
    chatter: ChatterLevel,
}

impl PlainStatusBackend {
    pub fn new(chatter: ChatterLevel) -> Self {
        PlainStatusBackend { chatter }
    }
}

impl StatusBackend for PlainStatusBackend {
    fn ereport(&mut self, kind: MessageKind, err: &Error) {
        if kind == MessageKind::Note && self.chatter <= ChatterLevel::Minimal {
            return;
        }

        let mut prefix = match kind {
            MessageKind::Note => "note:",
            MessageKind::Warning => "warning:",
            MessageKind::Error => "error:",
        };

        // To match the behaviour of PlainStatusBackend.report()
        let mut use_stdout = kind == MessageKind::Note;

        for item in err.iter() {
            if use_stdout {
                println!("{} {}", prefix, item);
            } else {
                eprintln!("{} {}", prefix, item);
            }
            prefix = "caused by: ";
            use_stdout = false;
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!("debugging: backtrace follows:\n{:?}", backtrace);
        }
    }

    fn report_error(&mut self, err: &Error) {
        let mut prefix = "error";

        for item in err.iter() {
            eprintln!("{}: {}", prefix, item);
            prefix = "caused by";
        }

        if let Some(backtrace) = err.backtrace() {
            eprintln!("debugging: backtrace follows:");
            eprintln!("{:?}", backtrace);
        }
    }

    fn note_highlighted(&mut self, before: &str, highlighted: &str, after: &str) {
        if self.chatter > ChatterLevel::Minimal {
            println!("note: {}{}{}", before, highlighted, after);
        }
    }

    fn dump_error_logs(&mut self, output: &[u8]) {
        eprintln!(
            "==============================================================================="
        );

        io::stderr()
            .write_all(output)
            .expect("write to stderr failed");

        eprintln!(
            "==============================================================================="
        );
    }
}
