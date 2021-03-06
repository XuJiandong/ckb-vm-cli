use std::fs::File;
use std::io::Read;
use std::process::exit;

use bytes::Bytes;
use ckb_vm::{DefaultMachineBuilder, ISA_B, ISA_IMC, ISA_MOP, DefaultCoreMachine, WXorXMemory, SparseMemory, TraceMachine};
use ckb_vm::machine::asm::{AsmCoreMachine, AsmMachine};
use ckb_vm::machine::SupportMachine;
use ckb_vm::machine::VERSION1;

mod cost_model;
mod debugger;

fn main() {
    use clap::{App, Arg};
    let matches = App::new("ckb-vm-b-cli")
        .version("0.3.1")
        .about("A command line tool for CKB VM, supporting B extension")
        .arg(
            Arg::with_name("bin")
                .long("bin")
                .short("b")
                .value_name("filename")
                .help("Specify the name of the executable")
                .required(true),
        )
        .arg(
            Arg::with_name("nomop")
                .long("nomop")
                .help("Disable mop")
                .takes_value(false)
                .required(false),
        )
        .arg(
            Arg::with_name("noasm")
                .long("noasm")
                .help("Disable ASM(x86)")
                .takes_value(false)
                .required(false),
        )
        .arg(Arg::with_name("args").multiple(true))
        .get_matches();

    let args: Vec<String> = matches
        .values_of("args")
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.clone().into())
        .collect();

    let disable_mop = matches.is_present("nomop");
    let disable_asm = matches.is_present("noasm");

    let bin_path = matches.value_of("bin").unwrap();
    let mut file = File::open(bin_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let buffer: Bytes = buffer.into();

    let mut args2: Vec<Bytes> = vec![Bytes::copy_from_slice(bin_path.as_ref())];
    let args3: Vec<Bytes> = args.into_iter().map(|s| s.into()).collect();
    args2.extend(args3);

    let mut flags = ISA_IMC | ISA_B;
    if !disable_mop {
        flags = flags | ISA_MOP;
    } else {
        println!("Warning: MOP is disabled.");
    }

    let cycles: u64;
    let result: Result<i8, ckb_vm::Error>;
    if disable_asm {
        println!("Warning: Run without ASM");
        let core_machine = DefaultCoreMachine::<u64, WXorXMemory<SparseMemory<u64>>>::new(
            flags,
            VERSION1,
            u64::max_value(),
        );
        let core = DefaultMachineBuilder::new(core_machine)
            .instruction_cycle_func(Box::new(cost_model::instruction_cycles))
            .syscall(Box::new(debugger::Debugger::new()))
            .build();
        let mut machine = TraceMachine::new(core);
        machine.load_program(&buffer, &args2).unwrap();
        result = machine.run();

        cycles = machine.machine.cycles();
    } else {
        let asm_core = AsmCoreMachine::new(flags, VERSION1, u64::max_value());

        let core = DefaultMachineBuilder::<Box<AsmCoreMachine>>::new(asm_core)
            .instruction_cycle_func(Box::new(cost_model::instruction_cycles))
            .syscall(Box::new(debugger::Debugger::new()))
            .build();
        let mut machine = AsmMachine::new(core, None);

        machine.load_program(&buffer, &args2).unwrap();
        result = machine.run();

        cycles = machine.machine.cycles();
    }

    let c: f64 = cycles as f64;
    if cycles > 1000000 {
        println!("Cycles = {:.1} M cycles", c / 1024. / 1024.);
    } else {
        println!("Cycles = {:.1} K cycles (It's below 1 M)", c / 1024.);
    }
    if result != Ok(0) {
        println!("Error result: {:?}", result);
        exit(i32::from(result.unwrap_or(-99)));
    }
}
