mod cost_model;
mod debugger;

use bytes::Bytes;
use ckb_vm::machine::asm::{AsmCoreMachine, AsmMachine};
use ckb_vm::machine::SupportMachine;
use ckb_vm::DefaultMachineBuilder;
use std::fs::File;
use std::io::Read;
use std::process::exit;

fn main() {
    use clap::{App, Arg};
    let matches = App::new("ckb-vm-cli")
        .version("0.1")
        .about("A command line tool for CKB VM")
        .arg(
            Arg::with_name("bin")
                .long("bin")
                .short("b")
                .value_name("filename")
                .help("Specify the name of the executable")
                .required(true),
        )
        .arg(Arg::with_name("args").multiple(true))
        .get_matches();

    let args: Vec<String> = matches
        .values_of("args")
        .unwrap_or_default()
        .into_iter()
        .map(|s| s.clone().into())
        .collect();

    let bin_path = matches.value_of("bin").unwrap();
    let mut file = File::open(bin_path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let buffer: Bytes = buffer.into();
    let args: Vec<Bytes> = args.into_iter().map(|s| s.into()).collect();

    let asm_core = AsmCoreMachine::new_with_max_cycles(1 << 31);
    let core = DefaultMachineBuilder::<Box<AsmCoreMachine>>::new(asm_core)
        .instruction_cycle_func(Box::new(cost_model::instruction_cycles))
        .syscall(Box::new(debugger::Debugger::new()))
        .build();
    let mut machine = AsmMachine::new(core, None);

    machine.load_program(&buffer, &args).unwrap();
    let result = machine.run();
    let cycles = machine.machine.cycles();
    println!("Cycles = {:?}", cycles);
    if result != Ok(0) {
        println!("Error result: {:?}", result);
        exit(i32::from(result.unwrap_or(-99)));
    }
}
